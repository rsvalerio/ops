---
id: TASK-0040
title: >-
  CQ-9: from_identity_filtered exceeds FN-1 with repetitive field-building
  pattern
status: Done
assignee: []
created_date: '2026-04-14 20:14'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - FN-1
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/core/src/project_identity.rs lines 108-205: from_identity_filtered() is 98 lines. It repeats the same pattern 11 times: if show(field) { if let Some(val) = id.field { fields.push((label, formatted_value)); } }. Each instance has minor formatting variations (singular/plural, format_number, etc.) but the overall structure is identical. A data-driven approach using field definitions (field_id, extractor closure, formatter) would reduce this to a loop over a field-spec table.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 from_identity_filtered() reduced to ≤50 lines using a data-driven field-definition approach or equivalent refactoring
- [ ] #2 All existing tests continue to pass
<!-- AC:END -->
