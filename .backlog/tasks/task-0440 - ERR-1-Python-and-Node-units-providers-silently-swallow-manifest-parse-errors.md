---
id: TASK-0440
title: 'ERR-1: Python and Node units providers silently swallow manifest parse errors'
status: To Do
assignee:
  - TASK-0532
created_date: '2026-04-28 04:44'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions-python/about/src/units.rs:53-60` (read_workspace_members — both read and toml errors return empty)
- `extensions-python/about/src/units.rs:104-108` (parse_package_metadata — toml parse error returns (None, None, None))
- `extensions-node/about/src/units.rs:184-187` (parse_package_metadata — JSON parse error returns (None, None, None))

**What**: These helpers swallow read/parse errors with no tracing diagnostic, while the corresponding identity providers (parse_pyproject, parse_package_json) emit tracing::warn! on parse errors and tracing::debug! on non-NotFound read errors per TASK-0394 contract. The units pipeline therefore loses observability for the same class of failures the identity pipeline reports.

**Why it matters**: A malformed member pyproject.toml/package.json causes the unit to disappear from the project card with no log line, indistinguishable from "no [project] table". TASK-0394 (Done) only covered the identity providers; the new units modules have the same pattern uncovered.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each Err(_) arm at the listed sites emits tracing::warn! (parse) / tracing::debug! (non-NotFound IO) with path and error structured fields
- [ ] #2 Behavior on success is unchanged (still returns empty / (None, None, None))
- [ ] #3 At least one test asserts the warn-level event is emitted on a malformed manifest, or a doc comment captures the contract alongside the existing identity provider
<!-- AC:END -->
