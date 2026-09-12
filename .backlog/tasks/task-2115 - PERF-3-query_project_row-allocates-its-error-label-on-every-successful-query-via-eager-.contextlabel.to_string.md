---
id: TASK-2115
title: 'PERF-3: query_project_row allocates its error label on every successful query via eager .context(label.to_string())'
status: Done
assignee: []
created_date: '2026-09-08 06:53'
updated_date: '2026-09-09 18:22'
labels:
  - code-review-rust
  - performance
dependencies: []
parent_task_id: 'TASK-2244'
modified_files:
  - extensions/duckdb/src/sql/query/helpers.rs
priority: low
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/helpers.rs:171`

**What**: `query_project_row` ends with

```rust
conn.query_row(spec.sql, [], row_mapper)
    .context(label.to_string())
```

`anyhow::Context::context` takes its argument by value and evaluates it
eagerly, so `label.to_string()` allocates a `String` on the success path of
every project-level query and immediately drops it. Every other `?` site in the
same module uses the lazy `with_context(|| format!("… {label}"))` form
(helpers.rs:122, 128, 131, 134, 165; also `collect_per_crate_map` at 296/299/302),
so this line is the odd one out.

`query_project_row` is the shared prologue behind `query_project_scalar`,
`query_project_coverage`, `query_project_loc`, `query_project_file_count`,
`query_dependency_count` and `query_rust_loc_file_count` — it runs several times
per `ops about` render.

This is the same shape TASK-1243 already fixed in
`SidecarIngestorConfig::create_tables_with`, where the `format!` was moved
inside the `map_err` closure precisely so the success path allocates nothing.

**Why it matters**: small but free; more importantly it is an inconsistency
that invites the pattern to spread — the surrounding code establishes the lazy
form as the house style for exactly this reason.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 query_project_row uses with_context(|| …) so no String is allocated on the success path
- [x] #2 the error message produced on failure is unchanged (still carries the query label)

<!-- AC:END -->
