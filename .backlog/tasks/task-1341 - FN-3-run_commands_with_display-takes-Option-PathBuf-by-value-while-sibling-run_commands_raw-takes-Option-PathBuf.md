---
id: TASK-1341
title: >-
  FN-3: run_commands_with_display takes Option<PathBuf> by value while sibling
  run_commands_raw takes Option<&PathBuf>
status: To Do
assignee:
  - TASK-1385
created_date: '2026-05-12 16:41'
updated_date: '2026-05-12 22:17'
labels:
  - code-review-rust
  - functions
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:191,255-260`

**What**: Two sibling helpers expose inconsistent ownership for the `tap` parameter — `run_commands_raw` at line 191 takes `tap: Option<&PathBuf>` while `run_commands_with_display` at line 258 takes `tap: Option<PathBuf>` by value. Both are called from the same plan-running entry point.

**Why it matters**: Mixed ownership for the same conceptual parameter forces the caller to remember which form each helper wants (the call sites at lines 173 and elsewhere disambiguate via `.as_ref()` for one and pass-by-value for the other). A swap or refactor will compile until it hits a borrow-checker / lifetime mismatch; picking one shape (`Option<&Path>` is the conventional one) eliminates the asymmetry.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both helpers take the same ownership form for tap (prefer Option<&Path>)
- [ ] #2 Call sites updated; cargo clippy --all-targets --workspace -- -D warnings clean
<!-- AC:END -->
