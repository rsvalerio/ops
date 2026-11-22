//! `CoverageProvider` schema and the `SQLite` readback projection.

use super::setup_loaded_db;
use crate::ingestor::CoverageIngestor;
use crate::provider::{provide_from_db, query_coverage_files, CoverageProvider};
use ops_extension::DataProvider;
use ops_sqlite::{DataIngestor, Sqlite};

#[test]
fn coverage_provider_name() {
    assert_eq!(CoverageProvider.name(), "coverage");
}

/// The schema's field list is bound to `CoverageRow` itself, not restated.
/// The expected field-name set is derived by serializing a `CoverageRow`
/// (it derives `Serialize`), so adding or renaming a struct field fails this
/// test until `schema()` follows.
///
/// Ordering is deliberately *not* asserted: the two sides order fields
/// differently by construction (struct order vs schema order), and consumers
/// bind columns by name (`query_coverage_files`), so order is not part of
/// the contract — set equality is.
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
    assert!(!schema.description.is_empty(), "expected non-empty: {:?}", schema.description);
    let schema_names: std::collections::BTreeSet<&str> =
        schema.fields.iter().map(|f| f.name).collect();

    assert_eq!(
        row_keys, schema_names,
        "schema().fields must advertise exactly CoverageRow's serialized columns"
    );
}

/// Every column read back from `coverage_files` carries the value that was
/// loaded, in the column it was loaded into.
///
/// The shared fixture repeats values across columns (every `*_percent` of
/// `src/lib.rs` is 95.0), so a swap of two same-typed columns in
/// `query_coverage_files` would read back unchanged. This test loads a row
/// whose 15 fields are pairwise distinct, so any swapped or mis-bound column
/// changes the row and fails the comparison.
#[test]
fn query_coverage_files_round_trip() {
    let expected = serde_json::json!({
        "filename": "src/distinct.rs",
        "lines_count": 101,
        "lines_covered": 102,
        "lines_percent": 10.5,
        "functions_count": 103,
        "functions_covered": 104,
        "functions_percent": 20.5,
        "regions_count": 105,
        "regions_covered": 106,
        "regions_notcovered": 107,
        "regions_percent": 30.5,
        "branches_count": 108,
        "branches_covered": 109,
        "branches_notcovered": 110,
        "branches_percent": 40.5,
    });
    let data_dir = tempfile::tempdir().expect("tempdir");
    let dir = super::ingest_anchor(&data_dir);
    let staged = serde_json::to_vec(&serde_json::json!([expected])).expect("serialize fixture");
    dir.write_atomic("coverage_files.json", &staged).expect("write coverage_files.json");
    dir.write_atomic("coverage_workspace.txt", b"/test/workspace").expect("write workspace");
    let db = Sqlite::open_in_memory().expect("open in-memory db");
    let _ = CoverageIngestor.load(&dir, &db).expect("load should succeed");

    let rows = query_coverage_files(&db).expect("query");
    assert_eq!(rows, serde_json::json!([expected]));
}

/// The shared two-file fixture reads back with its per-file values intact.
#[test]
fn query_coverage_files_reads_back_shared_fixture() {
    let (_data_dir, _dir, db) = setup_loaded_db();
    let rows = query_coverage_files(&db).expect("query");
    let arr = rows.as_array().expect("rows are an array");
    assert_eq!(arr.len(), 2);
    let lib =
        arr.iter().find(|r| r["filename"] == "src/lib.rs").expect("src/lib.rs row is read back");
    assert_eq!(
        *lib,
        serde_json::json!({
            "filename": "src/lib.rs",
            "lines_count": 200,
            "lines_covered": 190,
            "lines_percent": 95.0,
            "functions_count": 20,
            "functions_covered": 19,
            "functions_percent": 95.0,
            "regions_count": 40,
            "regions_covered": 38,
            "regions_notcovered": 2,
            "regions_percent": 95.0,
            "branches_count": 10,
            "branches_covered": 9,
            "branches_notcovered": 1,
            "branches_percent": 90.0,
        })
    );
    assert!(arr.iter().any(|r| r["filename"] == "src/main.rs"));
}

/// `CoverageProvider::provide` takes the attached-db branch when a handle
/// is attached — the fixture rows come back and no `cargo` subprocess is
/// spawned. The context's working directory is a bare tempdir with no
/// `Cargo.toml`: if the downcast in `try_provide_from_db` ever stopped
/// resolving, the fallback would run `collect_coverage`, whose
/// `cargo llvm-cov` invocation fails in a non-cargo directory — this test
/// would then fail instead of silently paying a 15-minute workspace run on
/// every `ops about`.
#[test]
fn provide_reads_rows_from_attached_db_without_running_cargo() {
    let (_data_dir, _dir, db) = setup_loaded_db();
    let cwd = tempfile::tempdir().expect("bare cwd with no Cargo.toml");

    let mut ctx = ops_extension::Context::test_context(cwd.path().to_path_buf());
    ctx.attach_db(std::sync::Arc::new(db));

    let value = CoverageProvider.provide(&mut ctx).expect("provide");
    let arr = value.as_array().expect("rows array");
    assert_eq!(arr.len(), 2, "both fixture rows come back: {value}");
    let filenames: Vec<&str> =
        arr.iter().map(|r| r["filename"].as_str().expect("filename")).collect();
    assert!(filenames.contains(&"src/main.rs"));
    assert!(filenames.contains(&"src/lib.rs"));
}

/// `provide_from_db` against a `SQLite` whose `coverage_files` table already
/// holds rows takes the `provide_via_ingestor` short-circuit — the
/// `table_has_data` probe finds data and the ingestor's `collect` (cargo)
/// never runs. Same discrimination as the sibling test: the bare non-cargo
/// cwd makes any attempted collect fail loudly instead of re-ingesting.
#[test]
fn provide_from_db_short_circuits_when_table_has_rows() {
    let (_data_dir, _dir, db) = setup_loaded_db();
    let cwd = tempfile::tempdir().expect("bare cwd with no Cargo.toml");

    let ctx = ops_extension::Context::test_context(cwd.path().to_path_buf());
    let value = provide_from_db(&db, &ctx).expect("provide_from_db");
    let arr = value.as_array().expect("rows array");
    assert_eq!(arr.len(), 2, "the seeded rows are read back: {value}");
}
