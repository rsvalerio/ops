---
id: TASK-1228
title: >-
  ARCH-9: manifest_max_bytes and ops_toml_max_bytes use divergent caching
  policies for the same env-knob pattern
status: Done
assignee:
  - TASK-1262
created_date: '2026-05-08 12:58'
updated_date: '2026-05-08 15:35'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:26-72` and `crates/core/src/config/loader.rs:42-48`

**What**: `manifest_max_bytes` (text.rs) memoises the env value behind `OnceLock<u64>` with one-shot warn on parse failure. `ops_toml_max_bytes` (loader.rs) re-reads the env on every call and silently returns the default. Two implementations of the same byte-cap-from-env pattern with opposite caching and opposite diagnostic policies.

**Why it matters**: TASK-1129 covers the per-call re-read on `ops_toml_max_bytes`; this is the broader concern of two divergent implementations of the same pattern in adjacent modules. Drift between them is invisible in code review.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a shared cached_byte_cap_env(env_var, default) helper
- [ ] #2 Both call sites delegate to it
- [ ] #3 Single test exercises parse-failure / zero / unset for both env vars
<!-- AC:END -->
