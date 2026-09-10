//! `CoverageProvider` schema and the `DuckDB` readback projection.

use super::setup_loaded_db;
use crate::provider::{provide_from_db, query_coverage_files, CoverageProvider};
use ops_extension::DataProvider;

#[test]
fn coverage_provider_name() {
    assert_eq!(CoverageProvider.name(), "coverage");
}

/// TEST-23 / TASK-2195: the schema's field list is bound to `CoverageRow`
/// itself, not restated. The expected field-name set is derived by
/// serializing a `CoverageRow` (it derives `Serialize`), so adding or
/// renaming a struct field fails this test until `schema()` follows — the
/// previous shape hardcoded the same 15 literals it guarded and passed
/// unchanged after a `CoverageRow` edit.
///
/// Ordering is deliberately *not* asserted: the two sides order fields
/// differently by construction (struct order vs schema order), and consumers
/// bind columns by name (`query_coverage_files`, TASK-1610), so order is not
/// part of the contract — set equality is.
#[test]
fn coverage_provider_schema_fields_match_covered_row_serialization() {
    let row = crate::parse::CoverageRow {
        filename: "src/lib.rs".to_string(),
        lines_count: 1,
        lines_covered: 2,
        lines_percent: 3.0,
        functions_count: 4,
        functions_covered: 5,
        functions_percent: 6.0,
        regions_count: 7,
        regions_covered: 8,
        regions_notcovered: 9,
        regions_percent: 10.0,
        branches_count: 11,
        branches_covered: 12,
        branches_notcovered: 13,
        branches_percent: 14.0,
    };
    let serialized = serde_json::to_value(&row).expect("CoverageRow serializes");
    let object = serialized.as_object().expect("row serializes to an object");
    let row_keys: std::collections::BTreeSet<&str> = object.keys().map(String::as_str).collect();

    let schema = CoverageProvider.schema();
    assert!(!schema.description.is_empty());
    let schema_names: std::collections::BTreeSet<&str> =
        schema.fields.iter().map(|f| f.name).collect();

    assert_eq!(
        row_keys, schema_names,
        "schema().fields must advertise exactly CoverageRow's serialized columns"
    );
}

#[test]
fn query_coverage_files_round_trip() {
    let (_data_dir, _dir, db) = setup_loaded_db();
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

/// TEST-5 / TASK-2190 AC #1 + #2: `CoverageProvider::provide` takes the
/// `DuckDB` branch when a handle is attached — the fixture rows come back and
/// no `cargo` subprocess is spawned. The context's working directory is a
/// bare tempdir with no `Cargo.toml`: if the downcast in
/// `try_provide_from_db` ever stopped resolving (the live hazard documented
/// at `extensions/duckdb/src/lib.rs` — a stray `DuckDbHandle` import flips
/// every downcast to `None`), the fallback would run `collect_coverage`,
/// whose `cargo llvm-cov` invocation fails in a non-cargo directory — this
/// test would then fail instead of silently paying a 15-minute workspace
/// run on every `ops about`.
#[test]
fn provide_reads_rows_from_attached_db_without_running_cargo() {
    let (_data_dir, _dir, db) = setup_loaded_db();
    let cwd = tempfile::tempdir().expect("bare cwd with no Cargo.toml");

    let mut ctx = ops_extension::Context::test_context(cwd.path().to_path_buf());
    ctx.attach_db(std::sync::Arc::new(db));

    let value = CoverageProvider.provide(&mut ctx).expect("provide");
    let arr = value.as_array().expect("rows array");
    assert_eq!(arr.len(), 2, "both fixture rows come back: {value}");
    let filenames: Vec<&str> = arr
        .iter()
        .map(|r| r["filename"].as_str().expect("filename"))
        .collect();
    assert!(filenames.contains(&"src/main.rs"));
    assert!(filenames.contains(&"src/lib.rs"));
}

/// TEST-5 / TASK-2190 AC #3: `provide_from_db` against a `DuckDB` whose
/// `coverage_files` table already holds rows takes the
/// `provide_via_ingestor` short-circuit — the `table_has_data` probe finds
/// data and the ingestor's `collect` (cargo) never runs. Same discrimination
/// as the sibling test: the bare non-cargo cwd makes any attempted collect
/// fail loudly instead of re-ingesting.
#[test]
fn provide_from_db_short_circuits_when_table_has_rows() {
    let (_data_dir, _dir, db) = setup_loaded_db();
    let cwd = tempfile::tempdir().expect("bare cwd with no Cargo.toml");

    let ctx = ops_extension::Context::test_context(cwd.path().to_path_buf());
    let value = provide_from_db(&db, &ctx).expect("provide_from_db");
    let arr = value.as_array().expect("rows array");
    assert_eq!(arr.len(), 2, "the seeded rows are read back: {value}");
}
