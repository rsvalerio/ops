//! Tests for the tokei extension.

use super::*;
use cargo_ops_duckdb::{init_schema, DataIngestor, DuckDb};
use cargo_ops_extension::Extension;

// -- Extension trait tests --

cargo_ops_extension::test_datasource_extension!(
    TokeiExtension,
    name: "tokei",
    data_provider: "tokei"
);

#[test]
fn tokei_extension_stack_is_none() {
    assert!(
        TokeiExtension.stack().is_none(),
        "tokei should be language-agnostic"
    );
}

// -- Provider tests --

#[test]
fn tokei_provider_name() {
    assert_eq!(TokeiProvider.name(), "tokei");
}

#[test]
fn tokei_provider_returns_valid_json_on_real_project() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut ctx = Context::test_context(manifest_dir);
    let value = TokeiProvider
        .provide(&mut ctx)
        .expect("tokei should succeed on this project");
    assert!(value.is_array(), "should return a JSON array");
    let arr = value.as_array().unwrap();
    assert!(!arr.is_empty(), "should find some files in the project");

    // Verify structure of first record
    let first = &arr[0];
    assert!(first.get("language").is_some());
    assert!(first.get("file").is_some());
    assert!(first.get("code").is_some());
    assert!(first.get("comments").is_some());
    assert!(first.get("blanks").is_some());
    assert!(first.get("lines").is_some());
}

#[test]
fn tokei_provider_empty_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut ctx = Context::test_context(dir.path().to_path_buf());
    let value = TokeiProvider
        .provide(&mut ctx)
        .expect("tokei should succeed on empty dir");
    assert!(value.is_array());
    assert!(
        value.as_array().unwrap().is_empty(),
        "empty dir should produce no records"
    );
}

#[test]
fn tokei_provider_schema_has_fields() {
    let schema = TokeiProvider.schema();
    assert!(!schema.description.is_empty());
    assert_eq!(schema.fields.len(), 6);
    let names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
    assert!(names.contains(&"language"));
    assert!(names.contains(&"file"));
    assert!(names.contains(&"code"));
    assert!(names.contains(&"comments"));
    assert!(names.contains(&"blanks"));
    assert!(names.contains(&"lines"));
}

// -- flatten_tokei_to_json tests --

#[test]
fn flatten_tokei_empty_languages() {
    let languages = Languages::new();
    let result = flatten_tokei_to_json(&languages, Path::new("/workspace"));
    assert!(result.is_array());
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
fn flatten_tokei_real_project_structure() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut languages = Languages::new();
    let config = TokeiConfig::default();
    let excluded: &[&str] = &[];
    languages.get_statistics(&[&manifest_dir], excluded, &config);

    let result = flatten_tokei_to_json(&languages, &manifest_dir);
    let arr = result.as_array().unwrap();
    assert!(!arr.is_empty(), "real project should have files");

    for record in arr {
        assert!(record["language"].is_string());
        assert!(record["file"].is_string());
        assert!(record["code"].is_number());
        assert!(record["comments"].is_number());
        assert!(record["blanks"].is_number());
        assert!(record["lines"].is_number());

        // lines should equal code + comments + blanks
        let code = record["code"].as_u64().unwrap();
        let comments = record["comments"].as_u64().unwrap();
        let blanks = record["blanks"].as_u64().unwrap();
        let lines = record["lines"].as_u64().unwrap();
        assert_eq!(
            lines,
            code + comments + blanks,
            "lines should be sum of code + comments + blanks"
        );
    }
}

#[test]
fn flatten_tokei_strips_workspace_prefix() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut languages = Languages::new();
    let config = TokeiConfig::default();
    let excluded: &[&str] = &[];
    languages.get_statistics(&[&manifest_dir], excluded, &config);

    let result = flatten_tokei_to_json(&languages, &manifest_dir);
    let arr = result.as_array().unwrap();

    for record in arr {
        let file = record["file"].as_str().unwrap();
        assert!(
            !file.starts_with(manifest_dir.to_str().unwrap()),
            "file path should be relative, got: {}",
            file
        );
    }
}

// -- collect_tokei tests --

#[test]
fn collect_tokei_on_real_project() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let result = collect_tokei(&manifest_dir).expect("collect_tokei should succeed");
    assert!(result.is_array());
    assert!(!result.as_array().unwrap().is_empty());
}

#[test]
fn collect_tokei_on_empty_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = collect_tokei(dir.path()).expect("collect_tokei should succeed on empty dir");
    assert!(result.is_array());
    assert!(result.as_array().unwrap().is_empty());
}

// -- DuckDB integration tests --

#[test]
fn tokei_collect_and_load_cycle() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    let ctx = Context::test_context(manifest_dir);

    // Collect
    let ingestor = TokeiIngestor;
    ingestor
        .collect(&ctx, data_dir.path())
        .expect("collect should succeed");
    assert!(data_dir.path().join("tokei_files.json").exists());
    assert!(data_dir.path().join("tokei_workspace.txt").exists());

    // Load
    ingestor
        .load(data_dir.path(), &db)
        .expect("load should succeed");

    // Verify data in DuckDB
    let conn = db.lock().expect("lock");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tokei_files", [], |row| row.get(0))
        .expect("count query");
    assert!(count > 0, "should have loaded records into tokei_files");

    // Verify view exists
    let lang_count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tokei_languages", [], |row| row.get(0))
        .expect("lang count query");
    assert!(lang_count > 0, "should have language aggregations");

    // Verify staged files cleaned up
    assert!(!data_dir.path().join("tokei_files.json").exists());
}

#[test]
fn tokei_files_has_data_returns_false_for_empty_db() {
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init schema");
    let has = cargo_ops_duckdb::sql::table_has_data(&db, "tokei_files").expect("check");
    assert!(!has, "empty db should have no tokei data");
}

// -- load_tokei tests --

#[test]
fn load_tokei_errors_when_json_missing() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    let err = load_tokei(data_dir.path(), &db).unwrap_err();
    assert!(
        err.to_string().contains("tokei_files.json not found"),
        "expected missing-json error, got: {}",
        err
    );
}

#[test]
fn load_tokei_succeeds_after_collect() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    // Collect data first
    let ctx = Context::test_context(manifest_dir);
    let ingestor = TokeiIngestor;
    ingestor
        .collect(&ctx, data_dir.path())
        .expect("collect should succeed");

    // load_tokei should succeed
    load_tokei(data_dir.path(), &db).expect("load_tokei should succeed");

    let conn = db.lock().expect("lock");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tokei_files", [], |row| row.get(0))
        .expect("count query");
    assert!(count > 0);
}

// -- query_tokei_files tests --

#[test]
fn query_tokei_files_returns_json_array() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    let ctx = Context::test_context(manifest_dir);
    let ingestor = TokeiIngestor;
    ingestor.collect(&ctx, data_dir.path()).expect("collect");
    ingestor.load(data_dir.path(), &db).expect("load");

    let result = query_tokei_files(&db).expect("query should succeed");
    let arr = result.as_array().expect("should be array");
    assert!(!arr.is_empty());

    // Verify all expected fields present with correct types
    for record in arr {
        assert!(record["language"].is_string());
        assert!(record["file"].is_string());
        assert!(record["code"].is_number());
        assert!(record["comments"].is_number());
        assert!(record["blanks"].is_number());
        assert!(record["lines"].is_number());
    }
}

// -- TokeiIngestor tests --

#[test]
fn tokei_ingestor_collect_empty_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let data_dir = tempfile::tempdir().expect("data tempdir");
    let ctx = Context::test_context(dir.path().to_path_buf());

    let ingestor = TokeiIngestor;
    ingestor
        .collect(&ctx, data_dir.path())
        .expect("collect on empty dir should succeed");

    // JSON file should exist even for empty dir (empty array)
    let json_path = data_dir.path().join("tokei_files.json");
    assert!(json_path.exists(), "json file should be created");
    let content = std::fs::read_to_string(&json_path).expect("read json");
    let parsed: serde_json::Value = serde_json::from_str(&content).expect("parse json");
    assert!(parsed.is_array());
    assert!(parsed.as_array().unwrap().is_empty());
}

#[test]
fn tokei_ingestor_checksum_changes_after_collect() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = tempfile::tempdir().expect("tempdir");
    let ctx = Context::test_context(manifest_dir);

    let ingestor = TokeiIngestor;
    ingestor.collect(&ctx, data_dir.path()).expect("collect");

    let checksum = ingestor
        .checksum(data_dir.path())
        .expect("checksum should succeed");
    assert!(!checksum.is_empty(), "checksum should not be empty");

    // Running checksum again should return the same value (deterministic)
    let checksum2 = ingestor.checksum(data_dir.path()).expect("checksum2");
    assert_eq!(checksum, checksum2, "checksum should be deterministic");
}

#[test]
fn tokei_ingestor_load_without_collect_fails() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    let ingestor = TokeiIngestor;
    let result = ingestor.load(data_dir.path(), &db);
    assert!(result.is_err(), "load without prior collect should fail");
}

// -- flatten_tokei_to_json edge cases --

#[test]
fn flatten_tokei_with_unrelated_prefix_keeps_full_path() {
    // When workspace_root is not a prefix of the file path, the full path is kept
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut languages = Languages::new();
    let config = TokeiConfig::default();
    let excluded: &[&str] = &[];
    languages.get_statistics(&[&manifest_dir], excluded, &config);

    // Use a completely unrelated root
    let unrelated_root = Path::new("/nonexistent/path/that/doesnt/match");
    let result = flatten_tokei_to_json(&languages, unrelated_root);
    let arr = result.as_array().unwrap();
    assert!(!arr.is_empty());

    // Files should retain full absolute paths since prefix doesn't match
    for record in arr {
        let file = record["file"].as_str().unwrap();
        assert!(
            file.starts_with('/') || file.contains(&manifest_dir.to_string_lossy().to_string()),
            "file should retain full path when prefix doesn't match, got: {}",
            file
        );
    }
}

// -- views tests --

#[test]
fn tokei_files_create_sql_with_real_json() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = tempfile::tempdir().expect("tempdir");

    // Collect real data to get a valid JSON file
    let ctx = Context::test_context(manifest_dir);
    let ingestor = TokeiIngestor;
    ingestor.collect(&ctx, data_dir.path()).expect("collect");

    let json_path = data_dir.path().join("tokei_files.json");
    let sql = views::tokei_files_create_sql(&json_path).expect("should generate SQL");
    assert!(
        sql.contains("tokei_files"),
        "SQL should reference tokei_files table"
    );
    assert!(
        sql.contains("tokei_files.json"),
        "SQL should reference the JSON file"
    );
}

// -- DuckDB query correctness --

#[test]
fn tokei_languages_view_aggregates_correctly() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    let ctx = Context::test_context(manifest_dir);
    let ingestor = TokeiIngestor;
    ingestor.collect(&ctx, data_dir.path()).expect("collect");
    ingestor.load(data_dir.path(), &db).expect("load");

    let conn = db.lock().expect("lock");

    // Get total from files table
    let files_total: i64 = conn
        .query_row("SELECT SUM(code) FROM tokei_files", [], |row| row.get(0))
        .expect("files sum");

    // Get total from languages view
    let view_total: i64 = conn
        .query_row("SELECT SUM(code) FROM tokei_languages", [], |row| {
            row.get(0)
        })
        .expect("view sum");

    assert_eq!(
        files_total, view_total,
        "languages view should aggregate all file-level code counts"
    );

    // Verify language count matches distinct languages
    let distinct_langs: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT language) FROM tokei_files",
            [],
            |row| row.get(0),
        )
        .expect("distinct langs");
    let view_langs: i64 = conn
        .query_row("SELECT COUNT(*) FROM tokei_languages", [], |row| row.get(0))
        .expect("view lang count");
    assert_eq!(
        distinct_langs, view_langs,
        "view should have one row per language"
    );
}
