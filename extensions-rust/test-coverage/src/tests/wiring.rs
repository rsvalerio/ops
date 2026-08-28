//! Extension registration wiring and the public `load_coverage` entry point.

use super::write_coverage_fixture;
use crate::{load_coverage, CoverageExtension};
use ops_duckdb::DuckDb;
use ops_extension::Extension;

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
    // An empty data_dir is missing both the workspace sidecar and the
    // coverage JSON; either is a fatal precondition for load. The
    // sidecar is read first, so the surfaced error is the IO NotFound
    // for `coverage_workspace.txt`. We only assert the load fails — the
    // exact error message is implementation detail.
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    let err = load_coverage(data_dir.path(), &db).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("No such file or directory") || msg.contains("not found"),
        "expected missing-file error, got: {msg}"
    );
}

/// READ-5 (TASK-0808): the public `load_coverage` returns the structured
/// `LoadResult` so callers can act on `record_count` instead of treating the
/// load as opaque.
#[test]
fn load_coverage_returns_record_count() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    write_coverage_fixture(data_dir.path());

    let result = load_coverage(data_dir.path(), &db).expect("load_coverage");
    assert_eq!(result.record_count, 2);
}
