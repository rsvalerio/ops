//! SQL utilities for Rust LOC statistics.
//!
//! # Security
//!
//! Path validation and SQL escaping are handled by `ops_duckdb::sql`
//! (shared defense-in-depth validation). This module only contains
//! rust-loc-specific SQL generation.

use ops_duckdb::sql::{CreateTableSql, CreateViewSql, SqlError, TableName};
use std::path::Path;

/// Builds the `CREATE TABLE` statement loading `rust_loc_files.json` at
/// `path` into the `rust_loc_files` table.
///
/// # Errors
///
/// [`SqlError`] if `path` fails path validation; the table name is a valid
/// static identifier.
pub fn rust_loc_files_create_sql(path: &Path) -> Result<CreateTableSql, SqlError> {
    ops_duckdb::sql::create_table_from_json_sql("rust_loc_files", path, None)
}

/// Builds the `CREATE OR REPLACE VIEW` statement aggregating
/// `rust_loc_files` into the per-region `rust_loc_summary` view.
///
/// Both identifiers are const-validated through [`TableName::from_static`], so
/// the statement is infallible and needs no `Result`. It is returned as the
/// gated [`CreateViewSql`] newtype, which is the only thing `load_with_sidecar`
/// will execute — SQL reaching the database is therefore provably
/// builder-produced rather than assembled from a string.
pub fn rust_loc_summary_view_sql() -> CreateViewSql {
    CreateViewSql::create_or_replace(
        TableName::from_static("rust_loc_summary"),
        TableName::from_static("rust_loc_files"),
        "SELECT region, COUNT(*) AS files, SUM(code) AS code, \
         SUM(docs) AS docs, SUM(comments) AS comments, \
         SUM(blanks) AS blanks, SUM(lines) AS lines \
         FROM <source> GROUP BY region ORDER BY code DESC",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    ops_duckdb::test_create_sql_validation!(rust_loc_files_create_sql, "rust_loc_files.json");

    #[test]
    fn rust_loc_summary_view_sql_contains_aggregation() {
        let sql = rust_loc_summary_view_sql().to_string();
        assert!(sql.contains("rust_loc_summary"));
        assert!(sql.contains("GROUP BY region"));
        assert!(sql.contains("SUM(code)"));
        assert!(sql.contains("COUNT(*)"));
    }

    #[test]
    fn rust_loc_summary_view_sql_quotes_identifiers() {
        let sql = rust_loc_summary_view_sql().to_string();
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
