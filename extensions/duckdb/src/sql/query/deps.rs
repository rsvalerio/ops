//! Dependency count and per-crate dependency queries.

use crate::DuckDb;
use std::collections::HashMap;

use super::helpers::{query_project_scalar, query_rows_fold, QuerySpec};

/// Query total dependency count from `crate_dependencies`.
pub fn query_dependency_count(db: &DuckDb) -> anyhow::Result<usize> {
    let count = query_project_scalar(
        db,
        "crate_dependencies",
        "SELECT COUNT(DISTINCT dependency_name) FROM crate_dependencies",
        "query_dependency_count",
    )?;
    Ok(usize::try_from(count).unwrap_or(0))
}

/// Query per-crate external dependencies (name + version_req) from `crate_dependencies` view.
///
/// Returns a map of crate_name -> Vec<(dep_name, version_req)>, sorted by dep name.
/// Returns an empty map if the view doesn't exist (graceful degradation).
pub fn query_crate_deps(db: &DuckDb) -> anyhow::Result<HashMap<String, Vec<(String, String)>>> {
    query_rows_fold(
        db,
        &QuerySpec {
            table: "crate_dependencies",
            sql: "SELECT crate_name, dependency_name, version_req \
                  FROM crate_dependencies \
                  WHERE dependency_kind = 'normal' \
                  ORDER BY crate_name, dependency_name",
            label: "query_crate_deps",
        },
        |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        },
        HashMap::new(),
        |map, (crate_name, dep_name, version_req)| {
            map.entry(crate_name)
                .or_default()
                .push((dep_name, version_req));
        },
    )
}

/// Query per-crate external dependency counts from `crate_dependencies` view.
///
/// Returns a map of package name -> normal dependency count.
/// Returns an empty map if the view doesn't exist (graceful degradation).
pub fn query_crate_dep_counts(db: &DuckDb) -> anyhow::Result<HashMap<String, i64>> {
    query_rows_fold(
        db,
        &QuerySpec {
            table: "crate_dependencies",
            sql: "SELECT crate_name, COUNT(*) AS dep_count \
                  FROM crate_dependencies \
                  WHERE dependency_kind = 'normal' \
                  GROUP BY crate_name",
            label: "query_crate_dep_counts",
        },
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        HashMap::new(),
        |map, (name, count)| {
            map.insert(name, count);
        },
    )
}
