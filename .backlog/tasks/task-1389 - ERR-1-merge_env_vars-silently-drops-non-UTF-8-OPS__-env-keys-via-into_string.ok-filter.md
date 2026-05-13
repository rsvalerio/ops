---
id: TASK-1389
title: >-
  ERR-1: merge_env_vars silently drops non-UTF-8 OPS__ env keys via
  into_string().ok() filter
status: To Do
assignee:
  - TASK-1454
created_date: '2026-05-13 18:03'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:113`

**What**: `merge_env_vars` builds the diagnostic `ops_keys` list with `std::env::vars_os().filter_map(|(k, _)| k.into_string().ok())`. Any `OPS__*` env key that is not valid UTF-8 is silently dropped from the diagnostic list, while the `config` crate's `Environment::with_prefix("OPS").separator("__")` may still observe or skip it independently. Misconfigured non-UTF-8 OPS keys therefore produce no warning, no error, and no diagnostic — they vanish.

**Why it matters**: The surrounding code is intentionally loud about misconfiguration (TASK-1181 alias hygiene, TASK-0943 byte cap, TASK-1086 umask). A silent drop here breaks the "fail loud" posture and makes a class of operator mistakes invisible — the typical first symptom is "OPS__ override didn't apply" with no log breadcrumb.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the filter_map silent-drop at loader.rs:113 with an iteration that counts non-UTF-8 OPS__* keys separately
- [ ] #2 Emit a single tracing::warn! with count when any non-UTF-8 OPS__ keys are observed, so operators see exactly one breadcrumb rather than nothing
- [ ] #3 Add a test (gated #[cfg(unix)] where OsString from raw bytes is straightforward) asserting the warn fires when a non-UTF-8 OPS__* key is present, and does not fire otherwise
<!-- AC:END -->
