---
id: TASK-0749
title: 'TRAIT-4: ToolSource derives Clone but not Copy despite being unit-variant enum'
status: Done
assignee:
  - TASK-0829
created_date: '2026-05-01 05:52'
updated_date: '2026-05-02 08:48'
labels:
  - code-review-rust
  - traits
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/tools.rs:54-60, 37`

**What**: ToolSource has two unit variants (Cargo, System) and derives Clone only. ToolSpec::source(&self) calls ext.source.clone() to return owned. Clone is unnecessary for a 1-byte enum.

**Why it matters**: TRAIT-4: derive deliberately. Unit-only enums are the canonical Copy case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ToolSource adds Copy to its derive list
- [x] #2 ToolSpec::source returns ToolSource directly without .clone()
- [x] #3 No call site needs a compensating clone
<!-- AC:END -->
