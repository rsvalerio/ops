//! `CoverageProvider` schema and the `DuckDB` readback projection.

use super::setup_loaded_db;
use crate::provider::{query_coverage_files, CoverageProvider};
use ops_extension::DataProvider;

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

#[test]
fn query_coverage_files_round_trip() {
    let (_data_dir, db) = setup_loaded_db();
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
