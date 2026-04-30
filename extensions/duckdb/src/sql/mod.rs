//! Shared SQL utilities for DuckDB extensions.
//!
//! # Security (SEC-001)
//!
//! This module constructs SQL queries with string interpolation for DuckDB's
//! `read_json_auto()` function. While parameterized queries are preferred, DuckDB
//! requires a string literal for file paths in this context.
//!
//! We employ **defense-in-depth** validation to prevent SQL injection:
//!
//! 1. **validate_no_traversal()**: Blocks `..` path components
//! 2. **validate_path_chars()**: Rejects dangerous characters (`;`, `$`, backticks)
//! 3. **sanitize_path_for_sql()**: Removes null bytes
//! 4. **escape_sql_string()**: Escapes quotes and backslashes
//!
//! These layers ensure that even if one check fails, others provide protection.
//! The path is validated before any SQL is constructed.

pub mod ingest;
pub mod query;
pub mod validation;

/// Run `query_fn` and return its `Ok` value, or log the error at `warn` and
/// return `fallback`.
///
/// DUP-1 (TASK-0475): consolidates the seven near-identical
/// `match query_X(db) { Ok(v) => v, Err(e) => { tracing::warn!(...); fallback } }`
/// blocks scattered across `extensions-rust/about`. Callers pass the query
/// label and a brief description of the degraded outcome so the log line
/// stays diagnostic.
pub fn query_or_warn<T, F>(label: &'static str, degraded: &str, fallback: T, query_fn: F) -> T
where
    F: FnOnce() -> anyhow::Result<T>,
{
    match query_fn() {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(query = label, "duckdb query failed; {degraded}: {e:#}");
            fallback
        }
    }
}

pub use ingest::{
    checksum_file, create_table_from_json_sql, data_dir_for_db, default_data_dir, default_db_path,
    external_err, provide_via_ingestor, query_rows_to_json, read_workspace_sidecar,
    remove_workspace_sidecar, sidecar_path, table_has_data, write_workspace_sidecar,
};
pub use query::{
    query_crate_coverage, query_crate_dep_counts, query_crate_deps, query_crate_file_count,
    query_crate_loc, query_dependency_count, query_project_coverage, query_project_file_count,
    query_project_languages, query_project_loc, CrateCoverage,
};
// `SqlError` and `quoted_ident` cross the crate boundary; the rest of the
// granular validation helpers stay module-internal (ARCH-9). `quoted_ident` is
// the SEC-12 defense-in-depth wrapper and is needed at every site that
// interpolates an identifier into a hand-written SQL string (e.g.
// `extensions/tokei/src/views::tokei_languages_view_sql`).
pub use validation::{quoted_ident, SqlError};
