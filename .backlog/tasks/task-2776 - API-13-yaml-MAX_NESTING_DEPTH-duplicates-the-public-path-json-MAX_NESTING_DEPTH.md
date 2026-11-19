---
id: TASK-2776
title: 'API-13: yaml::MAX_NESTING_DEPTH duplicates the public path json::MAX_NESTING_DEPTH'
status: Done
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 21:37'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2859'
modified_files:
  - extensions/config-checkers/src/yaml.rs
  - extensions/config-checkers/src/json.rs
  - extensions/config-checkers/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'API-13:extensions/config-checkers/src/yaml.rs:MAX_NESTING_DEPTH'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/yaml.rs:36`

**What**: `pub use crate::json::MAX_NESTING_DEPTH;` inside the public `yaml` module gives one constant two public paths (`ops_config_checkers::json::MAX_NESTING_DEPTH` and `ops_config_checkers::yaml::MAX_NESTING_DEPTH`). The doc comment even has to explain that they are the same item. The constant is also semantically shared by both checkers yet owned by the JSON module.

Related candidates, lower confidence (deliberate per lib.rs docs, workspace technical split): `lib.rs:74` `pub use ops_core::bounded_read::DEFAULT_MAX_BYTES` and `report.rs:10` `pub use ops_core::bounded_read::{FailedFile, FailureKind}` re-export a sibling crate's items under a second public path.

**Why it matters**: Two paths for one item double every mention in docs and search results and leave readers unsure whether the YAML limit can differ from the JSON one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 MAX_NESTING_DEPTH has exactly one public path, defined in a place that is not JSON-specific (e.g. the crate root or a shared limits module), and both checkers refer to it
- [x] #2 Intra-doc links in json.rs, yaml.rs and lib.rs point at the single path and cargo doc stays warning-free

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.23 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
