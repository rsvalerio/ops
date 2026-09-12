//! Extension registration wiring and the public `load_coverage` entry point.

use super::write_coverage_fixture;
use crate::{load_coverage, CoverageExtension};
use ops_extension::Extension;
use ops_sqlite::Sqlite;

ops_extension::test_datasource_extension!(
    CoverageExtension,
    name: "coverage",
    data_provider: "coverage"
);

#[test]
fn coverage_extension_stack_is_none() {
    assert!(
        CoverageExtension.stack().is_none(),
        "coverage should be language-agnostic"
    );
}

#[test]
fn load_coverage_missing_json_file_errors() {
    // Write the workspace sidecar but not `coverage_files.json`. An empty
    // data_dir is missing both, and the sidecar is read first — so it would
    // fail on `coverage_workspace.txt` and never reach the coverage JSON this
    // test is named for. Satisfying the sidecar precondition is what puts the
    // JSON read on the failing path. We only assert the load fails; the exact
    // error message is implementation detail.
    let data_dir = tempfile::tempdir().expect("tempdir");
    // SEC-25 / TASK-2054: stage through the same verified anchor
    // `provide_via_ingestor` builds.
    let dir = ops_sqlite::IngestDir::open(&data_dir.path().join("ingest")).expect("anchor");
    dir.write_atomic("coverage_workspace.txt", b"/test/workspace")
        .expect("write workspace sidecar");
    let db = Sqlite::open_in_memory().expect("open in-memory db");
    let err = load_coverage(&dir, &db).unwrap_err();
    let msg = err.to_string();
    // SQLite port: the failure comes from the anchored `open_read` inside
    // `execute_json_load` — the staged bytes are read in Rust and bound as a
    // parameter, so a missing file fails before any SQL runs. The open error
    // is wrapped with the entry name, which is what proves this test reaches
    // the file it names.
    assert!(
        msg.contains("coverage_files.json"),
        "expected the missing coverage JSON to be named, got: {msg}"
    );
}

/// READ-5 (TASK-0808): the public `load_coverage` returns the structured
/// `LoadResult` so callers can act on `record_count` instead of treating the
/// load as opaque.
#[test]
fn load_coverage_returns_record_count() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    // SEC-25 / TASK-2054: stage through the same verified anchor
    // `provide_via_ingestor` builds.
    let dir = ops_sqlite::IngestDir::open(&data_dir.path().join("ingest")).expect("anchor");
    let db = Sqlite::open_in_memory().expect("open in-memory db");
    write_coverage_fixture(&dir);

    let result = load_coverage(&dir, &db).expect("load_coverage");
    assert_eq!(result.record_count, 2);
}
