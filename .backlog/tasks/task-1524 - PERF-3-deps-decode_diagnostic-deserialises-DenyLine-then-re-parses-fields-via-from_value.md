---
id: TASK-1524
title: >-
  PERF-3: deps decode_diagnostic deserialises DenyLine then re-parses fields via
  from_value
status: To Do
assignee:
  - TASK-1574
created_date: '2026-05-19 07:32'
updated_date: '2026-05-19 16:45'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs:179-204`

**What**: Each diagnostic line is parsed twice: once into `DenyLine { fields: serde_json::Value }`, then `serde_json::from_value::<DiagnosticFields>(fields)` walks the in-memory Value tree again. This buffers the full fields subtree as Value then reallocates owned Strings on the second pass.

**Why it matters**: cargo-deny stderr can be hundreds of diagnostics on busy workspaces; the double-parse doubles allocations on the hot path and the intermediate Value is pure waste.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Define a single #[derive(Deserialize)] struct that flattens type + diagnostic fields (or use #[serde(tag = "type", content = "fields")] enum) so one from_str call yields the typed diagnostic
- [ ] #2 Non-diagnostic line types still skipped (untagged variant or pre-filter)
- [ ] #3 Existing parse_deny_* tests pass
<!-- AC:END -->
