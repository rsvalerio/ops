---
id: TASK-2560
title: 'ERR-9: HasStagedFilesError interpolates the #[source] field into its Display message'
status: To Do
assignee: []
created_date: '2026-10-10 15:40'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - err
dependencies: []
parent_task_id: 'TASK-2624'
modified_files:
  - extensions/hook-common/src/git_state.rs
priority: medium
ordinal: 1000
dedup_key: 'ERR-9:extensions/hook-common/src/git_state.rs:HasStagedFilesError'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git_state.rs:19,33`

**What**: The `Spawn` and `Io` variants of `HasStagedFilesError` mark their `source: std::io::Error` field with `#[source]` *and* interpolate that same field into the `#[error(...)]` message (`"failed to run \`{program} diff --cached\`: {source}"`, `"failed to read output from \`{program} diff --cached\`: {source}"`). thiserror wires `Error::source()` from the attribute, so the io::Error is set as both the message tail and the chain source: any chain-walking printer (anyhow `{:#}`, `{:?}` with its `Caused by:` block) renders the io::Error text twice.

**Why it matters**: ERR-9 — the variant's message should say what *this* layer was doing and let the chain supply the cause. Interpolating a field is correct for data the source does not carry (`program`, `exit_code`); it is wrong for the source itself. Operators reading a hung/failed pre-commit probe get doubled diagnostics ("No such file or directory" twice), which pads the message without adding signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Neither variant's #[error] format references the field marked #[source]; messages name the operation and program only
- [ ] #2 The #[source] attribution is kept so error chains still carry the underlying io::Error
- [ ] #3 cargo test -p ops-hook-common passes
<!-- AC:END -->
