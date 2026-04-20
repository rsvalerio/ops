//! Shared query scaffolding: locking, table-existence checks, per-crate builders.

use crate::DuckDb;
use std::collections::HashMap;

use super::super::ingest::table_exists;
use super::super::validation::validate_path_chars;

/// Per-crate coverage data from `coverage_files`.
#[derive(Debug, Clone)]
pub struct CrateCoverage {
    pub lines_count: i64,
    pub lines_covered: i64,
    pub lines_percent: f64,
}

impl CrateCoverage {
    pub fn zero() -> Self {
        Self {
            lines_count: 0,
            lines_covered: 0,
            lines_percent: 0.0,
        }
    }
}

/// Returns the SELECT column expressions for coverage SUM/CASE aggregation.
/// Pass `""` for direct table queries or `"c."` for aliased joins.
pub(super) fn coverage_col_select(prefix: &str) -> String {
    format!(
        "COALESCE(SUM({prefix}lines_count), 0), \
         COALESCE(SUM({prefix}lines_covered), 0), \
         CASE WHEN SUM({prefix}lines_count) > 0 \
             THEN ROUND(SUM({prefix}lines_covered) * 100.0 / SUM({prefix}lines_count), 2) \
             ELSE 0.0 END"
    )
}

/// Query spec bundling table name, SQL, and diagnostic label for `query_rows_fold`.
pub(super) struct QuerySpec<'a> {
    pub table: &'a str,
    pub sql: &'a str,
    pub label: &'a str,
}

/// Lock, check-table, execute no-param SQL, accumulate rows into T.
/// Returns `init` when the table doesn't exist.
pub(super) fn query_rows_fold<V, T, RM, FA>(
    db: &DuckDb,
    spec: &QuerySpec<'_>,
    row_mapper: RM,
    init: T,
    mut fold_fn: FA,
) -> anyhow::Result<T>
where
    RM: Fn(&duckdb::Row<'_>) -> Result<V, duckdb::Error>,
    FA: FnMut(&mut T, V),
{
    use anyhow::Context;
    let label = spec.label;
    let conn = db
        .lock()
        .with_context(|| format!("acquiring db lock for {label}"))?;
    if !table_exists(&conn, spec.table)? {
        return Ok(init);
    }
    let mut stmt = conn
        .prepare(spec.sql)
        .with_context(|| format!("preparing {label}"))?;
    let rows = stmt
        .query_map([], |row| row_mapper(row))
        .with_context(|| format!("querying {label}"))?;
    let mut acc = init;
    for row in rows {
        let v = row.with_context(|| format!("reading {label} row"))?;
        fold_fn(&mut acc, v);
    }
    Ok(acc)
}

/// Shared scaffolding: lock db, check table exists, run a scalar aggregate query.
/// Returns `Ok(0)` if the table doesn't exist.
pub(super) fn query_project_scalar(
    db: &DuckDb,
    table: &str,
    sql: &str,
    label: &str,
) -> anyhow::Result<i64> {
    use anyhow::Context;

    let conn = db
        .lock()
        .with_context(|| format!("acquiring db lock for {label}"))?;

    if !table_exists(&conn, table)? {
        return Ok(0);
    }

    conn.query_row(sql, [], |row: &duckdb::Row| row.get(0))
        .context(label.to_string())
}

/// Result of preparing per-crate query scaffolding.
/// `Ready` carries the lock, a `(?),...,(?)` placeholder clause, and the paths to bind.
pub(super) enum PerCrateSetup<'a> {
    Empty,
    NoTable,
    Ready(
        std::sync::MutexGuard<'a, duckdb::Connection>,
        String,
        Vec<String>,
    ),
}

/// Shared scaffolding: validate paths, lock db, check table exists, build VALUES CTE.
pub(super) fn prepare_per_crate<'a>(
    db: &'a DuckDb,
    table: &str,
    member_paths: &[&str],
    label: &str,
) -> anyhow::Result<PerCrateSetup<'a>> {
    use anyhow::Context;

    if member_paths.is_empty() {
        return Ok(PerCrateSetup::Empty);
    }

    for path in member_paths {
        validate_path_chars(path)?;
    }

    let conn = db
        .lock()
        .with_context(|| format!("acquiring db lock for {label}"))?;

    if !table_exists(&conn, table)? {
        return Ok(PerCrateSetup::NoTable);
    }

    let placeholders = member_paths
        .iter()
        .map(|_| "(?)")
        .collect::<Vec<_>>()
        .join(", ");
    let paths: Vec<String> = member_paths.iter().map(|p| p.to_string()).collect();

    Ok(PerCrateSetup::Ready(conn, placeholders, paths))
}

/// Execute a per-crate SQL with bound path params and collect rows via a row-mapper.
pub(super) fn collect_per_crate_map<T, F>(
    conn: &duckdb::Connection,
    sql: &str,
    label: &str,
    params: &[String],
    row_mapper: F,
) -> anyhow::Result<HashMap<String, T>>
where
    F: Fn(&duckdb::Row<'_>) -> Result<(String, T), duckdb::Error>,
{
    use anyhow::Context;
    let mut stmt = conn
        .prepare(sql)
        .with_context(|| format!("preparing {label}"))?;
    let rows = stmt
        .query_map(duckdb::params_from_iter(params.iter()), |row| {
            row_mapper(row)
        })
        .with_context(|| format!("querying {label}"))?;
    let mut result = HashMap::new();
    for row in rows {
        let (path, val) = row.with_context(|| format!("reading {label} row"))?;
        result.insert(path, val);
    }
    Ok(result)
}

/// Parameters for a per-crate i64 query.
pub(super) struct PerCrateI64Query<'a> {
    pub db: &'a DuckDb,
    pub table: &'a str,
    pub member_paths: &'a [&'a str],
    pub select_expr: &'a str,
    pub join_alias: &'a str,
    pub join_column: &'a str,
    pub label: &'a str,
}

/// Shared scaffolding: validate paths, lock db, check table exists, build VALUES CTE,
/// LEFT JOIN on `starts_with`, GROUP BY, collect into HashMap<String, i64>.
/// Returns zeroed map if table doesn't exist, empty map if no member_paths.
pub(super) fn query_per_crate_i64(
    q: &PerCrateI64Query<'_>,
) -> anyhow::Result<HashMap<String, i64>> {
    let (conn, placeholders, paths) =
        match prepare_per_crate(q.db, q.table, q.member_paths, q.label)? {
            PerCrateSetup::Empty => return Ok(HashMap::new()),
            PerCrateSetup::NoTable => {
                return Ok(q.member_paths.iter().map(|p| (p.to_string(), 0)).collect())
            }
            PerCrateSetup::Ready(conn, placeholders, paths) => (conn, placeholders, paths),
        };

    let (table, select_expr, join_alias, join_column, label) =
        (q.table, q.select_expr, q.join_alias, q.join_column, q.label);

    let sql = format!(
        "WITH members(path) AS (VALUES {placeholders}) \
         SELECT m.path, {select_expr} \
         FROM members m \
         LEFT JOIN {table} {join_alias} ON starts_with({join_alias}.{join_column}, m.path || '/') \
         GROUP BY m.path",
    );

    collect_per_crate_map(&conn, &sql, label, &paths, |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })
}
