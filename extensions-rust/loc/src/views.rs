//! SQL utilities for Rust LOC statistics.
//!
//! # Security (SEC-001)
//!
//! Path validation and SQL escaping are handled by `ops_duckdb::sql`
//! (shared defense-in-depth validation). This module only contains
//! rust-loc-specific SQL generation.

use ops_duckdb::sql::{SqlError, TableName};
use std::path::Path;

/// # Errors
///
/// [`SqlError`] if `path` fails path validation; the table name is a valid
/// static identifier.
pub fn rust_loc_files_create_sql(path: &Path) -> Result<String, SqlError> {
    ops_duckdb::sql::create_table_from_json_sql("rust_loc_files", path, None)
}

/// SEC-12 / ERR-5: identifiers are routed through the const-validated
/// [`TableName::from_static`] newtype, so the compile-time invariant
/// replaces a runtime `Result` whose `Err` variant could never occur.
#[must_use]
pub fn rust_loc_summary_view_sql() -> String {
    let view = TableName::from_static("rust_loc_summary").quoted();
    let table = TableName::from_static("rust_loc_files").quoted();
    format!(
        "CREATE OR REPLACE VIEW {view} AS \
         SELECT region, COUNT(*) AS files, SUM(code) AS code, \
         SUM(docs) AS docs, SUM(comments) AS comments, \
         SUM(blanks) AS blanks, SUM(lines) AS lines \
         FROM {table} GROUP BY region ORDER BY code DESC"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    ops_duckdb::test_create_sql_validation!(rust_loc_files_create_sql, "rust_loc_files.json");

    #[test]
    fn rust_loc_summary_view_sql_contains_aggregation() {
        let sql = rust_loc_summary_view_sql();
        assert!(sql.contains("rust_loc_summary"));
        assert!(sql.contains("GROUP BY region"));
        assert!(sql.contains("SUM(code)"));
        assert!(sql.contains("COUNT(*)"));
    }

    #[test]
    fn rust_loc_summary_view_sql_quotes_identifiers() {
        let sql = rust_loc_summary_view_sql();
        assert!(
            sql.contains("\"rust_loc_summary\""),
            "view name should be double-quoted: {sql}"
        );
        assert!(
            sql.contains("\"rust_loc_files\""),
            "table name should be double-quoted: {sql}"
        );
    }
}
