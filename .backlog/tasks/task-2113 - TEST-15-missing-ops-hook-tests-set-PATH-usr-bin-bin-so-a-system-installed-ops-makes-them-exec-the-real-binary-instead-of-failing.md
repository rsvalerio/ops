---
id: TASK-2113
title: 'TEST-15: missing-ops hook tests set PATH=/usr/bin:/bin, so a system-installed ops makes them exec the real binary instead of failing'
status: To Do
assignee: []
created_date: '2026-09-08 06:53'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - test-quality
dependencies: []
parent_task_id: 'TASK-2241'
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: medium
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:208` (`hook_script_reports_a_missing_ops_binary_by_name`), `extensions/run-before-commit/src/lib.rs:240` (`hook_script_honours_the_bypass_when_ops_is_missing`)

**What**: Both tests simulate "ops is not on PATH" by handing the child `.env("PATH", "/usr/bin:/bin")`, with a comment claiming this "deliberately excludes the ambient one so a developer's own installed `ops` cannot satisfy the probe". That only holds while nobody installs `ops` system-wide. `/usr/bin` and `/bin` are exactly where a distro package, a `cargo install --root /usr/local` (whose bin dir is on many PATHs), or a CI image that copies the built binary into `/usr/bin` would put it. When `ops` is present there:

- `command -v ops` succeeds, so `hook_script_reports_a_missing_ops_binary_by_name` never reaches the diagnostic and fails on `assert_eq!(out.status.code(), Some(1))` — or passes for the wrong reason if ops happens to exit 1.
- worse, the script falls through to `exec ops run-before-commit --changed-only`, so the test **runs the developer's real ops** — a real config load and a real configured command chain — with the cwd set to a bare tempdir.
- the `"maybe"` case at the end of `hook_script_honours_the_bypass_when_ops_is_missing` has the same fall-through.

The script needs no external command at all: `case`, `command -v` and `echo` are all `sh` builtins, and every path either exits or `exec`s. An empty `PATH` reproduces "ops is missing" unconditionally.

**Why it matters**: The test is environment-dependent in a way that inverts its own purpose — it is green on the machines where the guard is untested and can spawn an unbounded, side-effecting subprocess on the machines where it breaks. That is the flakiness shape the crate's own TEST-15 notes elsewhere were written to avoid.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both tests make the ops lookup fail unconditionally (empty PATH, or a PATH pointing only at an empty temp dir) rather than relying on /usr/bin and /bin not containing ops
- [ ] #2 Neither test can reach  on any machine, so no real ops process is spawned by the suite
- [ ] #3 The stale comment claiming /usr/bin:/bin excludes a developer's installed ops is removed or corrected
- [ ] #4 Both tests still assert exit code 1 and the ops / reinstall-command / bypass-variable substrings on stderr
<!-- AC:END -->
