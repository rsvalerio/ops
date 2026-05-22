---
id: TASK-1599
title: >-
  ERR-1: read_field warns identically for JSON null vs wrong-shape and floods
  logs per-file on schema drift
status: Done
assignee:
  - TASK-1634
created_date: '2026-05-21 22:53'
updated_date: '2026-05-22 08:47'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:93-112`

**What**: `read_field` warns "present but not an integer/float" identically for explicit `null` and for "wrong shape" (e.g., string `"42"`). The warn also fires once per missing-typed field per file — on a malformed export that's 4 sections × 3 numeric fields × N files of warn spam.

**Why it matters**: Operators silence noisy warns, then legitimate schema-drift signals get filtered out too. The TASK-0984-era diagnostic discipline is undermined.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Differentiate Value::Null (downgrade to debug! or skip) from wrong-shape (keep warn!)
- [ ] #2 Cap warning volume per flatten_coverage_json call — emit at most one summary warn per (section, field) pair, batched via a small HashSet
- [ ] #3 Test asserts a multi-file malformed case produces ≤1 warn per (section, field), not N
<!-- AC:END -->
