//! Project- and per-crate coverage queries over `coverage_files`.

use crate::DuckDb;
use std::collections::HashMap;

use super::super::ingest::table_exists;
use super::super::validation::validate_path_chars;
use super::helpers::{
    collect_per_crate_map, coverage_col_select, prepare_per_crate, CrateCoverage, PerCrateSetup,
};

/// Query total coverage across the whole project from `coverage_files`.
pub fn query_project_coverage(db: &DuckDb) -> anyhow::Result<CrateCoverage> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_project_coverage")?;

    if !table_exists(&conn, "coverage_files")? {
        return Ok(CrateCoverage::zero());
    }

    let sql = format!("SELECT {} FROM coverage_files", coverage_col_select(""));
    conn.query_row(&sql, [], |row: &duckdb::Row| {
        Ok(CrateCoverage {
            lines_count: row.get(0)?,
            lines_covered: row.get(1)?,
            lines_percent: row.get(2)?,
        })
    })
    .context("querying project coverage")
}

/// Query per-crate coverage from `coverage_files`.
///
/// Returns a map of member path -> CrateCoverage. Members with no matching
/// files get zeroed coverage. Handles both absolute and relative filenames
/// from LLVM coverage output.
pub fn query_crate_coverage(
    db: &DuckDb,
    member_paths: &[&str],
    workspace_root: &str,
) -> anyhow::Result<HashMap<String, CrateCoverage>> {
    validate_path_chars(workspace_root)?;

    let label = "query_crate_coverage";
    let (conn, placeholders, mut paths) =
        match prepare_per_crate(db, "coverage_files", member_paths, label)? {
            PerCrateSetup::Empty => return Ok(HashMap::new()),
            PerCrateSetup::NoTable => {
                return Ok(member_paths
                    .iter()
                    .map(|p| (p.to_string(), CrateCoverage::zero()))
                    .collect())
            }
            PerCrateSetup::Ready(conn, placeholders, paths) => (conn, placeholders, paths),
        };

    // workspace_root is the last bound parameter (? after VALUES placeholders)
    paths.push(workspace_root.to_string());

    let sql = format!(
        "WITH members(path) AS (VALUES {placeholders}) \
         SELECT m.path, {} \
         FROM members m \
         LEFT JOIN coverage_files c \
             ON starts_with(c.filename, m.path || '/') \
             OR starts_with(c.filename, ? || '/' || m.path || '/') \
         GROUP BY m.path",
        coverage_col_select("c.")
    );

    collect_per_crate_map(&conn, &sql, label, &paths, |row| {
        Ok((
            row.get::<_, String>(0)?,
            CrateCoverage {
                lines_count: row.get(1)?,
                lines_covered: row.get(2)?,
                lines_percent: row.get(3)?,
            },
        ))
    })
}
