---
id: TASK-1627
title: >-
  READ-8: build_read_json_auto_sql duplicates format! arms across Some/None
  extra_opts
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:12'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest/sql.rs:22-32`

**What**: The `match extra_opts { Some(opts) => ..., None => ... }` produces two near-identical `format!` calls that differ only by the trailing `, {opts}` fragment:

```
match extra_opts {
    Some(opts) => {
        validate_extra_opts(opts)?;
        Ok(format!("CREATE OR REPLACE TABLE {quoted} AS SELECT * FROM read_json_auto('{escaped}', {opts})"))
    }
    None => Ok(format!("CREATE OR REPLACE TABLE {quoted} AS SELECT * FROM read_json_auto('{escaped}')")),
}
```

**Why it matters**: Two copies of the SQL template are easy to drift (e.g. a future schema-evolution change updates one arm and not the other). A single `if let` validation followed by one `format!` removes the duplication without changing the validated/unvalidated semantics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single format! site producing the SQL string
- [ ] #2 validate_extra_opts still runs only when extra_opts is Some
- [ ] #3 Existing tests for build_read_json_auto_sql pass unchanged
<!-- AC:END -->
