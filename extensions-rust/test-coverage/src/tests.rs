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
    assert!(msg.contains("cargo llvm-cov failed"), "got: {msg}");
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
    assert!(err.to_string().contains("cargo llvm-cov failed"));
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

#[test]
fn flatten_coverage_json_missing_filename_defaults_to_empty() {
    let raw = serde_json::json!({
        "data": [{
            "files": [{
                "summary": {
                    "lines": { "count": 10, "covered": 5, "percent": 50.0 },
                    "functions": { "count": 2, "covered": 1, "percent": 50.0 },
                    "regions": { "count": 4, "covered": 2, "notcovered": 2, "percent": 50.0 },
                    "branches": { "count": 0, "covered": 0, "notcovered": 0, "percent": 0.0 }
                }
            }]
        }]
    });
    let result = flatten_coverage_json(&raw).expect("should handle missing filename");
    let arr = result.as_array().unwrap();
    assert_eq!(arr[0]["filename"], "");
    assert_eq!(arr[0]["lines_count"], 10);
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

#[test]
fn flatten_coverage_json_multiple_data_entries_uses_first() {
    let raw = serde_json::json!({
        "data": [
            { "files": [{ "filename": "first.rs", "summary": {} }] },
            { "files": [{ "filename": "second.rs", "summary": {} }] }
        ]
    });
    let result = flatten_coverage_json(&raw).expect("should use first data entry");
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0]["filename"], "first.rs");
}

// -- load_coverage tests --

#[test]
fn load_coverage_missing_json_file_errors() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    let err = load_coverage(data_dir.path(), &db).unwrap_err();
    assert!(
        err.to_string().contains("coverage_files.json not found"),
        "got: {}",
        err
    );
}

// -- CoverageIngestor checksum tests --

#[test]
fn coverage_ingestor_checksum_with_json_file() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    write_coverage_fixture(data_dir.path());
    let ingestor = CoverageIngestor;
    let checksum = ingestor.checksum(data_dir.path()).expect("checksum");
    assert_eq!(checksum.len(), 64, "SHA-256 hex should be 64 chars");
    assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));

    // Deterministic
    let checksum2 = ingestor.checksum(data_dir.path()).expect("checksum2");
    assert_eq!(checksum, checksum2);
}

#[test]
fn coverage_ingestor_checksum_missing_file_errors() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let ingestor = CoverageIngestor;
    let result = ingestor.checksum(data_dir.path());
    assert!(result.is_err(), "should fail when json file missing");
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
