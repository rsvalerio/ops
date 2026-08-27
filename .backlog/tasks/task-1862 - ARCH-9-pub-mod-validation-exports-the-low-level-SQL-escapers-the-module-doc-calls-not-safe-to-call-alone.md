---
id: TASK-1862
title: >-
  ARCH-9: pub mod validation exports the low-level SQL escapers the module doc
  calls 'not safe to call alone'
status: Triage
assignee: []
created_date: '2026-08-27 15:29'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - extensions/duckdb/src/sql/mod.rs
  - extensions/duckdb/src/sql/validation.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/mod.rs:309` (`pub mod validation;`), `extensions/duckdb/src/sql/validation.rs:16-20, 275, 288`

**What**: `sql/mod.rs` curates its re-export surface and states the intent explicitly:

```rust
// `SqlError` and `quoted_ident` cross the crate boundary; the rest of the
// granular validation helpers stay module-internal (ARCH-9).
pub use validation::{quoted_ident, ExtraOpts, SqlError, TableName};
```

That comment is false: the module itself is declared `pub mod validation;`, so every `pub fn` inside it is reachable as `ops_duckdb::sql::validation::*` — including the two the module's own doc header flags as unsafe to use in isolation:

> `escape_sql_string` / `sanitize_path_for_sql` — low-level escaping used inside `prepare_path_for_sql`; **not safe to call alone**.

`escape_sql_string` only doubles `'` and neutralises NUL; on its own it does not reject `;`, `$`, backticks, control characters, non-ASCII homoglyphs, or `..` traversal — every check `validate_path_chars` / `validate_no_traversal` add. A downstream extension that reaches for the obvious-looking `escape_sql_string` before interpolating a path into `read_json_auto('…')` silently opts out of the whole defence-in-depth stack the crate documents.

Also exported unintentionally: `validate_identifier`, `validate_path_chars`, `validate_no_traversal`, `validate_extra_opts`, `prepare_path_for_sql`, `EXTRA_OPTS_MAX_*`.

**Why it matters**: The crate's SEC-12 story is "you cannot reach the SQL builder without passing a construction gate" (`TableName::from_static`, `ExtraOpts::new`, `quoted_ident`). Leaving the raw escapers publicly reachable, while the code comment claims they are private, hands a future caller a documented-as-unsafe shortcut past that gate and gives reviewers a false reading of the public surface.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The validation module is no longer blanket-public: either pub(crate) with the four intended items re-exported, or the individual helpers demoted to pub(crate)/pub(super)
- [ ] #2 escape_sql_string and sanitize_path_for_sql are unreachable from outside the crate; prepare_path_for_sql remains the only standalone path helper on the public surface if one is needed
- [ ] #3 Downstream crates that legitimately use ops_duckdb::sql::validation::* (e.g. TableName in extensions/tokei) still compile via the curated re-export list
- [ ] #4 The ARCH-9 comment in sql/mod.rs describes the surface that actually exists
<!-- AC:END -->
