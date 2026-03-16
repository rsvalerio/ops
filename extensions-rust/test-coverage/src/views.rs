//! SQL utilities for LLVM coverage data.
//!
//! # Security (SEC-001)
//!
//! Path validation and SQL escaping are handled by `ops_duckdb::sql`
//! (shared defense-in-depth validation). This module only contains
//! coverage-specific SQL generation.

use ops_duckdb::sql::SqlError;
use std::path::Path;

pub fn coverage_files_create_sql(path: &Path) -> Result<String, SqlError> {
    ops_duckdb::sql::create_table_from_json_sql("coverage_files", path, None)
}

pub fn coverage_summary_view_sql() -> String {
    "CREATE OR REPLACE VIEW coverage_summary AS \
     SELECT \
     SUM(lines_count) AS lines_count, \
     SUM(lines_covered) AS lines_covered, \
     CASE WHEN SUM(lines_count) > 0 \
         THEN ROUND(SUM(lines_covered) * 100.0 / SUM(lines_count), 2) \
         ELSE 0.0 END AS lines_percent, \
     SUM(functions_count) AS functions_count, \
     SUM(functions_covered) AS functions_covered, \
     CASE WHEN SUM(functions_count) > 0 \
         THEN ROUND(SUM(functions_covered) * 100.0 / SUM(functions_count), 2) \
         ELSE 0.0 END AS functions_percent, \
     SUM(regions_count) AS regions_count, \
     SUM(regions_covered) AS regions_covered, \
     SUM(regions_notcovered) AS regions_notcovered, \
     CASE WHEN SUM(regions_count) > 0 \
         THEN ROUND(SUM(regions_covered) * 100.0 / SUM(regions_count), 2) \
         ELSE 0.0 END AS regions_percent, \
     SUM(branches_count) AS branches_count, \
     SUM(branches_covered) AS branches_covered, \
     SUM(branches_notcovered) AS branches_notcovered, \
     CASE WHEN SUM(branches_count) > 0 \
         THEN ROUND(SUM(branches_covered) * 100.0 / SUM(branches_count), 2) \
         ELSE 0.0 END AS branches_percent \
     FROM coverage_files"
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    ops_duckdb::test_create_sql_validation!(coverage_files_create_sql, "coverage_files.json");

    #[test]
    fn coverage_summary_view_sql_contains_aggregation() {
        let sql = coverage_summary_view_sql();
        assert!(sql.contains("CREATE OR REPLACE VIEW coverage_summary"));
        assert!(sql.contains("SUM(lines_count)"));
        assert!(sql.contains("SUM(lines_covered)"));
        assert!(sql.contains("SUM(functions_count)"));
        assert!(sql.contains("SUM(regions_count)"));
        assert!(sql.contains("SUM(branches_count)"));
        assert!(sql.contains("CASE WHEN"));
    }

    #[test]
    fn coverage_summary_view_sql_has_all_percentage_columns() {
        let sql = coverage_summary_view_sql();
        assert!(sql.contains("AS lines_percent"));
        assert!(sql.contains("AS functions_percent"));
        assert!(sql.contains("AS regions_percent"));
        assert!(sql.contains("AS branches_percent"));
    }

    #[test]
    fn coverage_summary_view_sql_has_zero_division_guards() {
        let sql = coverage_summary_view_sql();
        // Each metric type has a CASE WHEN ... > 0 guard with ELSE 0.0
        assert_eq!(
            sql.matches("ELSE 0.0 END").count(),
            4,
            "should have zero-division guards for lines, functions, regions, branches"
        );
    }

    #[test]
    fn coverage_summary_view_sql_has_notcovered_columns() {
        let sql = coverage_summary_view_sql();
        assert!(sql.contains("regions_notcovered"));
        assert!(sql.contains("branches_notcovered"));
    }
}
