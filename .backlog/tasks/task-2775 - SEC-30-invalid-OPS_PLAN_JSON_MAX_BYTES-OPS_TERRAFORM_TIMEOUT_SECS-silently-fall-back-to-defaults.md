---
id: TASK-2775
title: 'SEC-30: invalid OPS_PLAN_JSON_MAX_BYTES / OPS_TERRAFORM_TIMEOUT_SECS silently fall back to defaults'
status: Done
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 22:57'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2856'
modified_files:
  - extensions-terraform/plan/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-30:extensions-terraform/plan/src/lib.rs:PipelineEnv::from_env'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/lib.rs:409` (`plan_json_max_bytes`), `lib.rs:369` (`terraform_timeout`), resolved in `PipelineEnv::from_env` (`lib.rs:441`)

**What**: Both overrides go through `std::env::var(..).ok().and_then(|s| s.parse::<u64>().ok()).filter(|&n| n > 0)`. A value that is set but invalid, such as `512MB`, `1e9`, ` 600`, `0` or a non-UTF-8 value, is dropped without a word and the default applies. The cap error then tells the operator to "override via OPS_PLAN_JSON_MAX_BYTES", the variable they have already set, with no hint that the value was rejected.

**Why it matters**: SEC-30 asks that configuration be validated at startup and fail fast. The fallback is the safe value, so this is not a bypass. But an operator's explicit setting is discarded silently, and the diagnostic then points at the very setting that was ignored. A set-but-unparseable override should be either a hard error naming the variable and the value, or at least a `tracing::warn!`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A set-but-invalid OPS_PLAN_JSON_MAX_BYTES or OPS_TERRAFORM_TIMEOUT_SECS is reported (error or warn) naming the variable and the rejected value
- [x] #2 Unset variables still use the documented defaults
- [x] #3 The parsing is unit-tested on injected strings without mutating the process environment

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.21 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
