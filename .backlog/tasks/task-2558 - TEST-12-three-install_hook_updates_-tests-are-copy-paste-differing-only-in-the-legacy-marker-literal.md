---
id: TASK-2558
title: 'TEST-12: three install_hook_updates_* tests are copy-paste differing only in the legacy marker literal'
status: To Do
assignee: []
created_date: '2026-10-10 15:40'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - test-quality
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-12:extensions/run-before-commit/src/lib.rs:mod tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:319-350`, `:352-383`, `:388-413`

**What**: `install_hook_updates_legacy_before_commit_hook`, `install_hook_updates_legacy_pre_commit_hook` and `install_hook_updates_run_before_commit_hook` are three ~30-line test bodies that are byte-identical apart from the legacy marker string written into the pre-existing hook ("ops before-commit" / "ops pre-commit" / "ops run-before-commit") — including two verbatim copies of the returned-path assertion comment. Each marker is a distinct scenario, so the tests are not redundant in what they verify, but the copy-paste shape is exactly TEST-12's "identical logic, same paths, trivial differences". The crate's own idiom for one logic over several inputs is a loop (`hook_script_honours_the_bypass_when_ops_is_missing`, :283-306, iterates skip values), and `hook_config_legacy_markers_only_match_commit_hooks` (:443) already pins the marker list the loop should iterate.

**Why it matters**: Three copies must be read and re-approved in every future diff, and a change to the install-hook test setup (or a fourth legacy marker) edits three-plus places in lockstep — the classic drift surface duplication leaves behind.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A single test (or shared helper) covers all three legacy markers, iterating the pinned marker list so a new marker is covered automatically
- [ ] #2 cargo test -p ops-run-before-commit passes with equivalent per-marker coverage (upgrade message, hook path, script content)
<!-- AC:END -->
