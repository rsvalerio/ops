---
id: TASK-0394
title: >-
  READ-8: Providers print nothing on parse failure but also have no tracing
  visibility
status: Done
assignee:
  - TASK-0417
created_date: '2026-04-26 09:40'
updated_date: '2026-04-27 19:56'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/lib.rs:98` (across all parsers)

**What**: None of the parsers (parse_go_mod, parse_pyproject, parse_package_json, parse_pom_xml, parse_gradle_settings, etc.) emit any tracing event when they encounter unexpected file content; they silently return None. The crates have no tracing dependency.

**Why it matters**: For a data provider library running as part of a CLI, tracing::debug!/warn! on swallowed parse errors is the minimum diagnostic affordance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add tracing dep and emit tracing::warn!(path = %p.display(), error = %e, "failed to parse manifest") at every silent-fallback branch
- [ ] #2 Update the per-stack module docs to mention parse failures are reported via tracing at WARN level
<!-- AC:END -->
