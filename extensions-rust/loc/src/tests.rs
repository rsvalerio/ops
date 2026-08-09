//! Tests for the rust-loc extension.
//!
//! The cases below are chosen to pin the behaviours that are *not*
//! obvious from reading `counter.rs`: doc-comment reclassification,
//! lexer-provided string handling, and `cfg` predicate variants that a
//! textual matcher would miss.
//!
//! ## Test isolation policy (TEST-17/TEST-18)
//!
//! Mirrors the tokei extension: tests that scan the live workspace via
//! `env!("CARGO_MANIFEST_DIR")` are non-deterministic and slow, so they
//! are gated behind `#[ignore]`. Default coverage uses
//! `tempfile::tempdir()` plus canned fixture files.

use std::path::{Path, PathBuf};

use ops_duckdb::{init_schema, DataIngestor, DuckDb};
use ops_extension::{Context, DataProvider, Extension, ExtensionType};

use super::{RustLocExtension, RustLocIngestor, RustLocProvider};
use crate::counter::{count_source, region_from_path, LineKind, Locs, Region};

// -- Extension trait tests --

ops_extension::test_datasource_extension!(
    RustLocExtension,
    name: "rust-loc",
    data_provider: "rust-loc"
);

#[test]
fn rust_loc_extension_type_is_datasource() {
    assert_eq!(RustLocExtension.types(), ExtensionType::DATASOURCE);
}

/// The Rust breakdown must stay on the Rust stack; `tokei` keeps the
/// language-agnostic slot.
#[test]
fn rust_loc_extension_stack_is_rust() {
    assert_eq!(RustLocExtension.stack(), Some(ops_extension::Stack::Rust));
}

#[test]
fn rust_loc_provider_schema_has_fields() {
    let schema = RustLocProvider.schema();
    assert!(!schema.description.is_empty());
    let names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
    for expected in [
        "file", "region", "code", "docs", "comments", "blanks", "lines",
    ] {
        assert!(names.contains(&expected), "missing field {expected}");
    }
}

// -- region_from_path --

#[test]
fn region_from_path_detects_conventional_layouts() {
    assert_eq!(region_from_path(Path::new("src/lib.rs")), Region::Main);
    assert_eq!(region_from_path(Path::new("src/tests.rs")), Region::Test);
    assert_eq!(
        region_from_path(Path::new("crates/theme/src/tests/colour.rs")),
        Region::Test
    );
    assert_eq!(
        region_from_path(Path::new("crates/cli/tests/e2e.rs")),
        Region::Test
    );
    assert_eq!(
        region_from_path(Path::new("benches/parse.rs")),
        Region::Test
    );
    assert_eq!(
        region_from_path(Path::new("examples/demo.rs")),
        Region::Example
    );
}

/// A directory merely *containing* the substring must not match; only
/// whole path components count.
#[test]
fn region_from_path_requires_whole_component_match() {
    assert_eq!(
        region_from_path(Path::new("src/test_support/mod.rs")),
        Region::Main
    );
    assert_eq!(region_from_path(Path::new("src/latest.rs")), Region::Main);
}

// -- basic classification --

#[test]
fn counts_code_comments_docs_and_blanks() {
    let src = "\
/// Doc comment.
pub fn f() {
    // Ordinary comment.
    let x = 1;

}
";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.main.docs, 1);
    assert_eq!(counts.main.comments, 1);
    assert_eq!(counts.main.blanks, 1);
    assert_eq!(counts.main.code, 3, "fn, let, closing brace");
    assert_eq!(counts.main.lines(), 6);
}

/// A line holding both code and a trailing comment counts as code, the
/// convention every other counter uses.
#[test]
fn trailing_comment_counts_as_code() {
    let counts = count_source("let x = 1; // trailing\n", Region::Main);
    assert_eq!(counts.main.code, 1);
    assert_eq!(counts.main.comments, 0);
}

#[test]
fn inner_doc_comment_is_docs_not_code() {
    let counts = count_source("//! Module docs.\npub fn f() {}\n", Region::Main);
    assert_eq!(counts.main.docs, 1);
    assert_eq!(counts.main.code, 1);
}

/// A literal `#[doc = "..."]` is code the author wrote, not a comment.
/// This is the case that separates span-checking from a naive
/// "attribute named doc" rule.
#[test]
fn explicit_doc_attribute_counts_as_code() {
    let counts = count_source("#[doc = \"text\"]\npub fn f() {}\n", Region::Main);
    assert_eq!(counts.main.docs, 0);
    assert_eq!(counts.main.code, 2);
}

#[test]
fn multi_line_doc_block_is_docs() {
    let src = "/** line one\n line two */\npub fn f() {}\n";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.main.docs, 2);
    assert_eq!(counts.main.code, 1);
}

// -- lexer-provided correctness --
//
// These are the cases that motivated using `proc_macro2` instead of a
// hand-rolled scanner. Each one breaks a naive line-oriented matcher.

#[test]
fn comment_marker_inside_string_is_code() {
    let counts = count_source("let s = \"// not a comment\";\n", Region::Main);
    assert_eq!(counts.main.code, 1);
    assert_eq!(counts.main.comments, 0);
}

#[test]
fn raw_string_contents_are_code_not_comments() {
    let src = "let s = r#\"\n// still a string\n\"#;\n";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.main.code, 3);
    assert_eq!(counts.main.comments, 0);
}

#[test]
fn nested_block_comment_is_comments() {
    let src = "/* outer /* inner */ still outer */\npub fn f() {}\n";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.main.comments, 1);
    assert_eq!(counts.main.code, 1);
}

/// A comment nested inside a delimited group must survive: the group's
/// own span covers its whole body, so marking it wholesale would
/// swallow this.
#[test]
fn comment_inside_block_is_not_swallowed_by_group_span() {
    let src = "pub fn f() {\n    // inner\n    let x = 1;\n}\n";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.main.comments, 1);
    assert_eq!(counts.main.code, 3);
}

// -- test-region attribution --

#[test]
fn cfg_test_module_is_attributed_to_tests() {
    let src = "\
pub fn f() {}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {}
}
";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.main.code, 1);
    assert_eq!(counts.main.blanks, 1);
    assert_eq!(counts.test.code, 5);
}

/// The variants a string-matching scanner misses. Each of these is a
/// real test gate that must not be counted as production code.
#[test]
fn compound_cfg_predicates_are_test_gates() {
    for gate in [
        "#[cfg(all(test, unix))]",
        "#[cfg(any(test, feature = \"x\"))]",
        "#[cfg( test )]",
    ] {
        let src = format!("{gate}\nmod m {{\n    fn f() {{}}\n}}\n");
        let counts = count_source(&src, Region::Main);
        assert_eq!(
            counts.main,
            Locs::default(),
            "expected all lines attributed to tests for gate: {gate}"
        );
        assert_eq!(counts.test.code, 4, "gate: {gate}");
    }
}

/// `cfg(feature = "test")` carries `test` as a string literal, not an
/// ident, so it is production code. This is why the predicate scan
/// looks for a bare ident rather than doing a substring match.
#[test]
fn feature_named_test_is_not_a_test_gate() {
    let src = "#[cfg(feature = \"test\")]\npub fn f() {}\n";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.main.code, 2);
    assert_eq!(counts.test, Locs::default());
}

/// `#[cfg_attr(test, ..)]` applies an attribute conditionally; it never
/// removes the item, so the item ships in release builds and is
/// production code. Counting it as test misattributes the whole item —
/// a `#[cfg_attr(test, derive(Debug))]` struct is not a test.
#[test]
fn cfg_attr_is_not_a_test_gate() {
    let src = "\
#[cfg_attr(test, derive(Debug, PartialEq))]
pub struct Config {
    pub name: String,
}
";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.main.code, 4);
    assert_eq!(counts.test, Locs::default());
}

/// A `#[cfg_attr(test, ..)]` sitting *next to* a real `#[test]` is still
/// a test: the gate comes from `#[test]`, not from the `cfg_attr`.
#[test]
fn cfg_attr_beside_a_test_attribute_stays_test() {
    let src = "#[test]\n#[cfg_attr(test, ignore = \"slow\")]\nfn t() {}\n";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.test.code, 3);
    assert_eq!(counts.main, Locs::default());
}

/// `#[cfg(not(test))]` compiles the item *out* of test builds, so it is
/// production code even though the predicate mentions `test`.
#[test]
fn negated_cfg_test_is_not_a_test_gate() {
    let src = "#[cfg(not(test))]\npub fn f() {}\n";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.main.code, 2);
    assert_eq!(counts.test, Locs::default());
}

#[test]
fn file_level_region_overrides_and_suppresses_syn_pass() {
    let src = "#[test]\nfn t() {}\n";
    let counts = count_source(src, Region::Test);
    assert_eq!(counts.test.code, 2);
    assert_eq!(counts.main, Locs::default());
}

// -- degradation --

/// A file the lexer rejects must still contribute a line count rather
/// than silently vanishing from the totals.
#[test]
fn unlexable_source_falls_back_to_blank_versus_nonblank() {
    let src = "this ( is not ] valid rust\n\nstill counted\n";
    let counts = count_source(src, Region::Main);
    assert_eq!(counts.main.lines(), 3);
    assert_eq!(counts.main.blanks, 1);
}

#[test]
fn empty_source_counts_nothing() {
    assert_eq!(count_source("", Region::Main).main, Locs::default());
}

/// `count_source` drops proc-macro2's thread-local source map on entry
/// to bound memory across a whole-workspace scan. That invalidates every
/// span issued by an earlier call, so this pins the property that makes
/// it safe: counts depend only on the source passed in, never on what
/// was counted before it on the same thread.
#[test]
fn repeated_counts_on_one_thread_are_independent() {
    let first = "/// Doc.\npub fn a() {}\n";
    let second = "#[cfg(test)]\nmod t {\n    fn b() {}\n}\n";

    let baseline = count_source(first, Region::Main);
    let _ = count_source(second, Region::Main);
    let repeat = count_source(first, Region::Main);

    assert_eq!(baseline, repeat, "counts must not depend on scan order");
    assert_eq!(repeat.main.docs, 1);
    assert_eq!(repeat.main.code, 1);
}

// -- ordering invariant --

/// `mark_range` relies on `LineKind`'s ordering to implement
/// precedence. If the variants are reordered, counting silently
/// changes; this pins the invariant.
#[test]
fn line_kind_precedence_is_blank_comment_doc_code() {
    assert!(LineKind::Blank < LineKind::Comment);
    assert!(LineKind::Comment < LineKind::Doc);
    assert!(LineKind::Doc < LineKind::Code);
}

// -- collection over a real tree --

/// Canned tree: one production file, one path-convention test file, one
/// example. Deterministic, so it runs by default (TEST-17).
fn write_fixture_tree(root: &Path) {
    std::fs::create_dir_all(root.join("src")).expect("mkdir src");
    std::fs::create_dir_all(root.join("examples")).expect("mkdir examples");
    std::fs::write(
        root.join("src/lib.rs"),
        "//! Docs.\npub fn f() {}\n\n#[cfg(test)]\nmod t {\n    #[test]\n    fn x() {}\n}\n",
    )
    .expect("write lib.rs");
    std::fs::write(root.join("src/tests.rs"), "#[test]\nfn y() {}\n").expect("write tests.rs");
    std::fs::write(root.join("examples/demo.rs"), "fn main() {}\n").expect("write demo.rs");
}

#[test]
fn collect_rust_loc_splits_regions_across_a_canned_tree() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_fixture_tree(dir.path());

    let value = crate::collect_rust_loc(dir.path()).expect("collect should succeed");
    let records = value.as_array().expect("array");
    assert!(!records.is_empty());

    let regions_for = |file: &str| -> Vec<String> {
        records
            .iter()
            .filter(|r| r["file"] == file)
            .map(|r| r["region"].as_str().unwrap_or_default().to_string())
            .collect()
    };

    let lib_regions = regions_for("src/lib.rs");
    assert!(lib_regions.contains(&"main".to_string()));
    assert!(
        lib_regions.contains(&"test".to_string()),
        "the #[cfg(test)] mod should emit a test row: {lib_regions:?}"
    );
    assert_eq!(regions_for("src/tests.rs"), vec!["test".to_string()]);
    assert_eq!(regions_for("examples/demo.rs"), vec!["example".to_string()]);

    for record in records {
        let sum = record["code"].as_u64().unwrap()
            + record["docs"].as_u64().unwrap()
            + record["comments"].as_u64().unwrap()
            + record["blanks"].as_u64().unwrap();
        assert_eq!(
            record["lines"].as_u64().unwrap(),
            sum,
            "lines must be total"
        );
    }
}

/// Region attribution keys off the workspace-relative path. An absolute
/// prefix such as `/home/someone/tests/ws` must not leak a `tests`
/// component into the classification.
#[test]
fn region_uses_workspace_relative_path_not_absolute_prefix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().join("tests").join("workspace");
    std::fs::create_dir_all(root.join("src")).expect("mkdir");
    std::fs::write(root.join("src/lib.rs"), "pub fn f() {}\n").expect("write");

    let value = crate::collect_rust_loc(&root).expect("collect");
    let records = value.as_array().expect("array");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["region"], "main");
}

#[test]
fn collect_rust_loc_skips_build_directories() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn f() {}\n").expect("write src");
    for excluded in crate::EXCLUDED_DIRS {
        let path = dir.path().join(excluded);
        std::fs::create_dir_all(&path).expect("mkdir excluded");
        std::fs::write(path.join("noise.rs"), "fn b() {}\nfn c() {}\n").expect("write noise");
    }

    let value = crate::collect_rust_loc(dir.path()).expect("collect");
    let files: Vec<&str> = value
        .as_array()
        .expect("array")
        .iter()
        .map(|r| r["file"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(files, vec!["src/lib.rs"], "build dirs must be pruned");
}

#[test]
fn collect_rust_loc_on_empty_dir_returns_empty_array() {
    let dir = tempfile::tempdir().expect("tempdir");
    let value = crate::collect_rust_loc(dir.path()).expect("collect should succeed");
    assert_eq!(value.as_array().map(Vec::len), Some(0));
}

#[test]
#[ignore = "scans CARGO_MANIFEST_DIR; non-deterministic and slow (TEST-17)"]
fn collect_rust_loc_returns_records_for_this_crate() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let value = crate::collect_rust_loc(&manifest_dir).expect("collect should succeed");
    let records = value.as_array().expect("array");
    assert!(!records.is_empty(), "this crate has .rs files");
    assert!(
        records
            .iter()
            .any(|r| r.get("region").and_then(|v| v.as_str()) == Some("test")),
        "this file is itself a test region"
    );
}

// -- provider / DuckDB integration --

#[test]
fn rust_loc_provider_name() {
    assert_eq!(RustLocProvider.name(), "rust-loc");
}

#[test]
fn rust_loc_provider_returns_json_without_a_database() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_fixture_tree(dir.path());
    let mut ctx = Context::test_context(dir.path().to_path_buf());
    let value = RustLocProvider.provide(&mut ctx).expect("provide");
    assert!(value.is_array());
    assert!(!value.as_array().unwrap().is_empty());
}

#[test]
fn rust_loc_collect_and_load_cycle() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_fixture_tree(dir.path());
    let data_dir = tempfile::tempdir().expect("data tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    let ctx = Context::test_context(dir.path().to_path_buf());
    RustLocIngestor
        .collect(&ctx, data_dir.path())
        .expect("collect should succeed");
    assert!(data_dir.path().join("rust_loc_files.json").exists());

    let load_result = RustLocIngestor
        .load(data_dir.path(), &db)
        .expect("load should succeed");
    assert!(load_result.record_count > 0);

    let conn = db.lock().expect("lock");
    let files_total: i64 = conn
        .query_row("SELECT SUM(code) FROM rust_loc_files", [], |row| row.get(0))
        .expect("files sum");
    let view_total: i64 = conn
        .query_row("SELECT SUM(code) FROM rust_loc_summary", [], |row| {
            row.get(0)
        })
        .expect("view sum");
    assert_eq!(
        files_total, view_total,
        "summary view must aggregate every file row"
    );

    let regions: i64 = conn
        .query_row("SELECT COUNT(*) FROM rust_loc_summary", [], |row| {
            row.get(0)
        })
        .expect("region count");
    assert!(regions >= 2, "fixture spans main, test and example");
}

#[test]
fn rust_loc_ingestor_load_without_collect_fails() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init schema");
    assert!(
        RustLocIngestor.load(data_dir.path(), &db).is_err(),
        "load without prior collect should fail"
    );
}
