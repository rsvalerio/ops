//! `flatten_coverage_json` against malformed llvm-cov reports: missing
//! top-level shapes, schema drift in the per-section counters, and file
//! records the flattener must skip. The well-formed paths live in
//! [`super::parse`].

use crate::parse::flatten_coverage_json;

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

/// TASK-1599: when multiple files have the same wrong-shape field (e.g. count
/// is a string), the `DriftTracker` ensures the warn fires only once per
/// (section, field) pair. Two inserts of the same key → set grows by 1, not 2.
#[test]
fn drift_tracker_deduplicates_warnings_per_section_field_pair() {
    use std::collections::HashSet;
    let mut warned: HashSet<(String, String)> = HashSet::new();
    let v = serde_json::Value::String("not-an-int".to_string());
    {
        let mut drift = crate::parse::DriftTracker::new("lines", &mut warned);
        drift.warn_wrong_shape("count", &v, "an integer");
        drift.warn_wrong_shape("count", &v, "an integer");
        drift.warn_wrong_shape("covered", &v, "an integer");
    }
    assert_eq!(
        warned.len(),
        2,
        "should have exactly 2 unique (section, field) pairs"
    );
}

/// TASK-1599: `flatten_coverage_json` still produces correct output when fields
/// are wrong-shape across many files (drift-tracked, not spammed).
#[test]
fn flatten_coverage_json_wrong_shape_fields_still_flattens() {
    let raw = serde_json::json!({
        "data": [{
            "files": [
                { "filename": "a.rs", "summary": { "lines": { "count": "bad", "covered": "bad", "notcovered": "bad", "percent": "bad" } } },
                { "filename": "b.rs", "summary": { "lines": { "count": "bad", "covered": "bad", "notcovered": "bad", "percent": "bad" } } },
                { "filename": "c.rs", "summary": { "lines": { "count": "bad", "covered": "bad", "notcovered": "bad", "percent": "bad" } } }
            ]
        }]
    });
    let result = flatten_coverage_json(&raw).expect("should still flatten");
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), 3, "all three files should be present");
    // Wrong-shape fields coerce to default (0)
    for row in arr {
        assert_eq!(row["lines_count"], 0);
    }
}

/// TASK-1600: when all file records are malformed (missing/non-string
/// filename), the output has zero rows. The skipped-count summary warn
/// fires once (not per-record). We verify the output is empty; the warn
/// volume is covered by the `DriftTracker` pattern.
#[test]
fn flatten_coverage_json_all_malformed_records_produces_empty_output() {
    let raw = serde_json::json!({
        "data": [{
            "files": [
                { "summary": { "lines": { "count": 10, "covered": 5, "percent": 50.0 } } },
                { "filename": 42, "summary": {} },
                { "filename": "", "summary": {} }
            ]
        }]
    });
    let result = flatten_coverage_json(&raw).expect("should not error");
    let arr = result.as_array().unwrap();
    assert!(
        arr.is_empty(),
        "all three malformed records should be skipped, got {} rows",
        arr.len()
    );
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
