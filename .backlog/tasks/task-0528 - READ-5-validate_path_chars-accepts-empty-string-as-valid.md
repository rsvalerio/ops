---
id: TASK-0528
title: 'READ-5: validate_path_chars accepts empty string as valid'
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 06:53'
updated_date: '2026-04-28 19:03'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/validation.rs:123`

**What**: validate_path_chars("") returns Ok — the for-loop has zero iterations. Pinned by validate_path_chars_empty_is_ok test, but no caller meaningfully wants a zero-length path.

**Why it matters**: Empty-path acceptance is a silent footgun: a caller that forgot to populate a path slips through validation and produces SQL like `read_json_auto('')`. Failing fast is cheaper than tracing an empty quoted literal in DuckDB error output.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either reject empty paths or document the rationale on the function
- [ ] #2 Tighten test to assert prepare_path_for_sql rejects empty
<!-- AC:END -->
