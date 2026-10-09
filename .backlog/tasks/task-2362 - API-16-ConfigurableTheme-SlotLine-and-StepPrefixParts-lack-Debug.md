---
id: TASK-2362
title: 'API-16: ConfigurableTheme, SlotLine and StepPrefixParts lack Debug'
status: Done
assignee: []
created_date: '2026-10-04 14:10'
updated_date: '2026-10-04 15:52'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2423'
modified_files:
  - crates/theme/src/configurable.rs
  - crates/theme/src/step_line_theme.rs
priority: low
ordinal: 1000
dedup_key: 'API-16:crates/theme/src/configurable.rs:ConfigurableTheme'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:43`, `crates/theme/src/step_line_theme.rs:94,115`

**What**: Public types `ConfigurableTheme`, `SlotLine<'a>` and `StepPrefixParts<'a>` implement neither `Debug` nor `Clone`. Every field already supports `Debug`: `ThemeConfig` derives `Debug, Clone`, and the rest are `Option<String>`, `String`, `usize` and `&str`. `ThemeError` and `BoxSnapshot` do derive `Debug`.

**Why it matters**: C-COMMON-TRAITS: downstream crates cannot add the impls, and `{:?}` or `assert_eq!` failure output and `Result<ConfigurableTheme, _>::unwrap_err()` are unusable on the crate's main type. `ConfigurableTheme` needs `Debug` for `unwrap_err`. A workspace `missing_debug_implementations` lint would also catch this.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ConfigurableTheme, SlotLine and StepPrefixParts derive Debug (and Clone where cheap and sensible)
- [x] #2 cargo clippy --workspace passes

<!-- AC:END -->
