//! Dependency count and per-crate dependency queries.

use crate::DuckDb;
use std::collections::HashMap;

use super::helpers::{query_project_scalar, query_rows_fold, QuerySpec};

/// Query total normal-dependency count from `crate_dependencies`.
///
/// ERR-1 (TASK-0506): a negative i64 from COUNT (which DuckDB should never
/// emit but a future cast or schema bug could) used to be silently coerced
/// to 0. Now we surface the anomaly via `tracing::warn` before falling back
/// so a misbehaving view doesn't impersonate "no dependencies".
///
/// PATTERN-1 / TASK-1075: filter to `dependency_kind = 'normal'` so the
/// scalar matches the "Dependencies" identity-card label rendered by
/// `extensions-rust/about/src/identity/metrics.rs`. Without the filter,
/// the count silently includes dev- and build-deps (often 2-3× normal
/// — `serde-test`, `tempfile`, …), producing operator-visible
/// misreporting. Sister queries `query_crate_deps` and
/// `query_crate_dep_counts` already filter on `dependency_kind =
/// 'normal'`; this brings the project-total scalar into line with that
/// policy.
pub fn query_dependency_count(db: &DuckDb) -> anyhow::Result<usize> {
    let count = query_project_scalar(
        db,
        "crate_dependencies",
        "SELECT COUNT(DISTINCT dependency_name) FROM crate_dependencies \
         WHERE dependency_kind = 'normal'",
        "query_dependency_count",
    )?;
    Ok(coerce_count_to_usize(count))
}

/// Convert a `COUNT(*)` scalar to `usize`, logging anomalous (negative)
/// values via `tracing::warn` instead of silently returning 0. Negative
/// values from DuckDB COUNT should be impossible; surfacing them lets a
/// schema bug be diagnosed instead of presenting as "no data".
fn coerce_count_to_usize(count: i64) -> usize {
    match usize::try_from(count) {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(
                count,
                error = %e,
                "query_dependency_count: negative scalar from COUNT; coercing to 0"
            );
            0
        }
    }
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

#[cfg(test)]
mod tests {
    use super::coerce_count_to_usize;

    /// ERR-1 (TASK-0506): a negative scalar coerces to 0 (the safe degraded
    /// value) but does not panic. The accompanying warn covers the alerting
    /// requirement; this test pins the value-level contract.
    #[test]
    fn negative_count_coerces_to_zero() {
        assert_eq!(coerce_count_to_usize(-1), 0);
        assert_eq!(coerce_count_to_usize(i64::MIN), 0);
    }

    #[test]
    fn non_negative_count_round_trips() {
        assert_eq!(coerce_count_to_usize(0), 0);
        assert_eq!(coerce_count_to_usize(42), 42);
    }
}
