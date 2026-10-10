---
id: TASK-2446
title: 'API-13: ThemeConfig, PlanHeaderStyle and ErrorBlockChars reachable by two public paths in ops-theme'
status: Done
assignee: []
created_date: '2026-10-10 15:24'
updated_date: '2026-10-10 21:21'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2609'
modified_files:
  - crates/theme/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'API-13:crates/theme/src/lib.rs:crate-root'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/lib.rs:31-32`

**What**: lib.rs re-exports both the module and its items:
`pub use ops_core::config::theme_types;` followed by
`pub use ops_core::config::theme_types::{ErrorBlockChars, PlanHeaderStyle, ThemeConfig};`
Each of the three types is therefore reachable by two public paths in this crate — `ops_theme::ThemeConfig` and `ops_theme::theme_types::ThemeConfig` — doubling every mention in docs, search results and error messages, and leaving readers unsure whether the two are the same type.

**Why it matters**: API-13 — a public item should be reachable by exactly one path. Re-exporting from a private module into the root is the correct use of `pub use`; re-exporting items whose parent module re-export is also public is the compatibility-artifact shape the rule flags. (Leaking the ops-core sibling types themselves is fine per API-13's umbrella-crate exception; only the dual path is the finding.)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each of ThemeConfig, PlanHeaderStyle, ErrorBlockChars is reachable by exactly one public path in ops-theme (drop either the module re-export or the item re-exports)
- [x] #2 The crate docs in lib.rs that reference the re-export arrangement are updated to match

<!-- AC:END -->
