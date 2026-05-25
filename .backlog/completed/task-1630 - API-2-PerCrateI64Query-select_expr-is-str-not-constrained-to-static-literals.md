---
id: TASK-1630
title: >-
  API-2: PerCrateI64Query select_expr is &str, not constrained to &'static
  literals
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:12'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - api
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/helpers.rs:300-324`

**What**: `query_per_crate_i64` interpolates `q.select_expr: &str` directly into the SQL alongside `TableName` / `JoinAlias` / `JoinColumn` newtypes:

```
let sql = format!(
    "{cte} \
     SELECT m.path, {select_expr} \
     FROM members m \
     LEFT JOIN {table} {join_alias} ON starts_with({join_alias}.{join_column}, m.path || '/') \
     GROUP BY m.path",
);
```

All current callers in `loc.rs`/`deps.rs` pass `&'static str` literals (e.g. `"COUNT(f.file)"`, `"COALESCE(SUM(f.code), 0)"`), so the live call sites are safe. The type, however, does not enforce that — a future caller could pass a dynamically-built `String` reference (e.g. derived from config) and silently widen the SQL-injection surface.

**Why it matters**: The surrounding newtypes (`TableName`, `JoinAlias`, `JoinColumn`) exist precisely to make "static-vetted SQL fragment" a compile-time property. `select_expr` is the one remaining hole. Tightening it to `&'static str` (or a `SelectExpr` newtype constructed only from `&'static str`, mirroring `TableName::from_static`) preserves every current call site while making any future dynamic insertion a build failure rather than a silent regression.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 select_expr field of PerCrateI64Query is &'static str (or wrapped in a SelectExpr newtype constructible only from &'static str)
- [ ] #2 All call sites in loc.rs and deps.rs compile without changes other than the type narrowing
- [ ] #3 A doc comment on the field/newtype states the static-only invariant and links the SEC-12 rationale
<!-- AC:END -->
