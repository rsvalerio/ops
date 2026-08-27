---
id: TASK-1942
title: >-
  SEC-25: cleanup_artifacts deletes any pre-existing file at --json-out,
  including one it never wrote
status: Triage
assignee: []
created_date: '2026-08-27 15:48'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:360-380` (`cleanup_artifacts`), called from `:168-170`

**What**: Two problems in the same loop.

1. **Deletes a file it did not create.** `run_terraform_pipeline` only writes `json_path` when `opts.keep_plan` is set (`:346-348`). `cleanup_artifacts` only runs when `keep_plan` is *false*. So on every default run the JSON entry of the loop unlinks whatever happens to sit at the `--json-out` path even though this invocation never produced it. `ops plans --json-out ~/notes.json` silently deletes `~/notes.json`. The flag's help text - "JSON plan output path" - gives no hint that the path is also a delete target.

2. **TOCTOU.** `if path.exists() { std::fs::remove_file(&path) }` is the check-then-act pattern SEC-25 names. The existence probe adds nothing: `remove_file` already reports `NotFound`, and the branch is racy against anything touching the path between the two syscalls.

There is a third, quieter hazard: the paths are re-derived here by calling `expand_path` a second time (`:365`), independently of the expansion done in `run_terraform_pipeline` at `:273-274`. Any change to the environment between the two calls makes cleanup target a different path than the one written.

**Why it matters**: unlinking a user-named path the tool never created is destructive and surprising; combined with the racy existence check it is also unsound. SEC-25 asks for the operation to be attempted directly with errors handled, rather than gated on a separate probe.

**Suggested fix**: track the artefact paths actually created during this run (return them from `run_terraform_pipeline`) and delete only those; drop the `exists()` probe and match on `ErrorKind::NotFound` from `remove_file` instead.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cleanup deletes only artifact paths this invocation actually created, never a pre-existing file at --json-out or --out
- [ ] #2 The exists() probe is removed and remove_file's NotFound error is treated as success
- [ ] #3 Artifact paths are expanded once and reused by both the write and the cleanup, not re-derived
- [ ] #4 A test asserts that a pre-existing file at the --json-out path survives a default run that never wrote it
<!-- AC:END -->
