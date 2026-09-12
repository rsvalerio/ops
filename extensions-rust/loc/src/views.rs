//! SQL utilities for Rust LOC statistics.
//!
//! # Security
//!
//! Path validation and identifier gating are handled by `ops_sqlite::sql`
//! (shared defense-in-depth validation). This module only contains
//! rust-loc-specific SQL specs.

use ops_sqlite::sql::{CreateViewSql, JsonColumn, JsonTableLoad, TableName};

/// Declarative load spec for `rust_loc_files.json`: one row per file, one
/// column per counter region.
///
/// SEC-12: the table and column identifiers are const-validated at
/// construction, and the staged JSON reaches the engine as a bound `?1`
/// parameter — the load executes [`JsonTableLoad`]'s DDL batch +
/// `json_each` insert, never a path-bearing `read_json_auto` statement.
pub const RUST_LOC_FILES_LOAD: JsonTableLoad = JsonTableLoad::flat_array(
    "rust_loc_files",
    &[
        JsonColumn::text("file", "$.file"),
        JsonColumn::text("region", "$.region"),
        JsonColumn::integer("code", "$.code"),
        JsonColumn::integer("docs", "$.docs"),
        JsonColumn::integer("comments", "$.comments"),
        JsonColumn::integer("blanks", "$.blanks"),
        JsonColumn::integer("lines", "$.lines"),
    ],
);

/// Builds the `rust_loc_summary` view statement aggregating
/// `rust_loc_files` into per-region totals.
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

    /// SEC-12: the load spec's DDL quotes the table name and declares typed
    /// NOT NULL columns, mirroring the builder tests in `ops_sqlite`.
    #[test]
    fn rust_loc_files_load_declares_typed_columns() {
        let sql = RUST_LOC_FILES_LOAD.create_table_sql().to_string();
        assert!(
            sql.contains("DROP TABLE IF EXISTS \"rust_loc_files\";"),
            "expected drop+create batch: {sql}"
        );
        assert!(
            sql.contains("\"file\" TEXT NOT NULL")
                && sql.contains("\"region\" TEXT NOT NULL")
                && sql.contains("\"code\" INTEGER NOT NULL")
                && sql.contains("\"lines\" INTEGER NOT NULL"),
            "expected typed NOT NULL columns: {sql}"
        );
    }
}
