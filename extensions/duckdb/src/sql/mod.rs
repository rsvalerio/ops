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

pub use ingest::{
    checksum_file, create_table_from_json_sql, data_dir_for_db, default_data_dir, default_db_path,
    io_err, provide_via_ingestor, query_rows_to_json, read_workspace_sidecar,
    remove_workspace_sidecar, table_has_data, write_workspace_sidecar,
};
pub use query::{
    query_crate_coverage, query_crate_dep_counts, query_crate_deps, query_crate_file_count,
    query_crate_loc, query_dependency_count, query_project_coverage, query_project_file_count,
    query_project_languages, query_project_loc, CrateCoverage,
};
pub use validation::{
    escape_sql_string, prepare_path_for_sql, sanitize_path_for_sql, validate_identifier,
    validate_no_traversal, validate_path_chars, SqlError,
};
