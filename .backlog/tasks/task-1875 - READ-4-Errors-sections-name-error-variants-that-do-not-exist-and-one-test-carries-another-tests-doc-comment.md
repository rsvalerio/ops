---
id: TASK-1875
title: >-
  READ-4: # Errors sections name error variants that do not exist, and one test
  carries another test's doc comment
status: To Do
assignee:
  - TASK-2006
created_date: '2026-08-27 15:31'
updated_date: '2026-08-28 14:16'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/duckdb/src/sql/ingest/sidecar.rs
  - extensions/duckdb/src/sql/validation.rs
  - extensions/duckdb/src/ingestor.rs
  - extensions/duckdb/src/sql/query/helpers.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: candidates to inspect -->

**What**: Several doc comments describe an API that is not the one in the file. Each is a broken intra-doc link (`rustdoc::broken_intra_doc_links`) or a stale attribution:

1. `extensions/duckdb/src/sql/ingest/sidecar.rs:41` — `read_workspace_sidecar`'s `# Errors` promises `[DbError::SidecarTooLarge]`. No such variant exists in `error.rs`; the oversize path actually returns `DbError::Io(ErrorKind::InvalidData)`, which is what the test `read_workspace_sidecar_rejects_oversize_input` asserts. A caller matching on the documented variant cannot compile.
2. `extensions/duckdb/src/sql/validation.rs:309` — `validate_path_chars`'s `# Errors` names `[SqlError::InvalidPathChars]` (plural). The variant is `InvalidPathChar` (singular).
3. `extensions/duckdb/src/sql/validation.rs:367` — same misspelling in `prepare_path_for_sql`'s `# Errors`.
4. `extensions/duckdb/src/ingestor.rs:94-95, 180` — `collect_sidecar` links `[DbError::Io]` / `[DbError::Serialization]` and `load_with_sidecar` links `[DbError]`, but the module imports only `crate::error::DbResult` (`ingestor.rs:4`), so `DbError` is not in scope and none of these three links resolve.
5. `extensions/duckdb/src/sql/query/helpers.rs:437-444` — the paragraph beginning "SEC-12 AC #1: an attacker-shaped 'prefix' cannot reach the formatted SQL because `ColumnAlias::new` rejects non-identifier strings…" is attached to `collect_per_crate_map_keeps_one_entry_for_duplicate_keys`, a test about duplicate map keys. The SEC-12 claim belongs to `column_alias_rejects_non_identifier_prefix` at the bottom of the same module. Two unrelated doc paragraphs were merged onto one test during an edit.

Minor sibling: `helpers.rs:326-328` says a duplicate key is "surface it instead of silently overwriting", but the code warns *and* overwrites, and the test asserts "the second row wins" — the comment overstates what happens.

**Why it matters**: `# Errors` sections are the contract callers match on; naming a variant that does not exist sends them to code that will not compile and hides the variant that is actually returned. The misattached SEC-12 comment is worse than none: it tells a future reader that a security assertion is covered by a test that does not make it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every # Errors section in the crate names only variants that exist; DbError::SidecarTooLarge and SqlError::InvalidPathChars references are corrected
- [ ] #2 DbError is in scope in ingestor.rs (or the links are fully qualified) so the intra-doc links resolve
- [ ] #3 cargo doc for ops-duckdb emits no broken_intra_doc_links warnings
- [ ] #4 The SEC-12 AC #1 paragraph in query/helpers.rs is moved onto the test it describes and the duplicate-key comment matches the warn-and-overwrite behaviour
<!-- AC:END -->
