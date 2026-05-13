---
id: TASK-1409
title: 'PERF-3: Stack::default_commands re-parses embedded TOML on every call'
status: To Do
assignee:
  - TASK-1451
created_date: '2026-05-13 18:17'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack/mod.rs:104`

**What**: `Stack::default_commands` calls `toml::from_str(toml)` on every invocation. The result is a pure function of the `Stack` variant and the `&'static str` `include_str!`-embedded TOML — re-parsing yields identical output every time.

**Why it matters**: On the `ops init` path and any code that lazily fetches stack defaults, serde re-parses static content repeatedly. Memoize per-variant via `OnceLock<IndexMap<...>>` so repeated calls are O(1) after the first.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 memoize parsed IndexMap behind a per-variant OnceLock so repeated default_commands calls are O(1) after first
- [ ] #2 parse failure still degrades to empty IndexMap with one-shot tracing::warn, preserving current contract
- [ ] #3 add a regression test asserting two back-to-back default_commands() calls return equal-content IndexMaps
<!-- AC:END -->
