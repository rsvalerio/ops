---
id: TASK-0620
title: >-
  DUP-3: parse_package_metadata duplicated structurally between Node and Python
  units providers
status: Triage
assignee: []
created_date: '2026-04-29 05:21'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:196`

**What**: extensions-node/about/src/units.rs:196-215 (parse_package_metadata for package.json) and extensions-python/about/src/units.rs:114-136 (for pyproject.toml) implement the same shape: deserialize-or-warn, extract (name, version, description), description-trim-and-empty-filter. They differ only in format crate (serde_json vs toml) and the projection inside parsed root.

**Why it matters**: Two stack providers carrying same metadata-extraction skeleton means a behaviour change has to be applied twice — TASK-0440 already addressed parse-error swallowing variant. Folding shared shape into ops_about keeps it from re-fanning back out as Java/Maven adds a units provider.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Shared helper in ops_about::workspace returns (Option<String>, Option<String>, Option<String>) from parsed-manifest projection
- [ ] #2 Both Node and Python units.rs use it
- [ ] #3 All existing units-provider tests pass
<!-- AC:END -->
