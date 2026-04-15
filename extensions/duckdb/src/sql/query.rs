//! Query functions for extracting project and crate-level metrics from DuckDB.

use crate::DuckDb;
use std::collections::HashMap;

use super::ingest::table_exists;
use super::validation::{escape_sql_string, validate_path_chars};

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

/// Shared scaffolding: lock db, check table exists, run a scalar aggregate query.
/// Returns `Ok(0)` if the table doesn't exist.
fn query_project_scalar(db: &DuckDb, table: &str, sql: &str, label: &str) -> anyhow::Result<i64> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context(format!("acquiring db lock for {label}"))?;

    if !table_exists(&conn, table)? {
        return Ok(0);
    }

    conn.query_row(sql, [], |row: &duckdb::Row| row.get(0))
        .context(label.to_string())
}

/// Result of preparing per-crate query scaffolding.
/// `Ready` means the table exists and the VALUES clause is built.
enum PerCrateSetup<'a> {
    Empty,
    NoTable,
    Ready(std::sync::MutexGuard<'a, duckdb::Connection>, String),
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
        .context(format!("acquiring db lock for {label}"))?;

    if !table_exists(&conn, table)? {
        return Ok(PerCrateSetup::NoTable);
    }

    let values: Vec<String> = member_paths
        .iter()
        .map(|p| format!("('{}')", escape_sql_string(p)))
        .collect();

    Ok(PerCrateSetup::Ready(conn, values.join(", ")))
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
    use anyhow::Context;

    let (conn, values) = match prepare_per_crate(q.db, q.table, q.member_paths, q.label)? {
        PerCrateSetup::Empty => return Ok(HashMap::new()),
        PerCrateSetup::NoTable => {
            return Ok(q.member_paths.iter().map(|p| (p.to_string(), 0)).collect())
        }
        PerCrateSetup::Ready(conn, values) => (conn, values),
    };

    let table = q.table;
    let select_expr = q.select_expr;
    let join_alias = q.join_alias;
    let join_column = q.join_column;
    let label = q.label;

    let sql = format!(
        "WITH members(path) AS (VALUES {values}) \
         SELECT m.path, {select_expr} \
         FROM members m \
         LEFT JOIN {table} {join_alias} ON starts_with({join_alias}.{join_column}, m.path || '/') \
         GROUP BY m.path",
    );

    let mut stmt = conn.prepare(&sql).context(format!("preparing {label}"))?;
    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .context(format!("querying {label}"))?;

    let mut result = HashMap::new();
    for row in rows {
        let (path, val) = row.context(format!("reading {label} row"))?;
        result.insert(path, val);
    }
    Ok(result)
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

/// Query distinct languages from `tokei_files` with LOC percentage, ordered by total LOC descending.
///
/// Returns formatted strings like `"Rust 85.2%"`. Languages under 0.1% are omitted.
pub fn query_project_languages(db: &DuckDb) -> anyhow::Result<Vec<String>> {
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
                    ROUND(SUM(code) * 100.0 / NULLIF((SELECT SUM(code) FROM tokei_files), 0), 1) AS pct \
             FROM tokei_files \
             GROUP BY language \
             ORDER BY SUM(code) DESC",
        )
        .context("preparing query_project_languages")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            let lang: String = row.get(0)?;
            let pct: f64 = row.get(1)?;
            Ok((lang, pct))
        })
        .context("querying project languages")?;

    let mut languages = Vec::new();
    for row in rows {
        let (lang, pct) = row.context("reading language row")?;
        if pct >= 0.1 {
            languages.push(format!("{lang} {pct:.1}%"));
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

    conn.query_row(
        "SELECT COALESCE(SUM(lines_count), 0), \
                COALESCE(SUM(lines_covered), 0), \
                CASE WHEN SUM(lines_count) > 0 \
                    THEN ROUND(SUM(lines_covered) * 100.0 / SUM(lines_count), 2) \
                    ELSE 0.0 END \
         FROM coverage_files",
        [],
        |row: &duckdb::Row| {
            Ok(CrateCoverage {
                lines_count: row.get(0)?,
                lines_covered: row.get(1)?,
                lines_percent: row.get(2)?,
            })
        },
    )
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
    use anyhow::Context;

    validate_path_chars(workspace_root)?;

    let label = "query_crate_coverage";
    let (conn, values) = match prepare_per_crate(db, "coverage_files", member_paths, label)? {
        PerCrateSetup::Empty => return Ok(HashMap::new()),
        PerCrateSetup::NoTable => {
            return Ok(member_paths
                .iter()
                .map(|p| (p.to_string(), CrateCoverage::zero()))
                .collect())
        }
        PerCrateSetup::Ready(conn, values) => (conn, values),
    };

    let escaped_root = escape_sql_string(workspace_root);
    let sql = format!(
        "WITH members(path) AS (VALUES {values}) \
         SELECT m.path, \
                COALESCE(SUM(c.lines_count), 0), \
                COALESCE(SUM(c.lines_covered), 0), \
                CASE WHEN SUM(c.lines_count) > 0 \
                    THEN ROUND(SUM(c.lines_covered) * 100.0 / SUM(c.lines_count), 2) \
                    ELSE 0.0 END \
         FROM members m \
         LEFT JOIN coverage_files c \
             ON starts_with(c.filename, m.path || '/') \
             OR starts_with(c.filename, '{escaped_root}' || '/' || m.path || '/') \
         GROUP BY m.path",
    );

    let mut stmt = conn.prepare(&sql).context(format!("preparing {label}"))?;
    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok((
                row.get::<_, String>(0)?,
                CrateCoverage {
                    lines_count: row.get(1)?,
                    lines_covered: row.get(2)?,
                    lines_percent: row.get(3)?,
                },
            ))
        })
        .context(format!("querying {label}"))?;

    let mut result = HashMap::new();
    for row in rows {
        let (path, cov) = row.context(format!("reading {label} row"))?;
        result.insert(path, cov);
    }
    Ok(result)
}

/// Query per-crate external dependencies (name + version_req) from `crate_dependencies` view.
///
/// Returns a map of crate_name -> Vec<(dep_name, version_req)>, sorted by dep name.
/// Returns an empty map if the view doesn't exist (graceful degradation).
pub fn query_crate_deps(db: &DuckDb) -> anyhow::Result<HashMap<String, Vec<(String, String)>>> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_crate_deps")?;

    if !table_exists(&conn, "crate_dependencies")? {
        return Ok(HashMap::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT crate_name, dependency_name, version_req \
             FROM crate_dependencies \
             WHERE dependency_kind = 'normal' \
             ORDER BY crate_name, dependency_name",
        )
        .context("preparing query_crate_deps")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .context("querying crate deps")?;

    let mut result: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for row in rows {
        let (crate_name, dep_name, version_req) = row.context("reading crate dep row")?;
        result
            .entry(crate_name)
            .or_default()
            .push((dep_name, version_req));
    }
    Ok(result)
}

/// Query per-crate external dependency counts from `crate_dependencies` view.
///
/// Returns a map of package name -> normal dependency count.
/// Returns an empty map if the view doesn't exist (graceful degradation).
pub fn query_crate_dep_counts(db: &DuckDb) -> anyhow::Result<HashMap<String, i64>> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_crate_dep_counts")?;

    if !table_exists(&conn, "crate_dependencies")? {
        return Ok(HashMap::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT crate_name, COUNT(*) AS dep_count \
             FROM crate_dependencies \
             WHERE dependency_kind = 'normal' \
             GROUP BY crate_name",
        )
        .context("preparing query_crate_dep_counts")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .context("querying crate dep counts")?;

    let mut result = HashMap::new();
    for row in rows {
        let (name, count) = row.context("reading crate dep count row")?;
        result.insert(name, count);
    }
    Ok(result)
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
        assert!(langs[0].starts_with("Rust"));
        assert!(langs[1].starts_with("TOML"));
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
        assert!(langs[0].starts_with("Rust"));
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
