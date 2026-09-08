---
id: TASK-2237
title: code-review-plan-wave3
status: Done
assignee:
  - code-review-wave
created_date: '2026-09-08 10:50'
updated_date: '2026-09-08 17:05'
labels:
  - code-review-wave
dependencies:
  - TASK-2158
  - TASK-2165
  - TASK-2162
  - TASK-2132
  - TASK-2126
  - TASK-2166
modified_files:
  - extensions/config-checkers/src/json.rs
  - extensions/config-checkers/src/lib.rs
  - extensions/config-checkers/src/report.rs
  - extensions/config-checkers/src/runner.rs
  - extensions/config-checkers/src/tests.rs
  - extensions/config-checkers/src/yaml.rs
  - extensions/text-fixers/src/atomic.rs
  - extensions/text-fixers/src/discovery.rs
  - extensions/text-fixers/src/discovery/tests.rs
  - extensions/text-fixers/src/eof.rs
  - extensions/text-fixers/src/options.rs
  - extensions/text-fixers/src/report.rs
  - extensions/text-fixers/src/runner.rs
  - extensions/text-fixers/src/test_support.rs
  - extensions/text-fixers/src/tests.rs
ordinal: 143000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave3: In-place rewriter behaviour and the shared read-candidate layer it duplicates
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2247 wave13 (5 files: extensions/config-checkers/src/lib.rs ...); TASK-2248 wave14 (3 files: extensions/text-fixers/src/discovery.rs ...); TASK-2235 wave1 (2 files: extensions/text-fixers/src/atomic.rs ...); TASK-2234 wave0 (1 file: extensions/config-checkers/src/lib.rs)

Branch: code-review/TASK-2237
<!-- SECTION:NOTES:END -->
