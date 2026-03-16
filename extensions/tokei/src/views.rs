//! SQL utilities for tokei code statistics.
//!
//! # Security (SEC-001)
//!
//! Path validation and SQL escaping are handled by `ops_duckdb::sql`
//! (shared defense-in-depth validation). This module only contains
//! tokei-specific SQL generation.

use ops_duckdb::sql::SqlError;
use std::path::Path;

pub fn tokei_files_create_sql(path: &Path) -> Result<String, SqlError> {
    ops_duckdb::sql::create_table_from_json_sql("tokei_files", path, None)
}

pub fn tokei_languages_view_sql() -> String {
    "CREATE OR REPLACE VIEW tokei_languages AS \
     SELECT language, COUNT(*) AS files, SUM(code) AS code, \
     SUM(comments) AS comments, SUM(blanks) AS blanks, SUM(lines) AS lines \
     FROM tokei_files GROUP BY language ORDER BY code DESC"
        .to_string()
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
}
