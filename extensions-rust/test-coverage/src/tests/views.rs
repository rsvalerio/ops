//! `coverage_summary` view: the generated DDL and its query behaviour.

use super::setup_loaded_db;
use crate::ingestor::CoverageIngestor;
use crate::views::coverage_summary_view_sql;
use ops_duckdb::{DataIngestor, DuckDb};

ops_duckdb::test_create_sql_validation!(
    crate::views::coverage_files_create_sql,
    "coverage_files.json"
);

#[test]
fn coverage_summary_view_sql_contains_aggregation() {
    let sql = coverage_summary_view_sql().to_string();
    assert!(sql.contains("CREATE OR REPLACE VIEW \"coverage_summary\""));
    assert!(sql.contains("SUM(lines_count)"));
    assert!(sql.contains("SUM(lines_covered)"));
    assert!(sql.contains("SUM(functions_count)"));
    assert!(sql.contains("SUM(regions_count)"));
    assert!(sql.contains("SUM(branches_count)"));
    assert!(sql.contains("CASE WHEN"));
}

#[test]
fn coverage_summary_view_sql_has_all_percentage_columns() {
    let sql = coverage_summary_view_sql().to_string();
    assert!(sql.contains("AS lines_percent"));
    assert!(sql.contains("AS functions_percent"));
    assert!(sql.contains("AS regions_percent"));
    assert!(sql.contains("AS branches_percent"));
}

#[test]
fn coverage_summary_view_sql_has_zero_division_guards() {
    let sql = coverage_summary_view_sql().to_string();
    // Each metric type has a CASE WHEN ... > 0 guard with ELSE 0.0
    assert_eq!(
        sql.matches("ELSE 0.0 END").count(),
        4,
        "should have zero-division guards for lines, functions, regions, branches"
    );
}

#[test]
fn coverage_summary_view_sql_has_notcovered_columns() {
    let sql = coverage_summary_view_sql().to_string();
    assert!(sql.contains("regions_notcovered"));
    assert!(sql.contains("branches_notcovered"));
}

/// PATTERN-1 / TASK-1603: verify no ROUND in the view so downstream
/// consumers get full f64 precision.
#[test]
fn coverage_summary_view_sql_has_no_round() {
    let sql = coverage_summary_view_sql().to_string();
    assert!(
        !sql.contains("ROUND"),
        "view should not round; let presentation layer handle precision"
    );
}

/// READ-6 / TASK-1934: every count column is `COALESCE`d so the zero-row
/// aggregate returns 0 rather than NULL.
#[test]
fn coverage_summary_view_sql_coalesces_every_count_column() {
    let sql = coverage_summary_view_sql().to_string();
    for col in COUNT_COLUMNS {
        assert!(
            sql.contains(&format!("COALESCE(SUM({col}), 0) AS {col}")),
            "{col} must be COALESCEd so an empty coverage_files yields 0, not NULL"
        );
    }
}

/// The ten non-percentage columns of `coverage_summary`.
const COUNT_COLUMNS: &[&str] = &[
    "lines_count",
    "lines_covered",
    "functions_count",
    "functions_covered",
    "regions_count",
    "regions_covered",
    "regions_notcovered",
    "branches_count",
    "branches_covered",
    "branches_notcovered",
];

const PERCENT_COLUMNS: &[&str] = &[
    "lines_percent",
    "functions_percent",
    "regions_percent",
    "branches_percent",
];

#[test]
fn coverage_summary_view_computes_percentages() {
    let (_data_dir, _dir, db) = setup_loaded_db();
    let conn = db.lock().expect("lock");
    let lines_percent: f64 = conn
        .query_row("SELECT lines_percent FROM coverage_summary", [], |row| {
            row.get(0)
        })
        .expect("lines_percent query");
    drop(conn);
    // 270 covered / 300 total = 90.0%
    assert!(
        (lines_percent - 90.0).abs() < 0.01,
        "expected ~90.0%, got {lines_percent}"
    );
}

/// READ-6 / TASK-1934: **zero rows**, not one row of zeros. An ungrouped
/// aggregate over an empty table returns exactly one row whose SUMs are
/// NULL, so without `COALESCE` every count column decodes as NULL and a
/// consumer expecting a non-nullable integer fails outright.
///
/// This is a different case from
/// [`coverage_summary_view_handles_zero_counts`] below, which loads one
/// all-zero row and sums it to 0. Keep both: the two exercise different SQL
/// (NULL-vs-0 in `COALESCE`, and NULL-vs-0 in the percentage `CASE`).
#[test]
fn coverage_summary_view_empty_table_yields_zero_counts() {
    let (_data_dir, _dir, db) = setup_loaded_db();
    {
        let conn = db.lock().expect("lock");
        conn.execute_batch("DELETE FROM coverage_files")
            .expect("empty the table without dropping it or the view");
    }
    let conn = db.lock().expect("lock");
    for col in COUNT_COLUMNS {
        let value: i64 = conn
            .query_row(&format!("SELECT {col} FROM coverage_summary"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|e| panic!("{col} must decode as an integer over an empty table: {e}"));
        assert_eq!(value, 0, "{col} over zero rows must be 0, not NULL");
    }
    for col in PERCENT_COLUMNS {
        let value: f64 = conn
            .query_row(&format!("SELECT {col} FROM coverage_summary"), [], |row| {
                row.get(0)
            })
            .unwrap_or_else(|e| panic!("{col} must decode as a float over an empty table: {e}"));
        assert!(
            value.abs() < f64::EPSILON,
            "{col} over zero rows must be 0.0, got {value}"
        );
    }
    drop(conn);
}

/// One **all-zero row**, not zero rows — the counts sum to 0 and the
/// percentage `CASE` must yield 0.0 rather than NaN from a 0/0 division.
/// The zero-row case is
/// [`coverage_summary_view_empty_table_yields_zero_counts`] above; the two
/// are deliberately distinct and must not be merged.
#[test]
fn coverage_summary_view_handles_zero_counts() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    // SEC-25 / TASK-2054: stage through the same verified anchor
    // `provide_via_ingestor` builds.
    let dir = ops_duckdb::IngestDir::open(&data_dir.path().join("ingest")).expect("anchor");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    // Write fixture with all-zero counts
    let flat = serde_json::json!([{
        "filename": "empty.rs",
        "lines_count": 0, "lines_covered": 0, "lines_percent": 0.0,
        "functions_count": 0, "functions_covered": 0, "functions_percent": 0.0,
        "regions_count": 0, "regions_covered": 0, "regions_notcovered": 0, "regions_percent": 0.0,
        "branches_count": 0, "branches_covered": 0, "branches_notcovered": 0, "branches_percent": 0.0
    }]);
    let json_bytes = serde_json::to_vec_pretty(&flat).expect("serialize");
    std::fs::write(dir.entry_path("coverage_files.json"), &json_bytes).expect("write");
    std::fs::write(dir.entry_path("coverage_workspace.txt"), "/test/workspace")
        .expect("write workspace");

    let ingestor = CoverageIngestor;
    let _ = ingestor.load(&dir, &db).expect("load should succeed");

    let conn = db.lock().expect("lock");
    let lines_percent: f64 = conn
        .query_row("SELECT lines_percent FROM coverage_summary", [], |row| {
            row.get(0)
        })
        .expect("lines_percent query");
    drop(conn);
    assert!(
        (lines_percent - 0.0).abs() < 0.01,
        "zero counts should give 0% not NaN"
    );
}

#[test]
fn coverage_summary_view_all_metric_percentages() {
    let (_data_dir, _dir, db) = setup_loaded_db();
    let conn = db.lock().expect("lock");

    // functions: 27/30 = 90%
    let functions_percent: f64 = conn
        .query_row(
            "SELECT functions_percent FROM coverage_summary",
            [],
            |row| row.get(0),
        )
        .expect("functions_percent");
    assert!(
        (functions_percent - 90.0).abs() < 0.01,
        "expected ~90%, got {functions_percent}"
    );

    // regions: 54/60 = 90%
    let regions_percent: f64 = conn
        .query_row("SELECT regions_percent FROM coverage_summary", [], |row| {
            row.get(0)
        })
        .expect("regions_percent");
    assert!(
        (regions_percent - 90.0).abs() < 0.01,
        "expected ~90%, got {regions_percent}"
    );

    // branches: 12/15 = 80%
    let branches_percent: f64 = conn
        .query_row("SELECT branches_percent FROM coverage_summary", [], |row| {
            row.get(0)
        })
        .expect("branches_percent");
    assert!(
        (branches_percent - 80.0).abs() < 0.01,
        "expected ~80%, got {branches_percent}"
    );

    // notcovered aggregations
    let regions_notcovered: i64 = conn
        .query_row(
            "SELECT regions_notcovered FROM coverage_summary",
            [],
            |row| row.get(0),
        )
        .expect("regions_notcovered");
    assert_eq!(regions_notcovered, 6);

    let branches_notcovered: i64 = conn
        .query_row(
            "SELECT branches_notcovered FROM coverage_summary",
            [],
            |row| row.get(0),
        )
        .expect("branches_notcovered");
    drop(conn);
    assert_eq!(branches_notcovered, 3);
}
