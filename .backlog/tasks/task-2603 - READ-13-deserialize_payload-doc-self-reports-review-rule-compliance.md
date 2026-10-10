---
id: TASK-2603
title: 'READ-13: deserialize_payload doc self-reports review-rule compliance'
status: To Do
assignee: []
created_date: '2026-10-10 20:49'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2623'
modified_files:
  - extensions/about/src/providers.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/about/src/providers.rs:deserialize_payload'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/providers.rs:117`

**What**: The doc comment on `deserialize_payload` contains a review-rule self-report and a call-site census: "Both deserialization call sites in this crate — `load_or_default`, ... and `lib.rs::resolve_identity` for `project_identity` — must not propagate the raw `serde_json` error with a bare `?` ..." followed by "**ERR-14:** these payloads are nested ... `serde_path_to_error` reports the concrete location ... which is the difference between a fixable bug report and a bisect."

**Why it matters**: READ-13 names exactly this shape: "self-report tables of which guidelines a change followed are process artifacts". Citing ERR-14 in a production doc ties the doc to a review process, and enumerating sibling call sites goes stale the moment a third call site appears. The legitimate, enduring content — payloads are nested so errors carry the field path via serde_path_to_error — survives without the rule citation and the census.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc explains the field-path context behavior without citing review rule IDs
- [ ] #2 No enumeration of sibling call sites that must "not" do something (state what this function does instead)
<!-- AC:END -->
