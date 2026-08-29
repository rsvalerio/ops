//! SQL utilities for LLVM coverage data.
//!
//! # Security (SEC-001)
//!
//! Path validation and SQL escaping are handled by `ops_duckdb::sql`
//! (shared defense-in-depth validation). This module only contains
//! coverage-specific SQL generation.

use ops_duckdb::sql::{CreateTableSql, CreateViewSql, SqlError, TableName};
use std::path::Path;

pub fn coverage_files_create_sql(path: &Path) -> Result<CreateTableSql, SqlError> {
    ops_duckdb::sql::create_table_from_json_sql("coverage_files", path, None)
}

/// READ-6 / TASK-1934: every non-percentage SUM is wrapped in
/// `COALESCE(..., 0)`. An ungrouped aggregate over an empty
/// `coverage_files` returns exactly one row whose SUMs are all NULL, so
/// without the wrapper a consumer decoding `lines_count` as a
/// non-nullable integer gets a decode failure instead of a zero. The
/// percentage columns need no wrapper: `NULL > 0` is NULL, which falls to
/// the `ELSE 0.0` arm. This matches `coverage_col_select` in the duckdb
/// extension, which already made the same decision for the same data.
/// SEC-12 / TASK-1864: returned as the gated [`CreateViewSql`] newtype; the
/// view and source identifiers go through the const-validated [`TableName`]
/// and the body is a `&'static str`, so nothing runtime-derived can reach
/// `load_with_sidecar`.
pub fn coverage_summary_view_sql() -> CreateViewSql {
    CreateViewSql::create_or_replace(
        TableName::from_static("coverage_summary"),
        TableName::from_static("coverage_files"),
        "SELECT \
         COALESCE(SUM(lines_count), 0) AS lines_count, \
         COALESCE(SUM(lines_covered), 0) AS lines_covered, \
         CASE WHEN SUM(lines_count) > 0 \
             THEN SUM(lines_covered) * 100.0 / SUM(lines_count) \
             ELSE 0.0 END AS lines_percent, \
         COALESCE(SUM(functions_count), 0) AS functions_count, \
         COALESCE(SUM(functions_covered), 0) AS functions_covered, \
         CASE WHEN SUM(functions_count) > 0 \
             THEN SUM(functions_covered) * 100.0 / SUM(functions_count) \
             ELSE 0.0 END AS functions_percent, \
         COALESCE(SUM(regions_count), 0) AS regions_count, \
         COALESCE(SUM(regions_covered), 0) AS regions_covered, \
         COALESCE(SUM(regions_notcovered), 0) AS regions_notcovered, \
         CASE WHEN SUM(regions_count) > 0 \
             THEN SUM(regions_covered) * 100.0 / SUM(regions_count) \
             ELSE 0.0 END AS regions_percent, \
         COALESCE(SUM(branches_count), 0) AS branches_count, \
         COALESCE(SUM(branches_covered), 0) AS branches_covered, \
         COALESCE(SUM(branches_notcovered), 0) AS branches_notcovered, \
         CASE WHEN SUM(branches_count) > 0 \
             THEN SUM(branches_covered) * 100.0 / SUM(branches_count) \
             ELSE 0.0 END AS branches_percent \
         FROM <source>",
    )
}
