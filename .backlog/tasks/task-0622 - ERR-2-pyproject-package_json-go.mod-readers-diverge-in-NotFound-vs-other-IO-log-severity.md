---
id: TASK-0622
title: >-
  ERR-2: pyproject/package_json/go.mod readers diverge in NotFound vs other-IO
  log severity
status: Done
assignee:
  - TASK-0639
created_date: '2026-04-29 05:21'
updated_date: '2026-04-29 11:02'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/units.rs:54`

**What**: All four about crates have parse-or-default helpers that conditionally log via tracing::debug only on non-NotFound IO errors and tracing::warn on parse failures. Sites: extensions-python/about/src/lib.rs:165-180, units.rs:53-68, extensions-node/about/src/package_json.rs:67-82, units.rs:80-104, extensions-go/about/src/go_mod.rs:18-27, go_work.rs:11-19. Maven (TASK-0561) logs every read error at debug regardless of kind. Six near-identical match arms encode an implicit policy.

**Why it matters**: Without a single helper, future copy/paste will silently change severity (TASK-0467 already filed one such drift in duckdb providers). <!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Shared read_optional_text helper added in ops_about (or shared crate) encapsulating NotFound→silent, other→tracing::debug rule
- [x] #2 All six listed sites consume it
- [x] #3 Unit test covers NotFound and other-IO branches at helper layer
<!-- AC:END -->
