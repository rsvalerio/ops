use super::*;
use crate::init_schema;
use crate::DuckDb;

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

    // ERR-2 / TASK-1253: dep counts now key by `crate_manifest_path`, so
    // the synthetic view also surfaces that column.
    let conn = db.lock().expect("lock");
    conn.execute_batch(
        "CREATE VIEW crate_dependencies AS \
         SELECT * FROM (VALUES \
             ('ops-core', 'serde', '^1.0', 'normal', false, '/ws/core/Cargo.toml'), \
             ('ops-core', 'anyhow', '^1.0', 'normal', false, '/ws/core/Cargo.toml'), \
             ('ops-core', 'tempfile', '^3.0', 'dev', false, '/ws/core/Cargo.toml'), \
             ('ops-cli', 'clap', '^4.0', 'normal', false, '/ws/cli/Cargo.toml') \
         ) AS t(crate_name, dependency_name, version_req, dependency_kind, is_optional, crate_manifest_path)",
    )
    .expect("create view with test data");
    drop(conn);

    let result = query_crate_dep_counts(&db).expect("query should work");
    assert_eq!(result.len(), 2);
    assert_eq!(result["/ws/core/Cargo.toml"], 2); // only normal deps
    assert_eq!(result["/ws/cli/Cargo.toml"], 1);
}

/// ERR-2 / TASK-1253: when two workspace members share the same
/// `crate_name` (legal in cargo for renamed packages), keying by manifest
/// path keeps both rows distinct rather than collapsing into a single map
/// entry that silently mis-attributes counts.
#[test]
fn query_crate_dep_counts_distinguishes_duplicate_named_members() {
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init_schema");

    let conn = db.lock().expect("lock");
    conn.execute_batch(
        "CREATE VIEW crate_dependencies AS \
         SELECT * FROM (VALUES \
             ('lib', 'serde', '^1.0', 'normal', false, '/ws/a/lib/Cargo.toml'), \
             ('lib', 'serde', '^1.0', 'normal', false, '/ws/b/lib/Cargo.toml'), \
             ('lib', 'anyhow', '^1.0', 'normal', false, '/ws/b/lib/Cargo.toml') \
         ) AS t(crate_name, dependency_name, version_req, dependency_kind, is_optional, crate_manifest_path)",
    )
    .expect("create view with test data");
    drop(conn);

    let result = query_crate_dep_counts(&db).expect("query should work");
    assert_eq!(result.len(), 2, "duplicate-named members must not collide");
    assert_eq!(result["/ws/a/lib/Cargo.toml"], 1);
    assert_eq!(result["/ws/b/lib/Cargo.toml"], 2);
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
fn query_project_languages_returns_empty_when_all_below_threshold() {
    // READ-5 / TASK-0362: when every language is below the 0.1% threshold
    // the function must honour its documented "omit < 0.1%" contract and
    // return an empty Vec. Previously a fallback returned the top entry,
    // hiding "all tiny" behind "single dominant language".
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init_schema");

    let conn = db.lock().expect("lock");
    conn.execute_batch(
        "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
         comments BIGINT, blanks BIGINT, lines BIGINT);",
    )
    .expect("create");
    // 5000 unique languages, each contributing 1 line → each ~0.02% < 0.1%.
    conn.execute_batch(
        "INSERT INTO tokei_files \
         SELECT 'Lang' || i, 'f' || i, 1, 0, 0, 1 \
         FROM generate_series(0, 4999) AS gs(i);",
    )
    .expect("bulk insert");
    drop(conn);

    let langs = query_project_languages(&db).expect("query");
    assert!(
        langs.is_empty(),
        "all-below-threshold input must return empty (got {} entries)",
        langs.len()
    );
}

#[test]
fn query_project_languages_returns_empty_when_total_loc_is_zero() {
    // ERR-1 / TASK-1116: when tokei_files exists but every code value is 0,
    // total_loc is 0 and the SQL division returns NULL via NULLIF. The row
    // mapper used to error on row.get::<_, f64>(3)? for the NULL loc_pct,
    // surfacing as a misleading "language stats failed" log. The fix wraps
    // loc_pct in COALESCE(..., 0) so the >= 0.1 filter naturally drops the
    // row and the documented empty-result signal is preserved.
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init_schema");

    let conn = db.lock().expect("lock");
    conn.execute_batch(
        "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
         comments BIGINT, blanks BIGINT, lines BIGINT);
         INSERT INTO tokei_files VALUES ('Markdown', 'README.md', 0, 0, 10, 10);
         INSERT INTO tokei_files VALUES ('Markdown', 'CHANGELOG.md', 0, 5, 0, 5);
         INSERT INTO tokei_files VALUES ('Plain Text', 'NOTES.txt', 0, 0, 3, 3);",
    )
    .expect("insert test data");
    drop(conn);

    let langs = query_project_languages(&db).expect("query must not error on zero total_loc");
    assert!(
        langs.is_empty(),
        "all-zero-code input must return empty (got {} entries)",
        langs.len()
    );
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

/// Create a `rust_loc_files` table shaped like the one the `rust-loc`
/// ingestor loads, so the summary query runs against realistic column
/// types. The view on top is created by each test that needs one.
fn rust_loc_files_fixture(db: &DuckDb) {
    let conn = db.lock().expect("lock");
    conn.execute_batch(
        "CREATE TABLE rust_loc_files (file VARCHAR, region VARCHAR, code BIGINT, \
         docs BIGINT, comments BIGINT, blanks BIGINT, lines BIGINT);
         INSERT INTO rust_loc_files VALUES ('src/lib.rs', 'main', 300, 40, 10, 50, 400);
         INSERT INTO rust_loc_files VALUES ('src/util.rs', 'main', 200, 10, 5, 25, 240);
         INSERT INTO rust_loc_files VALUES ('src/lib.rs', 'test', 120, 0, 4, 16, 140);
         INSERT INTO rust_loc_files VALUES ('examples/demo.rs', 'example', 30, 2, 1, 7, 40);",
    )
    .expect("insert test data");
}

#[test]
fn query_rust_loc_summary_no_view() {
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init_schema");

    let stats = query_rust_loc_summary(&db).expect("missing view must not error");
    assert!(
        stats.is_empty(),
        "missing view must read as no data: {stats:?}"
    );
}

/// The aggregate columns come back as DuckDB `SUM(BIGINT)` (a 128-bit
/// HUGEINT), so this pins that they still decode into the `i64` fields of
/// [`RustLocStat`] — a silent type mismatch here would surface as an
/// unreadable "invalid column type" at the about page instead.
#[test]
fn query_rust_loc_summary_aggregates_per_region() {
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init_schema");
    rust_loc_files_fixture(&db);

    let conn = db.lock().expect("lock");
    conn.execute_batch(
        "CREATE VIEW rust_loc_summary AS \
         SELECT region, COUNT(*) AS files, SUM(code) AS code, SUM(docs) AS docs, \
         SUM(comments) AS comments, SUM(blanks) AS blanks, SUM(lines) AS lines \
         FROM rust_loc_files GROUP BY region ORDER BY code DESC",
    )
    .expect("create view");
    drop(conn);

    let stats = query_rust_loc_summary(&db).expect("query should work");
    assert_eq!(stats.len(), 3, "one row per region: {stats:?}");

    let main = stats
        .iter()
        .find(|s| s.region == "main")
        .expect("main region present");
    assert_eq!(main.files, 2);
    assert_eq!(main.code, 500);
    assert_eq!(main.docs, 50);
    assert_eq!(main.comments, 15);
    assert_eq!(main.blanks, 75);
    assert_eq!(main.lines, 640);

    let test = stats
        .iter()
        .find(|s| s.region == "test")
        .expect("test region present");
    assert_eq!(test.code, 120);
    assert_eq!(test.files, 1);

    let example = stats
        .iter()
        .find(|s| s.region == "example")
        .expect("example region present");
    assert_eq!(example.code, 30);
}

/// A `rust_loc_files` table with no rows produces an empty summary rather
/// than a row of zeros, so the renderer's "no data" branch is reachable
/// even after a successful ingest of an empty workspace.
#[test]
fn query_rust_loc_summary_empty_table_yields_no_rows() {
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init_schema");

    let conn = db.lock().expect("lock");
    conn.execute_batch(
        "CREATE TABLE rust_loc_files (file VARCHAR, region VARCHAR, code BIGINT, \
         docs BIGINT, comments BIGINT, blanks BIGINT, lines BIGINT);
         CREATE VIEW rust_loc_summary AS \
         SELECT region, COUNT(*) AS files, SUM(code) AS code, SUM(docs) AS docs, \
         SUM(comments) AS comments, SUM(blanks) AS blanks, SUM(lines) AS lines \
         FROM rust_loc_files GROUP BY region ORDER BY code DESC",
    )
    .expect("create view");
    drop(conn);

    let stats = query_rust_loc_summary(&db).expect("query should work");
    assert!(
        stats.is_empty(),
        "no source rows means no regions: {stats:?}"
    );
}

/// The distinct-file count must not double-count a file that carries both
/// production code and a `#[cfg(test)]` block — the fixture's `src/lib.rs`
/// contributes a `main` row and a `test` row, so summing the summary
/// view's per-region `files` column would report 4 files for 3.
#[test]
fn query_rust_loc_file_count_counts_each_file_once() {
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init_schema");
    rust_loc_files_fixture(&db);

    let files = query_rust_loc_file_count(&db).expect("query should work");
    assert_eq!(files, 3, "src/lib.rs spans two regions but is one file");
}

#[test]
fn query_rust_loc_file_count_no_table() {
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init_schema");

    let files = query_rust_loc_file_count(&db).expect("missing table must not error");
    assert_eq!(files, 0);
}
