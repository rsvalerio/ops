---
id: TASK-2449
title: 'API-14: All 16 public config passthrough accessors in config_access.rs lack doc summaries'
status: To Do
assignee: []
created_date: '2026-10-10 15:24'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2609'
modified_files:
  - crates/theme/src/configurable/config_access.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:crates/theme/src/configurable/config_access.rs:impl ConfigurableTheme'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable/config_access.rs:15,20,25,30,35,40,45,50,55,60,65,70,75,80,85,90`

**What**: Every public accessor in this impl block — `left_pad`, `left_pad_str`, `status_icon`, `separator_char`, `step_indent`, `summary_prefix`, `running_template`, `tick_chars`, `running_template_overhead`, `header_color`, `label_color`, `separator_color`, `duration_color`, `summary_color`, `plan_header_prefix`, `format_elapsed` — has no `///` doc comment. The module-level `//!` docs describe the file's role but API-14 attaches the mandatory summary to each public item.

<!-- scan confidence: candidates to inspect — trivial one-line forwarders; the reviewer may legitimately waive self-evident getters and close this as wontfix. Filed for consistency with the crate's otherwise fully documented surface. -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each public accessor carries a one-line /// summary, or the finding is explicitly waived as wontfix with a note in the task
<!-- AC:END -->
