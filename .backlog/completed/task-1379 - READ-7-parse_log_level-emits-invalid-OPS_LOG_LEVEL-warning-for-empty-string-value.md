---
id: TASK-1379
title: >-
  READ-7: parse_log_level emits 'invalid OPS_LOG_LEVEL=' warning for
  empty-string value
status: Done
assignee:
  - TASK-1386
created_date: '2026-05-12 21:52'
updated_date: '2026-05-12 23:45'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/main.rs:107-138`

**What**: `parse_log_level` takes `raw: Option<&str>`. The `let Some(v) = raw else { ... INFO }` early-return handles only the unset case (`std::env::var(...).ok()` → `None`). When `OPS_LOG_LEVEL` is set to an empty string (e.g. shell did `OPS_LOG_LEVEL= ops ...`), `std::env::var` returns `Ok("")`, so `raw = Some("")`. That falls through to the directive parse, which fails, and the user sees:

```
ops: warning: invalid OPS_LOG_LEVEL='': empty directive; falling back to info
```

This is a spurious warning — empty == unset is the universal shell convention. Sister env-flag helper `env_flag_enabled` (subcommands.rs:107-115) already treats an empty trimmed value as "off" with explicit documentation citing this convention.

**Why it matters**: a CI matrix that does `OPS_LOG_LEVEL=${LEVEL:-}` (with `LEVEL` unset) prints a stderr warning on every invocation, polluting CI logs and making real `OPS_LOG_LEVEL` typos harder to spot. Mirrors the `env_flag_enabled` falsy-empty handling that already lives one module over.

**Fix**: in `parse_log_level`, treat `Some("")` (or `Some(v) if v.trim().is_empty()`) the same as `None` — return the INFO default with no warning.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_log_level returns INFO default with no warning for Some("") and Some("   ")
- [x] #2 Unit test in log_level_tests asserts the empty-string and whitespace-only inputs are silent
- [x] #3 Behaviour matches env_flag_enabled empty-string handling documented at subcommands.rs:107-115
<!-- AC:END -->
