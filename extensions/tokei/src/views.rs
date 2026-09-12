//! SQL utilities for tokei code statistics.
//!
//! # Security (SEC-001)
//!
//! Identifier validation is handled by `ops_sqlite::sql` (shared
//! defense-in-depth validation), and the staged JSON reaches the engine as
//! a bound parameter — never interpolated. This module only contains
//! tokei-specific SQL specs.

use ops_sqlite::sql::{CreateViewSql, JsonColumn, JsonTableLoad, TableName};

/// The `tokei_files` load spec.
///
/// The staged sidecar is a flat array with one record per file, produced by
/// `report_to_json` — the column list mirrors that record shape exactly
/// (`language`/`file` text, the five counts integers). A record missing one
/// of these keys fails the `NOT NULL` insert loudly, so a collector/output
/// drift surfaces as an ingest error rather than a silent NULL column.
pub const TOKEI_FILES_LOAD: JsonTableLoad = JsonTableLoad::flat_array(
    "tokei_files",
    &[
        JsonColumn::text("language", "$.language"),
        JsonColumn::text("file", "$.file"),
        JsonColumn::integer("code", "$.code"),
        JsonColumn::integer("comments", "$.comments"),
        JsonColumn::integer("blanks", "$.blanks"),
        JsonColumn::integer("lines", "$.lines"),
    ],
);

/// SEC-12 (TASK-0593) / ERR-5 (TASK-1003): identifiers are routed through
/// the const-validated [`TableName::from_static`] newtype so the
/// compile-time invariant replaces the runtime `quoted_ident` Result.
///
/// Both literals are valid SQL identifiers — the assert in `from_static`
/// would fire at build time on a typo.
///
/// SEC-12 / TASK-1864: the statement is returned as the gated
/// [`CreateViewSql`] newtype, whose only constructor takes const-validated
/// [`TableName`]s plus a `&'static str` body — a runtime-derived `String`
/// can no longer reach `load_with_sidecar`.
pub fn tokei_languages_view_sql() -> CreateViewSql {
    CreateViewSql::create_or_replace(
        TableName::from_static("tokei_languages"),
        TableName::from_static("tokei_files"),
        "SELECT language, COUNT(*) AS files, SUM(code) AS code, \
         SUM(comments) AS comments, SUM(blanks) AS blanks, SUM(lines) AS lines \
         FROM <source> GROUP BY language ORDER BY code DESC",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SEC-12: the load spec's DDL quotes every identifier and declares the
    /// typed columns the queries decode against.
    #[test]
    fn tokei_files_load_declares_typed_quoted_columns() {
        let sql = TOKEI_FILES_LOAD.create_table_sql().to_string();
        assert!(
            sql.contains("CREATE TABLE \"tokei_files\" (\"language\" TEXT NOT NULL"),
            "expected quoted text column: {sql}"
        );
        assert!(
            sql.contains("\"file\" TEXT NOT NULL"),
            "expected quoted file column: {sql}"
        );
        assert!(
            sql.contains("\"code\" INTEGER NOT NULL")
                && sql.contains("\"comments\" INTEGER NOT NULL")
                && sql.contains("\"blanks\" INTEGER NOT NULL")
                && sql.contains("\"lines\" INTEGER NOT NULL"),
            "expected integer count columns: {sql}"
        );
    }

    #[test]
    fn tokei_languages_view_sql_contains_aggregation() {
        let sql = tokei_languages_view_sql().to_string();
        assert!(sql.contains("tokei_languages"));
        assert!(sql.contains("GROUP BY language"));
        assert!(sql.contains("SUM(code)"));
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("ORDER BY code DESC"));
    }

    /// SEC-12: identifiers must be double-quoted, matching the parity policy
    /// of the load spec's DDL.
    #[test]
    fn tokei_languages_view_sql_quotes_identifiers() {
        let sql = tokei_languages_view_sql().to_string();
        assert!(
            sql.contains("\"tokei_languages\""),
            "view name should be double-quoted: {sql}"
        );
        assert!(
            sql.contains("\"tokei_files\""),
            "table name should be double-quoted: {sql}"
        );
    }
}
