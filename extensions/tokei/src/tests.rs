//! Tests for the tokei extension.
//!
//! ## Test isolation policy (TEST-17/TEST-18)
//!
//! Tests that scan the live workspace via `env!("CARGO_MANIFEST_DIR")` are
//! non-deterministic (file counts depend on the working tree) and slow.
//! Any such test MUST be gated behind
//! `#[ignore = "scans CARGO_MANIFEST_DIR; non-deterministic and slow (TEST-17)"]`,
//! matching the precedent set by `tokei_provider_returns_valid_json_on_real_project`.
//! Prefer `tempfile::tempdir()` plus canned fixture files for default `cargo test`
//! coverage; ignored tests remain available for ad-hoc smoke runs via
//! `cargo test -- --ignored`.
//!
//! TEST-18 (TASK-1977): five tests used to scan the live crate directory
//! without that gate, and paid for it with `> 0` assertions that would have
//! passed on almost any input. They now build a [`fixture_project`] tree with
//! known contents and assert the exact counts it implies.

use super::*;
use ops_duckdb::{init_schema, DataIngestor, DuckDb};
use ops_extension::{Extension, ExtensionType};

// -- fixtures --

/// Number of files in [`fixture_project`], and therefore the exact record
/// count every scan of it must produce.
const FIXTURE_FILE_COUNT: usize = 2;
/// Lines of code across [`fixture_project`]: one per source file.
const FIXTURE_TOTAL_CODE: i64 = 2;
/// Distinct languages in [`fixture_project`]: Rust and Python.
const FIXTURE_LANGUAGE_COUNT: i64 = 2;

/// A deterministic two-file, two-language project in a tempdir.
///
/// `src/lib.rs` and `app.py` each hold exactly one line of code and one
/// comment line, so every count derived from this tree is exact.
fn fixture_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(dir.path().join("src/lib.rs"), "// comment\nfn a() {}\n").expect("write rust");
    std::fs::write(dir.path().join("app.py"), "# comment\nprint(1)\n").expect("write python");
    dir
}

// -- Extension trait tests --

// Macro generates two tests:
//   - extension_name: TokeiExtension.name() == "tokei"
//   - extension_registers_data_provider: registry.get("tokei").is_some() after register_data_providers
ops_extension::test_datasource_extension!(
    TokeiExtension,
    name: "tokei",
    data_provider: "tokei"
);

#[test]
fn tokei_extension_type_is_datasource() {
    assert_eq!(TokeiExtension.types(), ExtensionType::DATASOURCE);
}

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
fn tokei_provider_returns_valid_json_on_canned_dir() {
    // Deterministic: a fixed Rust file in a tempdir, independent of repo
    // contents. Avoids scanning the whole crate at test time (TEST-17).
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("hello.rs"),
        "// comment\nfn main() {\n    println!(\"hi\");\n}\n",
    )
    .expect("write source");

    let mut ctx = Context::test_context(dir.path().to_path_buf());
    let value = TokeiProvider
        .provide(&mut ctx)
        .expect("tokei should succeed on tempdir");
    assert!(value.is_array(), "should return a JSON array");
    let arr = value.as_array().unwrap();
    assert!(!arr.is_empty(), "should find the canned source file");

    let first = &arr[0];
    assert!(first.get("language").is_some());
    assert!(first.get("file").is_some());
    assert!(first.get("code").is_some());
    assert!(first.get("comments").is_some());
    assert!(first.get("blanks").is_some());
    assert!(first.get("lines").is_some());
}

// Live workspace scan retained but ignored — kept for ad-hoc smoke testing
// against the actual repo via `cargo test -- --ignored`.
#[test]
#[ignore = "scans CARGO_MANIFEST_DIR; non-deterministic and slow (TEST-17)"]
fn tokei_provider_returns_valid_json_on_real_project() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut ctx = Context::test_context(manifest_dir);
    let value = TokeiProvider
        .provide(&mut ctx)
        .expect("tokei should succeed on this project");
    assert!(value.is_array(), "should return a JSON array");
    assert!(!value.as_array().unwrap().is_empty());
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

// -- exclusion tests --

#[test]
fn collect_tokei_excludes_target_and_git() {
    let dir = tempfile::tempdir().expect("tempdir");
    // Real source file
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(dir.path().join("src/lib.rs"), "fn a() {}\n").expect("write src");

    // Build/VCS dirs that should be excluded
    for excluded in &["target", ".git", "node_modules", ".venv"] {
        let p = dir.path().join(excluded);
        std::fs::create_dir_all(&p).expect("mkdir excluded");
        std::fs::write(p.join("noise.rs"), "fn b() {}\nfn c() {}\nfn d() {}\n")
            .expect("write noise");
    }

    let value = super::collect_tokei(dir.path()).expect("collect");
    let arr = value.as_array().expect("array");

    let files: Vec<String> = arr
        .iter()
        .map(|v| v["file"].as_str().unwrap_or("").to_string())
        .collect();
    assert!(files.iter().any(|f| f.ends_with("src/lib.rs")));
    for excluded in &["target", ".git", "node_modules", ".venv"] {
        assert!(
            !files.iter().any(|f| f.contains(excluded)),
            "expected {excluded}/ to be excluded; got files = {files:?}"
        );
    }
}

// CL-3 (TASK-1974): exclusion is anchored to direct children of the scan
// root. A directory with an excluded name deeper in the tree is real source
// and is counted.
#[test]
fn collect_tokei_counts_build_dir_nested_under_src() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src/build")).expect("mkdir src/build");
    std::fs::write(dir.path().join("src/build/gen.rs"), "fn nested() {}\n").expect("write nested");
    // Same name at the root: pruned.
    std::fs::create_dir_all(dir.path().join("build")).expect("mkdir build");
    std::fs::write(dir.path().join("build/out.rs"), "fn artifact() {}\n").expect("write artifact");

    let value = super::collect_tokei(dir.path()).expect("collect");
    let files: Vec<String> = value
        .as_array()
        .expect("array")
        .iter()
        .map(|v| v["file"].as_str().unwrap_or_default().to_owned())
        .collect();

    assert_eq!(
        files.len(),
        1,
        "expected only the nested source; got {files:?}"
    );
    assert!(
        files.iter().any(|f| f.ends_with("src/build/gen.rs")),
        "nested build/ holds real source and must be counted; got {files:?}"
    );
}

// CL-3 (TASK-1974): only *directories* are pruned. A plain file whose name
// matches an entry is source like any other.
#[test]
fn collect_tokei_counts_file_named_like_an_excluded_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("build"), "#!/bin/sh\necho hi\n").expect("write script");

    let value = super::collect_tokei(dir.path()).expect("collect");
    let files: Vec<String> = value
        .as_array()
        .expect("array")
        .iter()
        .map(|v| v["file"].as_str().unwrap_or_default().to_owned())
        .collect();
    assert_eq!(files, vec!["build".to_owned()]);
}

// -- scan bound tests (SEC-33, TASK-1970) --

#[test]
fn scan_tokei_skips_files_over_the_byte_cap() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("small.rs"), "fn a() {}\n").expect("write small");
    let big = "fn b() {}\n".repeat(64);
    std::fs::write(dir.path().join("big.rs"), &big).expect("write big");

    let limits = super::ScanLimits {
        file_bytes: 32,
        ..super::ScanLimits::DEFAULT
    };
    let scan = super::scan_tokei(dir.path(), limits).expect("scan");

    assert_eq!(scan.skipped_oversize, 1);
    assert_eq!(scan.records.len(), 1);
    assert_eq!(scan.records[0]["file"], "small.rs");
    assert!(!scan.truncated);
}

#[test]
fn scan_tokei_truncates_at_the_file_cap() {
    let dir = tempfile::tempdir().expect("tempdir");
    for name in &["a.rs", "b.rs", "c.rs"] {
        std::fs::write(dir.path().join(name), "fn x() {}\n").expect("write source");
    }

    let limits = super::ScanLimits {
        files: 2,
        ..super::ScanLimits::DEFAULT
    };
    let scan = super::scan_tokei(dir.path(), limits).expect("scan");

    assert_eq!(scan.records.len(), 2, "cap must bound what is materialised");
    assert!(scan.truncated, "a truncated result must say so");
}

/// The file cap counts **candidates**, not directory entries: a file tokei has
/// no language for is never opened, counted, or materialised, so it must not
/// consume the budget. Counting every regular file instead would make an
/// asset-heavy but ordinary repository — images, fixtures, data — truncate
/// before reaching any source at all, turning a correct statistic into a
/// warned-but-wrong one. The cap bounds the expensive half (read + count),
/// which is what `ScanLimits::files` documents.
#[test]
fn unsupported_files_do_not_consume_the_file_cap() {
    let dir = tempfile::tempdir().expect("tempdir");
    for name in &["a.png", "b.png", "c.png", "d.bin", "e.bin"] {
        std::fs::write(dir.path().join(name), "not source\n").expect("write asset");
    }
    std::fs::write(dir.path().join("only.rs"), "fn x() {}\n").expect("write source");

    let limits = super::ScanLimits {
        files: 2,
        ..super::ScanLimits::DEFAULT
    };
    let scan = super::scan_tokei(dir.path(), limits).expect("scan");

    assert_eq!(
        scan.records.len(),
        1,
        "the source file must be reached despite five unsupported files: {:?}",
        scan.records
    );
    assert_eq!(scan.records[0]["file"], "only.rs");
    assert!(
        !scan.truncated,
        "one candidate against a cap of two is not a truncated scan"
    );
}

#[test]
fn scan_tokei_honours_the_depth_cap() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("top.rs"), "fn top() {}\n").expect("write top");
    std::fs::create_dir_all(dir.path().join("a/b")).expect("mkdir nested");
    std::fs::write(dir.path().join("a/b/deep.rs"), "fn deep() {}\n").expect("write deep");

    let limits = super::ScanLimits {
        depth: 1,
        ..super::ScanLimits::DEFAULT
    };
    let scan = super::scan_tokei(dir.path(), limits).expect("scan");

    assert_eq!(scan.records.len(), 1);
    assert_eq!(scan.records[0]["file"], "top.rs");
}

// -- scan root error tests (ERR-2, TASK-1972) --

#[test]
fn collect_tokei_errors_on_missing_directory() {
    let dir = tempfile::tempdir().expect("tempdir");
    let missing = dir.path().join("does-not-exist");
    let err = super::collect_tokei(&missing)
        .expect_err("a nonexistent root must not read as an empty project");
    assert!(
        err.to_string().contains("cannot read scan root"),
        "error should name the failure, got: {err}"
    );
}

#[test]
fn collect_tokei_errors_when_root_is_a_file() {
    let dir = tempfile::tempdir().expect("tempdir");
    let file = dir.path().join("lib.rs");
    std::fs::write(&file, "fn a() {}\n").expect("write source");
    let err = super::collect_tokei(&file).expect_err("a file root is not a scannable directory");
    assert!(
        err.to_string().contains("is not a directory"),
        "error should name the failure, got: {err}"
    );
}

/// ERR-2 (TASK-1972): a file tokei cannot open is counted, not silently
/// dropped from the statistics.
#[cfg(unix)]
#[test]
fn scan_tokei_counts_unreadable_files() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let secret = dir.path().join("secret.rs");
    std::fs::write(&secret, "fn a() {}\n").expect("write source");
    std::fs::set_permissions(&secret, std::fs::Permissions::from_mode(0o000)).expect("chmod");

    // Root ignores the mode bits; there is nothing to assert in that case.
    if std::fs::File::open(&secret).is_ok() {
        return;
    }

    let scan = super::scan_tokei(dir.path(), super::ScanLimits::DEFAULT).expect("scan");
    assert!(scan.records.is_empty());
    assert_eq!(
        scan.skipped_unreadable, 1,
        "an unreadable file must be counted, not silently dropped"
    );
}

#[test]
fn tokei_default_excluded_contains_expected_dirs() {
    let exc: std::collections::HashSet<&str> =
        super::TOKEI_DEFAULT_EXCLUDED.iter().copied().collect();
    for needed in &["target", ".git", "node_modules", ".venv"] {
        assert!(exc.contains(needed), "expected {needed} in defaults");
    }
}

// -- per-report transformation tests --

#[test]
fn report_to_json_strips_workspace_prefix() {
    let report = tokei::Report::new(std::path::PathBuf::from("/ws/root/src/lib.rs"));
    let value = super::report_to_json("Rust", &report, std::path::Path::new("/ws/root"));
    assert_eq!(value["language"], "Rust");
    assert_eq!(value["file"], "src/lib.rs");
}

#[test]
fn report_to_json_keeps_full_path_when_prefix_does_not_match() {
    let report = tokei::Report::new(std::path::PathBuf::from("/elsewhere/file.rs"));
    let value = super::report_to_json("Rust", &report, std::path::Path::new("/ws/root"));
    assert_eq!(value["file"], "/elsewhere/file.rs");
}

#[test]
fn report_to_json_includes_stats_fields() {
    let mut report = tokei::Report::new(std::path::PathBuf::from("/ws/x.rs"));
    report.stats.code = 10;
    report.stats.comments = 3;
    report.stats.blanks = 2;
    let value = super::report_to_json("Rust", &report, std::path::Path::new("/ws"));
    assert_eq!(value["code"], 10);
    assert_eq!(value["comments"], 3);
    assert_eq!(value["blanks"], 2);
    assert_eq!(value["lines"], 15);
}

// -- flatten_tokei_to_json tests --

#[test]
fn flatten_tokei_empty_languages() {
    let languages = Languages::new();
    let result =
        serde_json::Value::Array(flatten_tokei_records(&languages, Path::new("/workspace")));
    assert!(result.is_array());
    assert!(result.as_array().unwrap().is_empty());
}

#[test]
#[ignore = "scans CARGO_MANIFEST_DIR; non-deterministic and slow (TEST-17)"]
fn flatten_tokei_real_project_structure() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut languages = Languages::new();
    let config = TokeiConfig::default();
    let excluded: &[&str] = &[];
    languages.get_statistics(&[&manifest_dir], excluded, &config);

    let result = serde_json::Value::Array(flatten_tokei_records(&languages, &manifest_dir));
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
#[ignore = "scans CARGO_MANIFEST_DIR; non-deterministic and slow (TEST-17)"]
fn flatten_tokei_strips_workspace_prefix() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut languages = Languages::new();
    let config = TokeiConfig::default();
    let excluded: &[&str] = &[];
    languages.get_statistics(&[&manifest_dir], excluded, &config);

    let result = serde_json::Value::Array(flatten_tokei_records(&languages, &manifest_dir));
    let arr = result.as_array().unwrap();

    for record in arr {
        let file = record["file"].as_str().unwrap();
        assert!(
            !file.starts_with(manifest_dir.to_str().unwrap()),
            "file path should be relative, got: {file}"
        );
    }
}

// -- collect_tokei tests --

#[test]
#[ignore = "scans CARGO_MANIFEST_DIR; non-deterministic and slow (TEST-17)"]
fn collect_tokei_on_real_project() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let result = collect_tokei(&manifest_dir).expect("collect_tokei should succeed");
    assert!(result.is_array());
    assert!(!result.as_array().unwrap().is_empty());
}

// The counterpart to `collect_tokei_errors_on_missing_directory`: a readable
// but genuinely empty directory is an empty *success*, so a caller can tell
// the two apart (ERR-2, TASK-1972).
#[test]
fn collect_tokei_on_empty_dir() {
    let dir = tempfile::tempdir().expect("tempdir");
    let result = collect_tokei(dir.path()).expect("collect_tokei should succeed on empty dir");
    assert!(result.is_array());
    assert!(result.as_array().unwrap().is_empty());
}

// -- DuckDB integration tests --

#[test]
#[ignore = "scans CARGO_MANIFEST_DIR; non-deterministic and slow (TEST-17)"]
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
    let load_result = ingestor
        .load(data_dir.path(), &db)
        .expect("load should succeed");
    assert!(load_result.record_count > 0);

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
    drop(conn);
    assert!(lang_count > 0, "should have language aggregations");

    // Verify staged files cleaned up
    assert!(!data_dir.path().join("tokei_files.json").exists());
}

#[test]
fn tokei_files_has_data_returns_false_for_empty_db() {
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init schema");
    let has = ops_duckdb::sql::table_has_data(&db, "tokei_files").expect("check");
    assert!(!has, "empty db should have no tokei data");
}

// -- TokeiIngestor::load tests --
//
// `load_tokei` was removed (DUP-1, TASK-0226): it duplicated the
// `TokeiIngestor::load` path for no benefit. These tests now exercise the
// ingestor directly, which is the single supported entry point.

#[test]
fn ingestor_load_errors_when_json_missing() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init schema");
    let ingestor = TokeiIngestor;
    let err = ingestor.load(data_dir.path(), &db).unwrap_err();
    let msg = err.to_string().to_lowercase();
    assert!(
        msg.contains("not found") || msg.contains("no such file") || msg.contains("os error 2"),
        "expected missing-file error, got: {err}"
    );
}

#[test]
fn load_tokei_succeeds_after_collect() {
    let project = fixture_project();
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init schema");

    // Collect data first
    let ctx = Context::test_context(project.path().to_path_buf());
    let ingestor = TokeiIngestor;
    ingestor
        .collect(&ctx, data_dir.path())
        .expect("collect should succeed");

    let load_result = ingestor
        .load(data_dir.path(), &db)
        .expect("ingestor.load should succeed");
    assert_eq!(
        load_result.record_count,
        u64::try_from(FIXTURE_FILE_COUNT).expect("fits")
    );

    let conn = db.lock().expect("lock");
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM tokei_files", [], |row| row.get(0))
        .expect("count query");
    drop(conn);
    assert_eq!(count, i64::try_from(FIXTURE_FILE_COUNT).expect("fits"));
}

// TEST-1 (TASK-1978): the single-entry-point invariant used to live here as a
// test with a fully commented-out body, which could never fail. It is now a
// `compile_fail` doctest on the crate root, which actually breaks when
// `load_tokei` is reintroduced.

// -- query_tokei_files tests --

#[test]
fn query_tokei_files_returns_json_array() {
    let project = fixture_project();
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    let ctx = Context::test_context(project.path().to_path_buf());
    let ingestor = TokeiIngestor;
    ingestor.collect(&ctx, data_dir.path()).expect("collect");
    let _ = ingestor.load(data_dir.path(), &db).expect("load");

    let result = query_tokei_files(&db).expect("query should succeed");
    let arr = result.as_array().expect("should be array");
    assert_eq!(arr.len(), FIXTURE_FILE_COUNT);

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
    // When workspace_root is not a prefix of the file path, the full path is kept.
    let project = fixture_project();
    let mut languages = Languages::new();
    let config = TokeiConfig::default();
    let excluded: &[&str] = &[];
    languages.get_statistics(&[project.path()], excluded, &config);

    // Use a root that shares no prefix with the fixture. Built from the
    // platform's own root component so the test carries no unix-only path
    // assumption (TEST-18, TASK-1977).
    let unrelated_root = std::env::temp_dir().join("ops-tokei-unrelated-root");
    let result = serde_json::Value::Array(flatten_tokei_records(&languages, &unrelated_root));
    let arr = result.as_array().unwrap();
    assert_eq!(arr.len(), FIXTURE_FILE_COUNT);

    // Files should retain full absolute paths since the prefix does not match.
    for record in arr {
        let file = record["file"].as_str().unwrap();
        assert!(
            Path::new(file).is_absolute(),
            "file should retain its absolute path when prefix doesn't match, got: {file}"
        );
        assert!(
            Path::new(file).starts_with(project.path()),
            "file should still point into the fixture, got: {file}"
        );
    }
}

// -- views tests --

#[test]
fn tokei_files_create_sql_with_real_json() {
    let project = fixture_project();
    let data_dir = tempfile::tempdir().expect("tempdir");

    // Collect from the canned fixture to get a valid JSON file. The
    // assertions concern only the generated SQL, so there is nothing here a
    // workspace scan would add (TEST-18, TASK-1977).
    let ctx = Context::test_context(project.path().to_path_buf());
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
    let project = fixture_project();
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    let ctx = Context::test_context(project.path().to_path_buf());
    let ingestor = TokeiIngestor;
    ingestor.collect(&ctx, data_dir.path()).expect("collect");
    let _ = ingestor.load(data_dir.path(), &db).expect("load");

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
    assert_eq!(files_total, FIXTURE_TOTAL_CODE);

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
    drop(conn);
    assert_eq!(
        distinct_langs, view_langs,
        "view should have one row per language"
    );
    assert_eq!(distinct_langs, FIXTURE_LANGUAGE_COUNT);
}

/// READ-5 (TASK-0504): pin the lossy contract — non-UTF-8 bytes in a
/// relative path round-trip as `U+FFFD`. Avoids silent regressions if the
/// `to_string_lossy` is later "fixed" to a strict path policy without
/// updating the surrounding caller chain.
#[cfg(unix)]
#[test]
fn relativize_path_replaces_invalid_utf8_with_replacement_char() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::path::PathBuf;

    let root = PathBuf::from("/ws");
    let invalid = OsStr::from_bytes(b"/ws/bad\xFFname");
    let path = PathBuf::from(invalid);
    let rendered = super::relativize_path(&path, &root);
    assert!(
        rendered.contains('\u{FFFD}'),
        "expected lossy U+FFFD substitution, got {rendered:?}"
    );
    assert!(
        rendered.starts_with("bad") && rendered.ends_with("name"),
        "stripped + lossy result: {rendered:?}"
    );
}
