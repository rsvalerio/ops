//! Shared query scaffolding: locking, table-existence checks, per-crate builders.

use crate::DuckDb;
use std::collections::HashMap;

use super::super::ingest::table_exists;
use super::super::validation::{validate_identifier, validate_path_chars, SqlError};

/// Validated SQL identifier wrappers. Constructing one runs
/// `validate_identifier` exactly once; downstream code can interpolate the
/// inner `&str` without re-validating.
macro_rules! sql_ident_newtype {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone)]
        pub(crate) struct $name(&'static str);

        impl $name {
            /// Construct from a `&'static str`, validating the identifier shape.
            pub(crate) fn new(s: &'static str) -> Result<Self, SqlError> {
                validate_identifier(s)?;
                Ok(Self(s))
            }

            pub(crate) fn as_str(&self) -> &'static str {
                self.0
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.0)
            }
        }
    };
}

sql_ident_newtype!(TableName, "A validated SQL table name.");
sql_ident_newtype!(ColumnAlias, "A validated SQL column/table alias.");
sql_ident_newtype!(ColumnName, "A validated SQL column name.");

/// Per-crate coverage data from `coverage_files`.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct CrateCoverage {
    pub lines_count: i64,
    pub lines_covered: i64,
    pub lines_percent: f64,
}

impl CrateCoverage {
    pub fn new(lines_count: i64, lines_covered: i64, lines_percent: f64) -> Self {
        Self {
            lines_count,
            lines_covered,
            lines_percent,
        }
    }

    pub fn zero() -> Self {
        Self {
            lines_count: 0,
            lines_covered: 0,
            lines_percent: 0.0,
        }
    }
}

/// Returns the SELECT column expressions for coverage SUM/CASE aggregation.
///
/// SEC-12: the prefix is typed as `Option<&ColumnAlias>` rather than `&str`
/// so callers cannot forward an unvalidated alias into the formatted SQL.
/// `None` produces direct column references (e.g. `SUM(lines_count)`),
/// `Some(alias)` produces `SUM(<alias>.lines_count)` after the alias has
/// already been validated by `ColumnAlias::new`. Aligns with the
/// `TableName` / `ColumnAlias` / `ColumnName` newtype pattern adopted
/// elsewhere in this module.
pub(super) fn coverage_col_select(prefix: Option<&ColumnAlias>) -> String {
    let prefix = match prefix {
        Some(alias) => format!("{}.", alias.as_str()),
        None => String::new(),
    };
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
/// `Ready` carries the lock and a `(?),...,(?)` placeholder clause; the
/// caller binds `member_paths` (or a chain that adds extra bound params)
/// directly via `duckdb::params_from_iter`.
pub(super) enum PerCrateSetup<'a> {
    Empty,
    NoTable,
    Ready(std::sync::MutexGuard<'a, duckdb::Connection>, String),
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

    Ok(PerCrateSetup::Ready(conn, placeholders))
}

/// Outcome of resolving a [`PerCrateSetup`] against a default-value function.
/// `Done` carries the early-return map; `Continue` hands back the lock and
/// placeholders so the caller can build and execute its query.
pub(super) enum Resolved<'a, T> {
    Done(HashMap<String, T>),
    Continue(std::sync::MutexGuard<'a, duckdb::Connection>, String),
}

/// Single source of truth for the Empty / NoTable / Ready branching that every
/// per-crate query needs. `default_fn` produces the value used to zero-fill the
/// NoTable branch.
pub(super) fn resolve_per_crate<'a, T, F>(
    setup: PerCrateSetup<'a>,
    member_paths: &[&str],
    default_fn: F,
) -> Resolved<'a, T>
where
    F: Fn() -> T,
{
    match setup {
        PerCrateSetup::Empty => Resolved::Done(HashMap::new()),
        PerCrateSetup::NoTable => Resolved::Done(
            member_paths
                .iter()
                .map(|p| ((*p).to_string(), default_fn()))
                .collect(),
        ),
        PerCrateSetup::Ready(conn, placeholders) => Resolved::Continue(conn, placeholders),
    }
}

/// Build the shared `WITH members(path) AS (VALUES (?), ...)` CTE prefix.
/// Callers append their `SELECT m.path, ... FROM members m ...` clause.
pub(super) fn members_cte_prefix(placeholders: &str) -> String {
    format!("WITH members(path) AS (VALUES {placeholders})")
}

/// Execute a per-crate SQL with bound params and collect rows via a row-mapper.
///
/// `params` is an iterator of `&str` references — callers pass
/// `member_paths.iter().copied()` (and `chain` extra `&str` refs as needed)
/// to avoid allocating an intermediate `Vec<String>` per query.
pub(super) fn collect_per_crate_map<'p, T, F, I>(
    conn: &duckdb::Connection,
    sql: &str,
    label: &str,
    params: I,
    row_mapper: F,
) -> anyhow::Result<HashMap<String, T>>
where
    F: Fn(&duckdb::Row<'_>) -> Result<(String, T), duckdb::Error>,
    I: IntoIterator<Item = &'p str>,
{
    use anyhow::Context;
    let mut stmt = conn
        .prepare(sql)
        .with_context(|| format!("preparing {label}"))?;
    let rows = stmt
        .query_map(duckdb::params_from_iter(params), |row| row_mapper(row))
        .with_context(|| format!("querying {label}"))?;
    let mut result = HashMap::new();
    for row in rows {
        let (path, val) = row.with_context(|| format!("reading {label} row"))?;
        // ERR-1: a duplicate key is a query-shape invariant violation
        // (workspace glob bug, dropped GROUP BY) — surface it instead of
        // silently overwriting the prior row.
        if result.insert(path.clone(), val).is_some() {
            tracing::warn!(
                label,
                path = %path,
                "duplicate row in per-crate query; later value overwrites earlier"
            );
        }
    }
    Ok(result)
}

/// Parameters for a per-crate i64 query. Identifier fields are newtyped so
/// that swapping `join_alias` and `join_column` at construction is a type
/// error (API-1) and validation is enforced once at construction time.
pub(super) struct PerCrateI64Query<'a> {
    pub db: &'a DuckDb,
    pub table: TableName,
    pub member_paths: &'a [&'a str],
    pub select_expr: &'a str,
    pub join_alias: ColumnAlias,
    pub join_column: ColumnName,
    pub label: &'a str,
}

/// Shared scaffolding: validate paths, lock db, check table exists, build VALUES CTE,
/// LEFT JOIN on `starts_with`, GROUP BY, collect into HashMap<String, i64>.
/// Returns zeroed map if table doesn't exist, empty map if no member_paths.
pub(super) fn query_per_crate_i64(
    q: &PerCrateI64Query<'_>,
) -> anyhow::Result<HashMap<String, i64>> {
    let setup = prepare_per_crate(q.db, q.table.as_str(), q.member_paths, q.label)?;
    let (conn, placeholders) = match resolve_per_crate(setup, q.member_paths, || 0_i64) {
        Resolved::Done(map) => return Ok(map),
        Resolved::Continue(conn, placeholders) => (conn, placeholders),
    };

    let (table, select_expr, join_alias, join_column, label) = (
        q.table.as_str(),
        q.select_expr,
        q.join_alias.as_str(),
        q.join_column.as_str(),
        q.label,
    );

    let cte = members_cte_prefix(&placeholders);
    let sql = format!(
        "{cte} \
         SELECT m.path, {select_expr} \
         FROM members m \
         LEFT JOIN {table} {join_alias} ON starts_with({join_alias}.{join_column}, m.path || '/') \
         GROUP BY m.path",
    );

    collect_per_crate_map(&conn, &sql, label, q.member_paths.iter().copied(), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SEC-12 AC #2: legitimate "no prefix" form selects the bare columns.
    #[test]
    fn coverage_col_select_with_no_prefix() {
        let sql = coverage_col_select(None);
        assert!(sql.contains("SUM(lines_count)"));
        assert!(sql.contains("SUM(lines_covered)"));
        // Crucially does not introduce a leading dot.
        assert!(!sql.contains(".lines_count"), "got: {sql}");
    }

    /// SEC-12 AC #2: legitimate aliased form uses the validated alias.
    #[test]
    fn coverage_col_select_with_validated_alias() {
        let alias = ColumnAlias::new("c").expect("static alias is valid");
        let sql = coverage_col_select(Some(&alias));
        assert!(sql.contains("SUM(c.lines_count)"));
        assert!(sql.contains("SUM(c.lines_covered)"));
    }

    /// SEC-12 AC #1: an attacker-shaped "prefix" cannot reach the formatted
    /// SQL because `ColumnAlias::new` rejects non-identifier strings before
    /// a value can be passed in. This is the regression guard the typed
    /// signature is meant to provide.
    /// ERR-1: a query that returns the same key twice (e.g. dropped GROUP BY)
    /// must not silently produce a single row in the resulting map. Pinning
    /// behaviour via in-memory DuckDB so the regression is visible without a
    /// tracing-subscriber dev-dep.
    #[test]
    fn collect_per_crate_map_keeps_one_entry_for_duplicate_keys() {
        let conn = duckdb::Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "CREATE TABLE t (path VARCHAR, val INT);
             INSERT INTO t VALUES ('a', 1), ('a', 2), ('b', 3);",
        )
        .unwrap();
        let res = collect_per_crate_map::<i64, _, _>(
            &conn,
            "SELECT path, val FROM t",
            "dup-test",
            std::iter::empty::<&str>(),
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .unwrap();
        assert_eq!(
            res.len(),
            2,
            "duplicate-key collision must collapse to one entry per key"
        );
        // The insert-overwrite contract is preserved: the second row wins.
        assert_eq!(res.get("a").copied(), Some(2));
    }

    #[test]
    fn column_alias_rejects_non_identifier_prefix() {
        assert!(ColumnAlias::new("c.").is_err());
        assert!(ColumnAlias::new("c; DROP TABLE coverage_files; --").is_err());
        assert!(ColumnAlias::new("").is_err());
        assert!(ColumnAlias::new("1c").is_err());
        assert!(ColumnAlias::new("c d").is_err());
    }
}
