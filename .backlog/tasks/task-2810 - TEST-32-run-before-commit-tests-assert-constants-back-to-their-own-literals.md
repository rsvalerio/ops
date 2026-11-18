---
id: TASK-2810
title: 'TEST-32: run-before-commit tests assert constants back to their own literals'
status: To Do
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-32:extensions/run-before-commit/src/lib.rs:tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:200` (`skip_env_var_const_matches_the_literal_spelled_into_the_script`), `:362` (`hook_config_pins_every_macro_argument`), `:388-392` (`hook_config_legacy_markers_only_match_commit_hooks`)

**What**: `assert_eq!(SKIP_ENV_VAR, "SKIP_OPS_RUN_BEFORE_COMMIT")` appears twice, and `assert_eq!(HOOK_CONFIG.legacy_markers, &["ops run-before-commit", "ops before-commit", "ops pre-commit"])` restates the list literally written at `:90-94`. Each passes by construction and only forces a second edit on any change.

**Why it matters**: The property that actually matters at `:200` is that the script and the const agree; it can be asserted directly, e.g. `HOOK_SCRIPT.contains(&format!("case \"${{{SKIP_ENV_VAR}:-}}\""))`, which fails on real drift without hard-coding the value again. The legacy-marker list is already covered by properties (every marker mentions commit and not push; `install_hook_updates_every_legacy_marker` upgrades each one), so the literal copy adds review churn but no detection power.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The SKIP_ENV_VAR/script agreement is asserted by deriving the expected script text from the const, with no repeated literal
- [ ] #2 The tautological assert_eq of legacy_markers against a copied literal is removed or replaced by a property assertion
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.28 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
