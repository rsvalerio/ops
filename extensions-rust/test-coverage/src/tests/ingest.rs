//! `CoverageIngestor` end-to-end loads into `DuckDB`.

use super::{setup_loaded_db, write_coverage_fixture};
use crate::ingestor::CoverageIngestor;
use ops_duckdb::{init_schema, DataIngestor, DuckDb};

#[test]
fn coverage_load_creates_table_and_view() {
    let (_data_dir, dir, db) = setup_loaded_db();

    let conn = db.lock().expect("lock");

    // Verify table
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM coverage_files", [], |row| row.get(0))
        .expect("count query");
    assert_eq!(count, 2, "should have 2 coverage file records");

    // Verify a specific row
    let filename: String = conn
        .query_row(
            "SELECT filename FROM coverage_files WHERE lines_count = 100",
            [],
            |row| row.get(0),
        )
        .expect("filename query");
    assert_eq!(filename, "src/main.rs");

    // Verify view
    let summary_lines: i64 = conn
        .query_row("SELECT lines_count FROM coverage_summary", [], |row| {
            row.get(0)
        })
        .expect("summary query");
    drop(conn);
    assert_eq!(summary_lines, 300, "summary should aggregate lines");

    // Verify staged files cleaned up
    assert!(!dir.entry_path("coverage_files.json").exists());
}

#[test]
fn coverage_files_has_data_returns_false_for_empty_db() {
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init schema");
    let has = ops_duckdb::sql::table_has_data(&db, "coverage_files").expect("check");
    assert!(!has, "empty db should have no coverage data");
}

#[test]
fn coverage_load_is_idempotent() {
    let (_data_dir, dir, db) = setup_loaded_db();
    // Second load against the same fixture: `setup_loaded_db` already
    // performed the first ingest; restage the sidecar (the loader
    // removes the staged JSON after a successful load) and ingest again.
    write_coverage_fixture(&dir);
    let ingestor = CoverageIngestor;
    let _ = ingestor.load(&dir, &db).expect("second load");

    let conn = db.lock().expect("lock");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM coverage_files", [], |row| row.get(0))
        .expect("count");
    drop(conn);
    assert_eq!(count, 2, "idempotent load should not duplicate rows");
}
