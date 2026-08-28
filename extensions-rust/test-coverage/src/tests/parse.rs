//! `flatten_coverage_json` core behaviour: the well-formed report, the
//! multi-export merge, and the dedup policy. Malformed-input handling lives
//! in [`super::parse_edge`].

use super::sample_coverage_json;
use crate::parse::flatten_coverage_json;

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
