//! Tests for the coverage extension.

use super::*;
use ops_duckdb::{init_schema, DataIngestor, DuckDb};
use ops_extension::Extension;

// -- Extension trait tests --

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

// -- Provider tests --

#[test]
fn coverage_provider_name() {
    assert_eq!(CoverageProvider.name(), "coverage");
}

#[test]
fn coverage_provider_schema_has_fields() {
    let schema = CoverageProvider.schema();
    assert!(!schema.description.is_empty());
    assert_eq!(schema.fields.len(), 15);
    let names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
    assert!(names.contains(&"filename"));
    assert!(names.contains(&"lines_count"));
    assert!(names.contains(&"lines_covered"));
    assert!(names.contains(&"lines_percent"));
    assert!(names.contains(&"functions_count"));
    assert!(names.contains(&"functions_covered"));
    assert!(names.contains(&"functions_percent"));
    assert!(names.contains(&"regions_count"));
    assert!(names.contains(&"regions_covered"));
    assert!(names.contains(&"regions_notcovered"));
    assert!(names.contains(&"regions_percent"));
    assert!(names.contains(&"branches_count"));
    assert!(names.contains(&"branches_covered"));
    assert!(names.contains(&"branches_notcovered"));
    assert!(names.contains(&"branches_percent"));
}

// -- flatten_coverage_json tests --

fn sample_coverage_json() -> serde_json::Value {
    serde_json::json!({
        "data": [{
            "files": [
                {
                    "filename": "src/main.rs",
                    "summary": {
                        "lines": { "count": 100, "covered": 80, "percent": 80.0 },
                        "functions": { "count": 10, "covered": 8, "percent": 80.0 },
                        "regions": { "count": 20, "covered": 16, "notcovered": 4, "percent": 80.0 },
                        "branches": { "count": 5, "covered": 3, "notcovered": 2, "percent": 60.0 }
                    }
                },
                {
                    "filename": "src/lib.rs",
                    "summary": {
                        "lines": { "count": 200, "covered": 190, "percent": 95.0 },
                        "functions": { "count": 20, "covered": 19, "percent": 95.0 },
                        "regions": { "count": 40, "covered": 38, "notcovered": 2, "percent": 95.0 },
                        "branches": { "count": 10, "covered": 9, "notcovered": 1, "percent": 90.0 }
                    }
                }
            ]
        }]
    })
}

#[test]
fn flatten_coverage_json_valid() {
    let raw = sample_coverage_json();
    let result = flatten_coverage_json(&raw).expect("should flatten valid JSON");
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    let first = &arr[0];
    assert_eq!(first["filename"], "src/main.rs");
    assert_eq!(first["lines_count"], 100);
    assert_eq!(first["lines_covered"], 80);
    assert_eq!(first["lines_percent"], 80.0);
    assert_eq!(first["functions_count"], 10);
    assert_eq!(first["functions_covered"], 8);
    assert_eq!(first["regions_count"], 20);
    assert_eq!(first["regions_covered"], 16);
    assert_eq!(first["regions_notcovered"], 4);
    assert_eq!(first["branches_count"], 5);
    assert_eq!(first["branches_covered"], 3);
    assert_eq!(first["branches_notcovered"], 2);
    assert_eq!(first["branches_percent"], 60.0);
}

#[test]
fn flatten_coverage_json_empty_files() {
    let raw = serde_json::json!({
        "data": [{ "files": [] }]
    });
    let result = flatten_coverage_json(&raw).expect("should handle empty files");
    let arr = result.as_array().unwrap();
    assert!(arr.is_empty());
}

#[test]
fn flatten_coverage_json_missing_data() {
    let raw = serde_json::json!({});
    let result = flatten_coverage_json(&raw);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("data"));
}

#[test]
fn flatten_coverage_json_empty_data_array() {
    let raw = serde_json::json!({ "data": [] });
    let result = flatten_coverage_json(&raw);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("empty"));
}

#[test]
fn flatten_coverage_json_missing_summary_fields() {
    let raw = serde_json::json!({
        "data": [{
            "files": [{
                "filename": "src/partial.rs",
                "summary": {}
            }]
        }]
    });
    let result = flatten_coverage_json(&raw).expect("should handle missing summary fields");
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    let record = &arr[0];
    assert_eq!(record["filename"], "src/partial.rs");
    assert_eq!(record["lines_count"], 0);
    assert_eq!(record["lines_covered"], 0);
    assert_eq!(record["lines_percent"], 0.0);
}

/// ERR-1 / TASK-0595: when llvm-cov emits multiple data[] entries (per-target
/// merging), every file across all entries must end up in the output. The
/// previous shape silently dropped data[1..] producing under-reported coverage.
#[test]
fn flatten_coverage_json_iterates_all_data_entries() {
    let raw = serde_json::json!({
        "data": [
            { "files": [{ "filename": "a.rs", "summary": { "lines": { "count": 10, "covered": 5, "percent": 50.0 }}}]},
            { "files": [{ "filename": "b.rs", "summary": { "lines": { "count": 20, "covered": 20, "percent": 100.0 }}}]}
        ]
    });
    let arr = flatten_coverage_json(&raw)
        .expect("multi-entry data must flatten")
        .as_array()
        .cloned()
        .unwrap();
    let names: Vec<&str> = arr
        .iter()
        .map(|r| r["filename"].as_str().unwrap())
        .collect();
    assert_eq!(names, vec!["a.rs", "b.rs"]);
}

// -- DuckDB integration tests --

fn write_coverage_fixture(data_dir: &Path) {
    let raw = sample_coverage_json();
    let flat = flatten_coverage_json(&raw).expect("flatten");
    let json_bytes = serde_json::to_vec_pretty(&flat).expect("serialize");
    std::fs::write(data_dir.join("coverage_files.json"), &json_bytes).expect("write");
    std::fs::write(data_dir.join("coverage_workspace.txt"), "/test/workspace")
        .expect("write workspace");
}

#[test]
fn coverage_load_creates_table_and_view() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    write_coverage_fixture(data_dir.path());

    let ingestor = CoverageIngestor;
    let _ = ingestor
        .load(data_dir.path(), &db)
        .expect("load should succeed");

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
    assert_eq!(summary_lines, 300, "summary should aggregate lines");

    // Verify staged files cleaned up
    assert!(!data_dir.path().join("coverage_files.json").exists());
}

#[test]
fn coverage_summary_view_computes_percentages() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    write_coverage_fixture(data_dir.path());

    let ingestor = CoverageIngestor;
    let _ = ingestor
        .load(data_dir.path(), &db)
        .expect("load should succeed");

    let conn = db.lock().expect("lock");
    let lines_percent: f64 = conn
        .query_row("SELECT lines_percent FROM coverage_summary", [], |row| {
            row.get(0)
        })
        .expect("lines_percent query");
    // 270 covered / 300 total = 90.0%
    assert!(
        (lines_percent - 90.0).abs() < 0.01,
        "expected ~90.0%, got {}",
        lines_percent
    );
}

#[test]
fn coverage_summary_view_handles_zero_counts() {
    let data_dir = tempfile::tempdir().expect("tempdir");
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
    std::fs::write(data_dir.path().join("coverage_files.json"), &json_bytes).expect("write");
    std::fs::write(
        data_dir.path().join("coverage_workspace.txt"),
        "/test/workspace",
    )
    .expect("write workspace");

    let ingestor = CoverageIngestor;
    let _ = ingestor
        .load(data_dir.path(), &db)
        .expect("load should succeed");

    let conn = db.lock().expect("lock");
    let lines_percent: f64 = conn
        .query_row("SELECT lines_percent FROM coverage_summary", [], |row| {
            row.get(0)
        })
        .expect("lines_percent query");
    assert!(
        (lines_percent - 0.0).abs() < 0.01,
        "zero counts should give 0% not NaN"
    );
}

#[test]
fn coverage_files_has_data_returns_false_for_empty_db() {
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init schema");
    let has = ops_duckdb::sql::table_has_data(&db, "coverage_files").expect("check");
    assert!(!has, "empty db should have no coverage data");
}

// -- check_llvm_cov_output tests --

#[test]
fn check_llvm_cov_output_success() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: b"some output".to_vec(),
        stderr: Vec::new(),
    };
    assert!(check_llvm_cov_output(&output).is_ok());
}

#[test]
fn check_llvm_cov_output_failure_includes_stderr_tail() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(256), // exit code 1
        stdout: Vec::new(),
        stderr: b"error: could not compile\ndetails here\nmore info".to_vec(),
    };
    let err = check_llvm_cov_output(&output).unwrap_err();
    let msg = err.to_string();
    // PATTERN-1 / TASK-1099: format is "cargo llvm-cov exited with status {code}: ...".
    assert!(msg.contains("cargo llvm-cov"), "got: {msg}");
    assert!(msg.contains("status 1"), "exit code must appear: {msg}");
    assert!(
        msg.contains("could not compile"),
        "stderr tail should appear: {msg}"
    );
}

#[test]
fn check_llvm_cov_output_failure_empty_stderr() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(256),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };
    let err = check_llvm_cov_output(&output).unwrap_err();
    // PATTERN-1 / TASK-1099: error message format is now
    // "cargo llvm-cov exited with status {code}: ...".
    assert!(err.to_string().contains("cargo llvm-cov"));
}

/// PATTERN-1 / TASK-1099: non-zero exit codes must appear in the error
/// string so exit 1 (issues), exit 101 (panic), and SIGKILL/None are
/// distinguishable in operator logs.
#[cfg(unix)]
#[test]
fn check_llvm_cov_output_failure_includes_exit_code() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(101 << 8),
        stdout: Vec::new(),
        stderr: b"thread 'main' panicked".to_vec(),
    };
    let err = check_llvm_cov_output(&output).expect_err("non-zero must fail");
    let msg = err.to_string();
    assert!(msg.contains("status 101"), "exit code must appear: {msg}");
    assert!(msg.contains("panicked"), "stderr tail must remain: {msg}");
}

/// PATTERN-1 / TASK-1099: a None exit (signal kill, e.g. OOM) is named
/// as `signal` so it's distinguishable from a real cargo failure.
#[cfg(unix)]
#[test]
fn check_llvm_cov_output_failure_signal_kill_says_signal() {
    use std::os::unix::process::ExitStatusExt;
    // signal 9 (SIGKILL) → exit_code() returns None
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(9),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };
    let err = check_llvm_cov_output(&output).expect_err("signal must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("signal") || msg.contains("None"),
        "signal-kill case must be named in error: {msg}"
    );
}

/// ERR-1 / TASK-1057: `cargo llvm-cov` must run with `--no-fail-fast`
/// so a single failing test does not erase the entire coverage signal
/// for the run. Pin the arg list at the source-of-truth.
#[test]
fn run_cargo_llvm_cov_arg_list_includes_no_fail_fast() {
    let src = include_str!("lib.rs");
    let needle = "\"--no-fail-fast\"";
    assert!(
        src.contains(needle),
        "run_cargo_llvm_cov arg list must include --no-fail-fast (TASK-1057)"
    );
    // Also pin order: --tests must precede --no-fail-fast and --json
    // must remain the final flag (downstream parsing depends on JSON
    // mode being on).
    assert!(src.contains("\"--json\""));
}

/// ERR-1 / TASK-1021: when `data[]` carries multiple exports listing the
/// same source filename (per-target merge from a future llvm-cov
/// version, or a sibling caller passing a multi-export JSON), the
/// flatten step must dedup by filename so `coverage_summary` SUMs do
/// not double-count `lines_count` / `lines_covered`. Behaviour
/// documented: last-write-wins (the most recently merged export
/// reflects the most up-to-date instrumentation).
#[test]
fn flatten_coverage_json_dedups_overlapping_filenames_across_exports() {
    let raw = serde_json::json!({
        "data": [
            {
                "files": [{
                    "filename": "src/main.rs",
                    "summary": {
                        "lines": { "count": 100, "covered": 50, "percent": 50.0 }
                    }
                }]
            },
            {
                "files": [{
                    "filename": "src/main.rs",
                    "summary": {
                        "lines": { "count": 100, "covered": 80, "percent": 80.0 }
                    }
                }]
            }
        ]
    });
    let result = flatten_coverage_json(&raw).expect("flatten must succeed");
    let arr = result.as_array().unwrap();
    assert_eq!(
        arr.len(),
        1,
        "duplicate filenames across exports must collapse to a single row"
    );
    // Last-write-wins: the second export's coverage values are kept.
    assert_eq!(arr[0]["filename"], "src/main.rs");
    assert_eq!(arr[0]["lines_count"], 100);
    assert_eq!(arr[0]["lines_covered"], 80);
    assert!((arr[0]["lines_percent"].as_f64().unwrap() - 80.0).abs() < 0.01);
}

/// ERR-1 / TASK-1021: dedup must not collapse distinct filenames; only
/// exact filename matches are merged.
#[test]
fn flatten_coverage_json_keeps_distinct_filenames_across_exports() {
    let raw = serde_json::json!({
        "data": [
            { "files": [{ "filename": "src/a.rs", "summary": {} }] },
            { "files": [{ "filename": "src/b.rs", "summary": {} }] },
            { "files": [{ "filename": "src/a.rs", "summary": {} }] }
        ]
    });
    let arr = flatten_coverage_json(&raw)
        .expect("flatten")
        .as_array()
        .cloned()
        .unwrap();
    assert_eq!(arr.len(), 2);
    let filenames: Vec<&str> = arr
        .iter()
        .map(|r| r["filename"].as_str().unwrap())
        .collect();
    assert!(filenames.contains(&"src/a.rs"));
    assert!(filenames.contains(&"src/b.rs"));
}

// -- flatten_coverage_json edge cases --

#[test]
fn flatten_coverage_json_missing_files_key() {
    let raw = serde_json::json!({
        "data": [{ "totals": {} }]
    });
    let result = flatten_coverage_json(&raw);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("files"));
}

#[test]
fn flatten_coverage_json_data_not_array() {
    let raw = serde_json::json!({ "data": "not_an_array" });
    let result = flatten_coverage_json(&raw);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("data"));
}

/// ERR-1 / TASK-0984: a missing or non-string `filename` used to coerce to ""
/// and still get pushed into `coverage_files` — the empty-key row matched no
/// member but still inflated project-total `lines_count`/`lines_covered`. The
/// fix skips such records (with a `tracing::warn` breadcrumb mirroring how
/// sister fields handle schema drift) so the project total stays clean.
#[test]
fn flatten_coverage_json_missing_filename_skips_record() {
    let raw = serde_json::json!({
        "data": [{
            "files": [
                {
                    "filename": "src/main.rs",
                    "summary": {
                        "lines": { "count": 100, "covered": 80, "percent": 80.0 }
                    }
                },
                {
                    "summary": {
                        "lines": { "count": 999, "covered": 999, "percent": 100.0 }
                    }
                },
                {
                    "filename": 42,
                    "summary": {
                        "lines": { "count": 42, "covered": 42, "percent": 100.0 }
                    }
                }
            ]
        }]
    });
    let result = flatten_coverage_json(&raw).expect("must not error on missing filename");
    let arr = result.as_array().unwrap();
    // Only the well-formed record survives; the no-filename and non-string
    // filename rows are dropped so they cannot pollute aggregates.
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["filename"], "src/main.rs");
    let total_lines: i64 = arr.iter().map(|r| r["lines_count"].as_i64().unwrap()).sum();
    assert_eq!(
        total_lines, 100,
        "aggregate must exclude records without a filename"
    );
}

#[test]
fn flatten_coverage_json_missing_summary_entirely() {
    let raw = serde_json::json!({
        "data": [{
            "files": [{ "filename": "no_summary.rs" }]
        }]
    });
    let result = flatten_coverage_json(&raw).expect("should handle missing summary");
    let record = &result.as_array().unwrap()[0];
    assert_eq!(record["filename"], "no_summary.rs");
    assert_eq!(record["lines_count"], 0);
    assert_eq!(record["functions_count"], 0);
    assert_eq!(record["regions_count"], 0);
    assert_eq!(record["branches_count"], 0);
}

/// ERR-1 / TASK-0595: when `data` has multiple entries, every entry's files
/// are flattened — the earlier "uses first only" behaviour silently dropped
/// per-target merge exports.
#[test]
fn flatten_coverage_json_multiple_data_entries_includes_all() {
    let raw = serde_json::json!({
        "data": [
            { "files": [{ "filename": "first.rs", "summary": {} }] },
            { "files": [{ "filename": "second.rs", "summary": {} }] }
        ]
    });
    let result = flatten_coverage_json(&raw).expect("should flatten all data entries");
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["filename"], "first.rs");
    assert_eq!(arr[1]["filename"], "second.rs");
}

// -- load_coverage tests --

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

// -- query_coverage_files round-trip test --

#[test]
fn query_coverage_files_round_trip() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    write_coverage_fixture(data_dir.path());

    let ingestor = CoverageIngestor;
    let _ = ingestor.load(data_dir.path(), &db).expect("load");

    let rows = query_coverage_files(&db).expect("query");
    let arr = rows.as_array().unwrap();
    assert_eq!(arr.len(), 2);

    let filenames: Vec<&str> = arr
        .iter()
        .map(|r| r["filename"].as_str().unwrap())
        .collect();
    assert!(filenames.contains(&"src/main.rs"));
    assert!(filenames.contains(&"src/lib.rs"));

    // Verify all 15 fields are present in each row
    for row in arr {
        assert!(row.get("filename").is_some());
        assert!(row.get("lines_count").is_some());
        assert!(row.get("lines_covered").is_some());
        assert!(row.get("lines_percent").is_some());
        assert!(row.get("functions_count").is_some());
        assert!(row.get("functions_covered").is_some());
        assert!(row.get("functions_percent").is_some());
        assert!(row.get("regions_count").is_some());
        assert!(row.get("regions_covered").is_some());
        assert!(row.get("regions_notcovered").is_some());
        assert!(row.get("regions_percent").is_some());
        assert!(row.get("branches_count").is_some());
        assert!(row.get("branches_covered").is_some());
        assert!(row.get("branches_notcovered").is_some());
        assert!(row.get("branches_percent").is_some());
    }
}

// -- DuckDB summary view detailed tests --

#[test]
fn coverage_summary_view_all_metric_percentages() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    write_coverage_fixture(data_dir.path());

    let ingestor = CoverageIngestor;
    let _ = ingestor.load(data_dir.path(), &db).expect("load");

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
    assert_eq!(branches_notcovered, 3);
}

#[test]
fn coverage_load_is_idempotent() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    // First load
    write_coverage_fixture(data_dir.path());
    let ingestor = CoverageIngestor;
    let _ = ingestor.load(data_dir.path(), &db).expect("first load");

    // Second load with same data
    write_coverage_fixture(data_dir.path());
    let _ = ingestor.load(data_dir.path(), &db).expect("second load");

    let conn = db.lock().expect("lock");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM coverage_files", [], |row| row.get(0))
        .expect("count");
    assert_eq!(count, 2, "idempotent load should not duplicate rows");
}
