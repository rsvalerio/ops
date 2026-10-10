---
id: TASK-2436
title: 'READ-13: remaining ops-theme and ops-extension files narrate change history and carry rule/TASK tags'
status: To Do
assignee: []
created_date: '2026-10-04 15:56'
updated_date: '2026-10-10 14:31'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2439'
modified_files:
  - crates/theme/src/style/strip.rs
  - crates/theme/src/configurable/report.rs
  - crates/theme/src/resolve.rs
  - crates/theme/src/lib.rs
  - crates/theme/src/configurable.rs
  - crates/theme/src/configurable/boxed.rs
  - crates/extension/tests/public_api.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/style/strip.rs:4-16,42,65,133,309`, `crates/theme/src/configurable/report.rs:3`, `crates/theme/src/resolve.rs:24-54`, `crates/theme/src/lib.rs:8-10`, `crates/theme/src/configurable.rs` and `crates/theme/src/configurable/boxed.rs` (rule/TASK tags on inline comments), `crates/theme/src/tests/`, `crates/extension/tests/public_api.rs` (about 40 rule/TASK tags)

**What**: TASK-2352 and TASK-2361 rewrote the docs in the files they named to describe the end state. The same shape remains outside those file lists: `strip.rs` says the private copy `ops-cargo-update` "used to carry" was removed and that tab "used to be" handled differently, `report.rs` and `strip.rs` open with "split out of ...", and rule-ID / TASK-NNNN provenance tags are threaded through doc and inline comments in the theme crate and through the ops-extension integration tests.

**Why it matters**: migration notes and provenance tags mean nothing to an API reader and go stale while looking authoritative; history belongs in commit messages and backlog tasks.

**Origin**: discovered during TASK-2423 while fixing TASK-2361 and TASK-2352.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No 'used to', 'previously', 'split out of' or other migration narration remains in the listed files
- [ ] #2 Rule-ID / TASK-NNNN tags are removed from doc and inline comments unless the comment explains a non-obvious current invariant
<!-- AC:END -->
