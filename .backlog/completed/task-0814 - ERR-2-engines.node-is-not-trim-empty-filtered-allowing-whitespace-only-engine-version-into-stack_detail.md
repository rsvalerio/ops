---
id: TASK-0814
title: >-
  ERR-2: engines.node is not trim/empty-filtered, allowing whitespace-only
  engine version into stack_detail
status: Done
assignee:
  - TASK-0823
created_date: '2026-05-01 06:03'
updated_date: '2026-05-01 09:21'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:123`

**What**: engines_node: raw.engines.and_then(|e| e.node) returns the raw value. A whitespace-only engines.node produces Some(spaces), which build_stack_detail formats as Node spaces dot pnpm.

**Why it matters**: Same trim/non-empty contract pattern (TASK-0563, TASK-0704); only this field still slips through.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pass through trim_nonempty
- [ ] #2 Test pins that whitespace-only engine yields no Node prefix in stack_detail
<!-- AC:END -->
