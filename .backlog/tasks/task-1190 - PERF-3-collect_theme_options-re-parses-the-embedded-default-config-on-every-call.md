---
id: TASK-1190
title: >-
  PERF-3: collect_theme_options re-parses the embedded default config on every
  call
status: To Do
assignee:
  - TASK-1262
created_date: '2026-05-08 08:12'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/theme_cmd.rs:20`

**What**: `parse_default_config` deserialises the embedded default_ops_toml on every call to collect_theme_options, which runs from both run_theme_list and run_theme_select. Result is then only used to compute a `themes.contains_key(name)` Boolean — pure HashSet membership — but the entire Config is reconstructed.

**Why it matters**: Hot enough on the help / theme-list path that the allocation is wasted. A OnceLock<HashSet<String>> of just the built-in theme names reduces this to a hash lookup, aligning with the OnceLock discipline used in expand.rs::TMPDIR_DISPLAY and text.rs::MANIFEST_MAX_BYTES.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Built-in theme names are computed once and cached behind a OnceLock<HashSet<&'static str>>, shared by both list and select paths.
- [ ] #2 A test asserts the parser is invoked at most once across N calls.
<!-- AC:END -->
