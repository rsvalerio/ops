//! Query functions for extracting project and crate-level metrics from DuckDB.

use crate::DuckDb;
use std::collections::HashMap;

use ops_core::project_identity::LanguageStat;

use super::ingest::table_exists;
use super::validation::validate_path_chars;

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
fn coverage_col_select(prefix: &str) -> String {
    format!(
        "COALESCE(SUM({prefix}lines_count), 0), \
         COALESCE(SUM({prefix}lines_covered), 0), \
         CASE WHEN SUM({prefix}lines_count) > 0 \
             THEN ROUND(SUM({prefix}lines_covered) * 100.0 / SUM({prefix}lines_count), 2) \
             ELSE 0.0 END"
    )
}

/// Query spec bundling table name, SQL, and diagnostic label for `query_rows_fold`.
struct QuerySpec<'a> {
    table: &'a str,
    sql: &'a str,
    label: &'a str,
}

/// Lock, check-table, execute no-param SQL, accumulate rows into T.
/// Returns `init` when the table doesn't exist.
fn query_rows_fold<V, T, RM, FA>(
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
fn query_project_scalar(db: &DuckDb, table: &str, sql: &str, label: &str) -> anyhow::Result<i64> {
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
enum PerCrateSetup<'a> {
    Empty,
    NoTable,
    Ready(
        std::sync::MutexGuard<'a, duckdb::Connection>,
        String,
        Vec<String>,
    ),
}

/// Shared scaffolding: validate paths, lock db, check table exists, build VALUES CTE.
fn prepare_per_crate<'a>(
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
fn collect_per_crate_map<T, F>(
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
struct PerCrateI64Query<'a> {
    db: &'a DuckDb,
    table: &'a str,
    member_paths: &'a [&'a str],
    select_expr: &'a str,
    join_alias: &'a str,
    join_column: &'a str,
    label: &'a str,
}

/// Shared scaffolding: validate paths, lock db, check table exists, build VALUES CTE,
/// LEFT JOIN on `starts_with`, GROUP BY, collect into HashMap<String, i64>.
/// Returns zeroed map if table doesn't exist, empty map if no member_paths.
fn query_per_crate_i64(q: &PerCrateI64Query<'_>) -> anyhow::Result<HashMap<String, i64>> {
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

/// Query total file count across the whole project from `tokei_files`.
pub fn query_project_file_count(db: &DuckDb) -> anyhow::Result<i64> {
    query_project_scalar(
        db,
        "tokei_files",
        "SELECT COUNT(*) FROM tokei_files",
        "query_project_file_count",
    )
}

/// Query per-crate file counts from `tokei_files`.
///
/// Returns a map of member path -> file count. Members with no matching
/// files get 0.
pub fn query_crate_file_count(
    db: &DuckDb,
    member_paths: &[&str],
) -> anyhow::Result<HashMap<String, i64>> {
    query_per_crate_i64(&PerCrateI64Query {
        db,
        table: "tokei_files",
        member_paths,
        select_expr: "COUNT(f.file)",
        join_alias: "f",
        join_column: "file",
        label: "query_crate_file_count",
    })
}

/// Query total lines of code across the whole project from `tokei_files`.
pub fn query_project_loc(db: &DuckDb) -> anyhow::Result<i64> {
    query_project_scalar(
        db,
        "tokei_files",
        "SELECT COALESCE(SUM(code), 0) FROM tokei_files",
        "query_project_loc",
    )
}

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

/// Query per-language breakdown from `tokei_files`: LOC, file count, and
/// percentages of both. Ordered by LOC descending. Languages contributing
/// under 0.1% of total LOC are omitted.
pub fn query_project_languages(db: &DuckDb) -> anyhow::Result<Vec<LanguageStat>> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_project_languages")?;

    if !table_exists(&conn, "tokei_files")? {
        return Ok(vec![]);
    }

    let mut stmt = conn
        .prepare(
            "SELECT language, \
                    SUM(code) AS loc, \
                    COUNT(*) AS files, \
                    ROUND(SUM(code) * 100.0 / NULLIF((SELECT SUM(code) FROM tokei_files), 0), 1) AS loc_pct, \
                    ROUND(COUNT(*) * 100.0 / NULLIF((SELECT COUNT(*) FROM tokei_files), 0), 1) AS files_pct \
             FROM tokei_files \
             GROUP BY language \
             ORDER BY SUM(code) DESC",
        )
        .context("preparing query_project_languages")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok(LanguageStat {
                name: row.get(0)?,
                loc: row.get(1)?,
                files: row.get(2)?,
                loc_pct: row.get(3)?,
                files_pct: row.get(4)?,
            })
        })
        .context("querying project languages")?;

    let mut languages = Vec::new();
    for row in rows {
        let stat = row.context("reading language row")?;
        if stat.loc_pct >= 0.1 {
            languages.push(stat);
        }
    }
    Ok(languages)
}

/// Query per-crate lines of code from `tokei_files`.
///
/// Returns a map of member path -> total code lines. Members with no matching
/// files get 0.
pub fn query_crate_loc(db: &DuckDb, member_paths: &[&str]) -> anyhow::Result<HashMap<String, i64>> {
    query_per_crate_i64(&PerCrateI64Query {
        db,
        table: "tokei_files",
        member_paths,
        select_expr: "COALESCE(SUM(f.code), 0)",
        join_alias: "f",
        join_column: "file",
        label: "query_crate_loc",
    })
}

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
    use super::*;
    use crate::init_schema;

    #[test]
    fn query_project_file_count_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);
             INSERT INTO tokei_files VALUES ('Rust', 'src/main.rs', 500, 50, 20, 570);
             INSERT INTO tokei_files VALUES ('Rust', 'src/lib.rs', 300, 30, 10, 340);
             INSERT INTO tokei_files VALUES ('TOML', 'Cargo.toml', 40, 5, 3, 48);",
        )
        .expect("insert test data");
        drop(conn);

        let count = query_project_file_count(&db).expect("query should work");
        assert_eq!(count, 3);
    }

    #[test]
    fn query_project_file_count_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let count = query_project_file_count(&db).expect("query should work");
        assert_eq!(count, 0);
    }

    #[test]
    fn query_crate_file_count_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-lib/src/lib.rs', 3000, 200, 100, 3300);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-lib/src/utils.rs', 1231, 50, 30, 1311);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-cli/src/main.rs', 1892, 100, 50, 2042);",
        )
        .expect("insert test data");
        drop(conn);

        let result = query_crate_file_count(&db, &["crates/my-lib", "crates/my-cli"])
            .expect("query should work");
        assert_eq!(result.len(), 2);
        assert_eq!(result["crates/my-lib"], 2);
        assert_eq!(result["crates/my-cli"], 1);
    }

    #[test]
    fn query_crate_file_count_empty() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let result = query_crate_file_count(&db, &["crates/my-lib"]).expect("query should work");
        assert_eq!(result["crates/my-lib"], 0);
    }

    #[test]
    fn query_project_loc_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);
             INSERT INTO tokei_files VALUES ('Rust', 'src/main.rs', 500, 50, 20, 570);
             INSERT INTO tokei_files VALUES ('Rust', 'src/lib.rs', 300, 30, 10, 340);
             INSERT INTO tokei_files VALUES ('TOML', 'Cargo.toml', 40, 5, 3, 48);",
        )
        .expect("insert test data");
        drop(conn);

        let loc = query_project_loc(&db).expect("query should work");
        assert_eq!(loc, 840);
    }

    #[test]
    fn query_project_loc_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let loc = query_project_loc(&db).expect("query should work");
        assert_eq!(loc, 0);
    }

    #[test]
    fn query_crate_loc_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-lib/src/lib.rs', 3000, 200, 100, 3300);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-lib/src/utils.rs', 1231, 50, 30, 1311);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-cli/src/main.rs', 1892, 100, 50, 2042);",
        )
        .expect("insert test data");
        drop(conn);

        let result =
            query_crate_loc(&db, &["crates/my-lib", "crates/my-cli"]).expect("query should work");
        assert_eq!(result.len(), 2);
        assert_eq!(result["crates/my-lib"], 4231);
        assert_eq!(result["crates/my-cli"], 1892);
    }

    #[test]
    fn query_crate_loc_empty_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);",
        )
        .expect("create empty table");
        drop(conn);

        let result =
            query_crate_loc(&db, &["crates/my-lib", "crates/my-cli"]).expect("query should work");
        assert_eq!(result["crates/my-lib"], 0);
        assert_eq!(result["crates/my-cli"], 0);
    }

    #[test]
    fn query_crate_loc_no_members() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let result = query_crate_loc(&db, &[]).expect("query should work");
        assert!(result.is_empty());
    }

    #[test]
    fn query_crate_deps_no_view() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        let result = query_crate_deps(&db).expect("query should work");
        assert!(result.is_empty());
    }

    #[test]
    fn query_crate_deps_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE VIEW crate_dependencies AS \
             SELECT * FROM (VALUES \
                 ('ops-core', 'anyhow', '^1.0', 'normal', false), \
                 ('ops-core', 'serde', '^1.0', 'normal', false), \
                 ('ops-core', 'tempfile', '^3.0', 'dev', false), \
                 ('ops-cli', 'clap', '^4.0', 'normal', false), \
                 ('ops-cli', 'tokio', '^1.0', 'normal', false) \
             ) AS t(crate_name, dependency_name, version_req, dependency_kind, is_optional)",
        )
        .expect("create view with test data");
        drop(conn);

        let result = query_crate_deps(&db).expect("query should work");
        assert_eq!(result.len(), 2);

        let core_deps = &result["ops-core"];
        assert_eq!(core_deps.len(), 2); // only normal deps
        assert_eq!(core_deps[0], ("anyhow".to_string(), "^1.0".to_string()));
        assert_eq!(core_deps[1], ("serde".to_string(), "^1.0".to_string()));

        let cli_deps = &result["ops-cli"];
        assert_eq!(cli_deps.len(), 2);
        assert_eq!(cli_deps[0], ("clap".to_string(), "^4.0".to_string()));
        assert_eq!(cli_deps[1], ("tokio".to_string(), "^1.0".to_string()));
    }

    #[test]
    fn query_crate_dep_counts_no_view() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        let result = query_crate_dep_counts(&db).expect("query should work");
        assert!(result.is_empty());
    }

    #[test]
    fn query_crate_dep_counts_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE VIEW crate_dependencies AS \
             SELECT * FROM (VALUES \
                 ('ops-core', 'serde', '^1.0', 'normal', false), \
                 ('ops-core', 'anyhow', '^1.0', 'normal', false), \
                 ('ops-core', 'tempfile', '^3.0', 'dev', false), \
                 ('ops-cli', 'clap', '^4.0', 'normal', false) \
             ) AS t(crate_name, dependency_name, version_req, dependency_kind, is_optional)",
        )
        .expect("create view with test data");
        drop(conn);

        let result = query_crate_dep_counts(&db).expect("query should work");
        assert_eq!(result.len(), 2);
        assert_eq!(result["ops-core"], 2); // only normal deps
        assert_eq!(result["ops-cli"], 1);
    }

    #[test]
    fn query_project_coverage_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let cov = query_project_coverage(&db).expect("query should work");
        assert_eq!(cov.lines_count, 0);
        assert_eq!(cov.lines_covered, 0);
        assert!((cov.lines_percent - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn query_project_coverage_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE coverage_files (filename VARCHAR, lines_count BIGINT, \
             lines_covered BIGINT, lines_percent DOUBLE);
             INSERT INTO coverage_files VALUES ('crates/core/src/lib.rs', 100, 80, 80.0);
             INSERT INTO coverage_files VALUES ('crates/cli/src/main.rs', 200, 150, 75.0);",
        )
        .expect("insert test data");
        drop(conn);

        let cov = query_project_coverage(&db).expect("query should work");
        assert_eq!(cov.lines_count, 300);
        assert_eq!(cov.lines_covered, 230);
        // 230/300 * 100 = 76.67
        assert!((cov.lines_percent - 76.67).abs() < 0.01);
    }

    #[test]
    fn query_crate_coverage_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let result =
            query_crate_coverage(&db, &["crates/core"], "/workspace").expect("query should work");
        assert_eq!(result["crates/core"].lines_count, 0);
    }

    #[test]
    fn query_crate_coverage_empty_members() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let result = query_crate_coverage(&db, &[], "/workspace").expect("query should work");
        assert!(result.is_empty());
    }

    #[test]
    fn query_crate_coverage_with_relative_paths() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE coverage_files (filename VARCHAR, lines_count BIGINT, \
             lines_covered BIGINT, lines_percent DOUBLE);
             INSERT INTO coverage_files VALUES ('crates/core/src/lib.rs', 100, 80, 80.0);
             INSERT INTO coverage_files VALUES ('crates/core/src/util.rs', 50, 40, 80.0);
             INSERT INTO coverage_files VALUES ('crates/cli/src/main.rs', 200, 150, 75.0);",
        )
        .expect("insert test data");
        drop(conn);

        let result = query_crate_coverage(&db, &["crates/core", "crates/cli"], "/workspace")
            .expect("query should work");

        assert_eq!(result.len(), 2);
        assert_eq!(result["crates/core"].lines_count, 150);
        assert_eq!(result["crates/core"].lines_covered, 120);
        assert_eq!(result["crates/cli"].lines_count, 200);
        assert_eq!(result["crates/cli"].lines_covered, 150);
    }

    #[test]
    fn query_dependency_count_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let count = query_dependency_count(&db).expect("query should work");
        assert_eq!(count, 0);
    }

    #[test]
    fn query_dependency_count_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE VIEW crate_dependencies AS \
             SELECT * FROM (VALUES \
                 ('ops-core', 'serde', '^1.0', 'normal', false), \
                 ('ops-core', 'anyhow', '^1.0', 'normal', false), \
                 ('ops-cli', 'serde', '^1.0', 'normal', false), \
                 ('ops-cli', 'clap', '^4.0', 'normal', false) \
             ) AS t(crate_name, dependency_name, version_req, dependency_kind, is_optional)",
        )
        .expect("create view with test data");
        drop(conn);

        let count = query_dependency_count(&db).expect("query should work");
        assert_eq!(count, 3); // serde, anyhow, clap (DISTINCT)
    }

    #[test]
    fn query_project_languages_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let langs = query_project_languages(&db).expect("query should work");
        assert!(langs.is_empty());
    }

    #[test]
    fn query_project_languages_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);
             INSERT INTO tokei_files VALUES ('Rust', 'src/main.rs', 850, 50, 20, 920);
             INSERT INTO tokei_files VALUES ('Rust', 'src/lib.rs', 100, 10, 5, 115);
             INSERT INTO tokei_files VALUES ('TOML', 'Cargo.toml', 50, 5, 3, 58);",
        )
        .expect("insert test data");
        drop(conn);

        let langs = query_project_languages(&db).expect("query should work");
        assert_eq!(langs.len(), 2);
        assert_eq!(langs[0].name, "Rust");
        assert_eq!(langs[0].loc, 950);
        assert_eq!(langs[0].files, 2);
        assert_eq!(langs[1].name, "TOML");
        assert_eq!(langs[1].loc, 50);
        assert_eq!(langs[1].files, 1);
    }

    #[test]
    fn query_project_languages_omits_tiny_percentages() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);
             INSERT INTO tokei_files VALUES ('Rust', 'src/main.rs', 100000, 0, 0, 100000);
             INSERT INTO tokei_files VALUES ('Markdown', 'README.md', 5, 0, 0, 5);",
        )
        .expect("insert test data");
        drop(conn);

        let langs = query_project_languages(&db).expect("query should work");
        // Markdown is ~0.005% which is < 0.1%, should be omitted
        assert_eq!(langs.len(), 1);
        assert_eq!(langs[0].name, "Rust");
    }

    #[test]
    fn query_crate_coverage_with_absolute_paths() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE coverage_files (filename VARCHAR, lines_count BIGINT, \
             lines_covered BIGINT, lines_percent DOUBLE);
             INSERT INTO coverage_files VALUES ('/workspace/crates/core/src/lib.rs', 100, 90, 90.0);
             INSERT INTO coverage_files VALUES ('/workspace/crates/cli/src/main.rs', 200, 100, 50.0);",
        )
        .expect("insert test data");
        drop(conn);

        let result = query_crate_coverage(&db, &["crates/core", "crates/cli"], "/workspace")
            .expect("query should work");

        assert_eq!(result.len(), 2);
        assert_eq!(result["crates/core"].lines_count, 100);
        assert_eq!(result["crates/core"].lines_covered, 90);
        assert_eq!(result["crates/cli"].lines_count, 200);
        assert_eq!(result["crates/cli"].lines_covered, 100);
    }
}
