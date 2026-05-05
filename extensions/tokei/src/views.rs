//! SQL utilities for tokei code statistics.
//!
//! # Security (SEC-001)
//!
//! Path validation and SQL escaping are handled by `ops_duckdb::sql`
//! (shared defense-in-depth validation). This module only contains
//! tokei-specific SQL generation.

use ops_duckdb::sql::{SqlError, TableName};
use std::path::Path;

pub fn tokei_files_create_sql(path: &Path) -> Result<String, SqlError> {
    ops_duckdb::sql::create_table_from_json_sql("tokei_files", path, None)
}

/// SEC-12 (TASK-0593) / ERR-5 (TASK-1003): identifiers are routed through
/// the const-validated [`TableName::from_static`] newtype so the
/// compile-time invariant replaces the runtime `quoted_ident` Result.
/// Both literals are valid SQL identifiers — the assert in
/// `from_static` would fire at build time on a typo, eliminating the
/// pre-prod `Result<_, SqlError>` whose `Err` variant could never occur
/// and the `expect("static idents must validate")` calls in tests.
pub fn tokei_languages_view_sql() -> String {
    let view = TableName::from_static("tokei_languages").quoted();
    let table = TableName::from_static("tokei_files").quoted();
    format!(
        "CREATE OR REPLACE VIEW {view} AS \
         SELECT language, COUNT(*) AS files, SUM(code) AS code, \
         SUM(comments) AS comments, SUM(blanks) AS blanks, SUM(lines) AS lines \
         FROM {table} GROUP BY language ORDER BY code DESC"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    ops_duckdb::test_create_sql_validation!(tokei_files_create_sql, "tokei_files.json");

    #[test]
    fn tokei_languages_view_sql_contains_aggregation() {
        let sql = tokei_languages_view_sql();
        assert!(sql.contains("tokei_languages"));
        assert!(sql.contains("GROUP BY language"));
        assert!(sql.contains("SUM(code)"));
        assert!(sql.contains("COUNT(*)"));
        assert!(sql.contains("ORDER BY code DESC"));
    }

    /// SEC-12: identifiers must be double-quoted, matching the parity policy
    /// of the sister `tokei_files_create_sql` helper.
    #[test]
    fn tokei_languages_view_sql_quotes_identifiers() {
        let sql = tokei_languages_view_sql();
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
