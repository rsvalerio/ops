---
id: TASK-0195
title: >-
  SEC-11: OPS__ env overlay has no schema validation; a malformed
  OPS__COMMANDS__FOO can panic deserialization
status: To Do
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - SEC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/config/loader.rs:18-33

**What**: merge_env_vars builds a config crate Environment source with prefix OPS and separator __, then try_deserialize::<ConfigOverlay>. Any user whose shell exports OPS__COMMANDS__MYCMD__PROGRAM="cargo" (happens anytime someone accidentally double-underscores a non-ops variable) drives a deserialization that may or may not fit ConfigOverlay. Failures are logged as tracing::warn and silently discarded (Ok(Err(e)) arm). Unknown keys under serde(deny_unknown_fields) on ConfigOverlay cause the entire env overlay to be dropped — a single typo in one OPS__ var disables every other OPS__ var in the same process. This is a correctness cliff without a loud failure path.

**Why it matters**: SEC-11 / ERR-1. Silently swallowing env-config errors means an operator who sets OPS__OUTPUT__THEME=compact in CI and mis-types it as OPS__OUTOUT__THEME=compact sees no diff — their intended config quietly does not apply. Fail-loud on env-config parse errors (return anyhow::Error from load_config), or at minimum elevate to tracing::error! with the offending env var names so it is visible in CI logs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Elevate env-config parse/deserialize failures from warn to error with the offending key(s)
- [ ] #2 Consider failing load_config on deny_unknown_fields errors from env; document env-var naming rules
<!-- AC:END -->
