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
use crate::counter::{count_source, region_from_path, LineKind, Locs, Region, MAX_NESTING_DEPTH};

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

/// Pins the [`MAX_NESTING_DEPTH`] bail-out (SEC-33). The input below
/// lexes cleanly — `proc_macro2`'s lexer is iterative — so without the
/// cap the recursive token walkers would overflow the stack, which
/// aborts the test process with `SIGSEGV` rather than failing an
/// assertion.
#[test]
fn nesting_past_the_depth_cap_degrades_instead_of_overflowing() {
    let depth = MAX_NESTING_DEPTH * 8;
    let src = format!(
        "fn f() {{\n    let _ = {}{};\n}}\n",
        "(".repeat(depth),
        ")".repeat(depth)
    );

    let counts = count_source(&src, Region::Main);

    assert_eq!(counts.main.lines(), 3, "every line still counted");
    assert_eq!(
        counts.main.code, 3,
        "the fallback counts each non-blank line as code"
    );
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

    let value = crate::collect_rust_loc(dir.path(), None).expect("collect should succeed");
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

/// `collect_rust_loc` walks in parallel, so workers finish in arbitrary
/// order; it sorts before returning. Pins that the emitted rows are
/// deterministic rather than schedule-dependent.
#[test]
fn collect_rust_loc_returns_rows_sorted_by_file_and_region() {
    let dir = tempfile::tempdir().expect("tempdir");
    write_fixture_tree(dir.path());

    let value = crate::collect_rust_loc(dir.path(), None).expect("collect");
    let keys: Vec<(String, String)> = value
        .as_array()
        .expect("array")
        .iter()
        .map(|r| {
            (
                r["file"].as_str().unwrap_or_default().to_string(),
                r["region"].as_str().unwrap_or_default().to_string(),
            )
        })
        .collect();

    let mut sorted = keys.clone();
    sorted.sort();
    assert_eq!(keys, sorted, "rows must come back in (file, region) order");
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

    let value = crate::collect_rust_loc(&root, None).expect("collect");
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

    let value = crate::collect_rust_loc(dir.path(), None).expect("collect");
    let files: Vec<&str> = value
        .as_array()
        .expect("array")
        .iter()
        .map(|r| r["file"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(files, vec!["src/lib.rs"], "build dirs must be pruned");
}

/// Pins the depth-agnostic half of the [`crate::EXCLUDED_DIRS`] policy
/// (CL-3, TASK-2016): unlike tokei's root-anchored `TOKEI_DEFAULT_EXCLUDED`,
/// an excluded name is pruned wherever it appears below the root, because a
/// nested `target/` is a nested cargo workspace's build directory and a
/// nested `.git/` a submodule's.
#[test]
fn excluded_directories_are_pruned_below_the_scan_root_too() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn f() {}\n").expect("write src");
    for excluded in crate::EXCLUDED_DIRS {
        // Two levels down, so neither depth 0 nor depth 1 can explain the prune.
        let path = dir.path().join("nested/crate-a").join(excluded);
        std::fs::create_dir_all(&path).expect("mkdir nested excluded");
        std::fs::write(path.join("noise.rs"), "fn b() {}\nfn c() {}\n").expect("write noise");
    }

    let value = crate::collect_rust_loc(dir.path(), None).expect("collect");
    let files: Vec<&str> = value
        .as_array()
        .expect("array")
        .iter()
        .map(|r| r["file"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        files,
        vec!["src/lib.rs"],
        "excluded names must be pruned at any depth, not only at the root"
    );
}

/// Pins the `MAX_SOURCE_BYTES` gate (SEC-33): a file past the cap is
/// counted by the streaming blank-vs-non-blank fallback instead of being
/// read, lexed and `syn`-parsed at full size, and its presence never
/// stops the rest of the scan from being emitted.
#[test]
fn oversized_file_degrades_to_a_line_count_without_aborting_the_scan() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn f() {}\n").expect("write lib.rs");

    let filler = "// filler filler filler filler filler filler filler filler\n";
    let cap = usize::try_from(crate::MAX_SOURCE_BYTES).expect("cap fits in usize");
    let repeats = cap / filler.len() + 2;
    std::fs::write(dir.path().join("big.rs"), filler.repeat(repeats)).expect("write big.rs");

    let value = crate::collect_rust_loc(dir.path(), None).expect("collect");
    let records = value.as_array().expect("array");

    assert!(
        records.iter().any(|r| r["file"] == "src/lib.rs"),
        "the normal file's rows must still be emitted: {records:?}"
    );
    let big: Vec<_> = records.iter().filter(|r| r["file"] == "big.rs").collect();
    assert_eq!(big.len(), 1, "one degraded row for the over-cap file");
    assert_eq!(big[0]["region"], "main");
    assert_eq!(
        big[0]["code"].as_u64(),
        u64::try_from(repeats).ok(),
        "the fallback counts every non-blank line as code"
    );
    assert_eq!(
        big[0]["comments"].as_u64(),
        Some(0),
        "the degraded count does no comment classification"
    );
}

/// The streaming fallback must stay bounded by its reader's buffer, not by
/// the longest line in the file. `read_until(b'\n')` accumulated a whole line
/// into one `Vec`, so a single over-cap line — a generated table, a minified
/// blob, any machine-written `.rs` with no newlines — reproduced in memory
/// exactly the allocation `MAX_SOURCE_BYTES` exists to prevent, on the very
/// path chosen to avoid it. One line past the cap, and no newline at all.
#[test]
fn a_single_line_larger_than_the_cap_is_counted_without_buffering_it() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn f() {}\n").expect("write lib.rs");

    let cap = usize::try_from(crate::MAX_SOURCE_BYTES).expect("cap fits in usize");
    let one_long_line = "x".repeat(cap.saturating_add(1024));
    std::fs::write(dir.path().join("wide.rs"), &one_long_line).expect("write wide.rs");

    let value = crate::collect_rust_loc(dir.path(), None).expect("collect");
    let records = value.as_array().expect("array");

    assert!(
        records.iter().any(|r| r["file"] == "src/lib.rs"),
        "the normal file's rows must still be emitted: {records:?}"
    );
    let wide: Vec<_> = records.iter().filter(|r| r["file"] == "wide.rs").collect();
    assert_eq!(wide.len(), 1, "one degraded row for the over-cap file");
    assert_eq!(
        wide[0]["code"].as_u64(),
        Some(1),
        "an unterminated final line counts once"
    );
    assert_eq!(wide[0]["blanks"].as_u64(), Some(0));
}

/// Blank-vs-non-blank state has to survive a chunk boundary now that the
/// scan works in fixed-size reads: a run of blank lines longer than the
/// reader's buffer must not be reclassified as code, and a line whose only
/// non-blank byte lands in a later chunk must not stay blank.
#[test]
fn blank_state_survives_chunk_boundaries() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cap = usize::try_from(crate::MAX_SOURCE_BYTES).expect("cap fits in usize");

    // Blank lines well past any buffer size, then one very long line that is
    // blank until its final byte, then a normal line.
    let mut src = "\n".repeat(cap / 8);
    src.push_str(&" ".repeat(cap));
    src.push_str("x\n");
    src.push('\n');
    std::fs::write(dir.path().join("chunks.rs"), &src).expect("write chunks.rs");

    let value = crate::collect_rust_loc(dir.path(), None).expect("collect");
    let records = value.as_array().expect("array");
    let row = records
        .iter()
        .find(|r| r["file"] == "chunks.rs")
        .expect("chunks.rs row");

    assert_eq!(
        row["code"].as_u64(),
        Some(1),
        "only the line carrying a non-blank byte is code"
    );
    assert_eq!(
        row["blanks"].as_u64(),
        u64::try_from(cap / 8 + 1).ok(),
        "every blank line, across chunk boundaries, stays blank"
    );
}

// -- degradation policy (lib.rs:120-126, lib.rs:135-144, counter.rs syn pass) --

/// Pins the **unreadable-file** branch of the warn-and-skip policy: a
/// `.rs` file holding invalid UTF-8 makes `read_to_string` fail with
/// `InvalidData` for every user, root included, so the walk must warn,
/// skip it, and still return the valid file's rows.
#[test]
fn unreadable_file_is_skipped_and_the_rest_of_the_scan_survives() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn f() {}\n").expect("write lib.rs");
    std::fs::write(dir.path().join("src/broken.rs"), b"fn f() {}\n\xff\xfe\n")
        .expect("write broken");

    let value = crate::collect_rust_loc(dir.path(), None).expect("collect must not error");
    let files: Vec<&str> = value
        .as_array()
        .expect("array")
        .iter()
        .map(|r| r["file"].as_str().unwrap_or_default())
        .collect();

    assert_eq!(
        files,
        vec!["src/lib.rs"],
        "the valid file's rows survive and the undecodable one is skipped"
    );
}

/// Pins the **unwalkable-path** branch of the warn-and-skip policy: a
/// subdirectory the process cannot descend produces a walker `Err`,
/// which must be warned and skipped rather than aborting the scan.
///
/// Skips itself when the process can read the directory anyway (running
/// as root), rather than passing vacuously.
#[cfg(unix)]
#[test]
fn unwalkable_subdirectory_is_skipped_and_the_rest_of_the_scan_survives() {
    use std::os::unix::fs::PermissionsExt as _;

    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn f() {}\n").expect("write lib.rs");

    let locked = dir.path().join("locked");
    std::fs::create_dir_all(&locked).expect("mkdir locked");
    std::fs::write(locked.join("hidden.rs"), "fn g() {}\n").expect("write hidden.rs");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))
        .expect("chmod locked");

    let readable_anyway = std::fs::read_dir(&locked).is_ok();
    if readable_anyway {
        // Restore before bailing so the tempdir can be cleaned up.
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))
            .expect("restore permissions");
        eprintln!("skipped: this process can read a 0o000 directory (running as root)");
        return;
    }

    let result = crate::collect_rust_loc(dir.path(), None);

    // Restore before asserting, so a failure still leaves a removable tree.
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755))
        .expect("restore permissions");

    let value = result.expect("collect must not error");
    let files: Vec<&str> = value
        .as_array()
        .expect("array")
        .iter()
        .map(|r| r["file"].as_str().unwrap_or_default())
        .collect();
    assert_eq!(
        files,
        vec!["src/lib.rs"],
        "the readable file's rows survive the unwalkable subtree"
    );
}

/// Pins the **lexes-but-does-not-parse** branch: `let x = 1;` at file
/// scope is a valid token stream but not a valid top-level item, so
/// `TokenStream::from_str` succeeds and `syn` rejects the file. The
/// whole test-attribution pass is then skipped, and every line —
/// including the `#[cfg(test)]` module that would otherwise be a test
/// gate — stays attributed to the file's base region.
///
/// Distinct from `unlexable_source_falls_back_to_blank_versus_nonblank`,
/// which covers the earlier `TokenStream::from_str` failure and its
/// different outcome (no classification at all).
#[test]
fn source_that_lexes_but_fails_syn_keeps_every_line_in_the_base_region() {
    let src = "let x = 1;\n#[cfg(test)]\nmod t {}\n";

    let counts = count_source(src, Region::Main);

    assert_eq!(counts.test, Locs::default(), "no test split without syn");
    assert_eq!(counts.example, Locs::default());
    assert_eq!(counts.main.lines(), 3, "all three lines counted as main");
    assert_eq!(
        counts.main.code, 3,
        "the lexer still classifies each line as code"
    );
}

#[test]
fn collect_rust_loc_on_empty_dir_returns_empty_array() {
    let dir = tempfile::tempdir().expect("tempdir");
    let value = crate::collect_rust_loc(dir.path(), None).expect("collect should succeed");
    assert_eq!(value.as_array().map(Vec::len), Some(0));
}

#[test]
#[ignore = "scans CARGO_MANIFEST_DIR; non-deterministic and slow (TEST-17)"]
fn collect_rust_loc_returns_records_for_this_crate() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let value = crate::collect_rust_loc(&manifest_dir, None).expect("collect should succeed");
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
    let workspace = tempfile::tempdir().expect("tempdir");
    write_fixture_tree(workspace.path());
    let data_dir = tempfile::tempdir().expect("data tempdir");
    // SEC-25 / TASK-2054: ingestors stage through a verified anchor, so the
    // test drives the same handle `provide_via_ingestor` builds.
    let dir =
        ops_duckdb::IngestDir::open(&data_dir.path().join("ingest")).expect("open ingest dir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    let ctx = Context::test_context(workspace.path().to_path_buf());
    RustLocIngestor
        .collect(&ctx, &dir)
        .expect("collect should succeed");
    assert!(dir.entry_path("rust_loc_files.json").exists());

    let load_result = RustLocIngestor
        .load(&dir, &db)
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
    drop(conn);
    assert!(regions >= 2, "fixture spans main, test and example");
}

/// Cross-crate contract: `ops about loc` reads this crate's summary view
/// through [`ops_duckdb::sql::query_rust_loc_summary`], which selects the
/// columns by name. The view SQL lives here and the SELECT lives in
/// `ops-duckdb`, so a rename on either side would otherwise only fail at
/// runtime, on a real workspace, as an empty about page.
#[test]
fn rust_loc_summary_view_satisfies_the_shared_summary_query() {
    let workspace = tempfile::tempdir().expect("tempdir");
    write_fixture_tree(workspace.path());
    let data_dir = tempfile::tempdir().expect("data tempdir");
    // SEC-25 / TASK-2054: ingestors stage through a verified anchor, so the
    // test drives the same handle `provide_via_ingestor` builds.
    let dir =
        ops_duckdb::IngestDir::open(&data_dir.path().join("ingest")).expect("open ingest dir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");

    let ctx = Context::test_context(workspace.path().to_path_buf());
    RustLocIngestor
        .collect(&ctx, &dir)
        .expect("collect should succeed");
    let loaded = RustLocIngestor
        .load(&dir, &db)
        .expect("load should succeed");
    assert!(loaded.record_count > 0, "fixture rows must reach DuckDB");

    let stats = ops_duckdb::sql::query_rust_loc_summary(&db).expect("summary query");
    assert!(!stats.is_empty(), "ingested fixture must produce regions");

    let regions: Vec<&str> = stats.iter().map(|s| s.region.as_str()).collect();
    for expected in ["main", "test", "example"] {
        assert!(
            regions.contains(&expected),
            "fixture covers {expected}: {regions:?}"
        );
    }

    let main = stats
        .iter()
        .find(|s| s.region == "main")
        .expect("main region");
    assert!(main.code > 0, "main region has code lines: {main:?}");
    assert!(
        main.docs > 0,
        "fixture's `//! Docs.` must land in the docs column: {main:?}"
    );
    assert_eq!(
        main.lines,
        main.code + main.docs + main.comments + main.blanks,
        "region totals must reconcile: {main:?}"
    );
}

#[test]
fn rust_loc_ingestor_load_without_collect_fails() {
    let data_dir = tempfile::tempdir().expect("tempdir");
    // SEC-25 / TASK-2054: ingestors stage through a verified anchor, so the
    // test drives the same handle `provide_via_ingestor` builds.
    let dir =
        ops_duckdb::IngestDir::open(&data_dir.path().join("ingest")).expect("open ingest dir");
    let db = DuckDb::open_in_memory().expect("open in-memory db");
    init_schema(&db).expect("init schema");
    assert!(
        RustLocIngestor.load(&dir, &db).is_err(),
        "load without prior collect should fail"
    );
}

// -- SEC-33 / TASK-2052: the parallel walk honours the dispatch deadline --

/// AC #2: with a budget already spent, the provider aborts the walk instead
/// of counting the tree and being told afterwards that it was too slow.
///
/// `rust-loc` is the harder of the two walkers: its per-entry closure runs on
/// `ignore`'s worker threads, which cannot borrow the dispatch's `&mut
/// Context`, so this pins that the detached deadline reaches them and that the
/// resulting `Quit` is turned back into a *typed* `TimedOut` — not swallowed
/// into a silently short row set, which is the failure mode a partial-count
/// policy makes easy to miss.
#[test]
fn a_spent_budget_aborts_the_rust_loc_walk_with_a_typed_timeout() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(dir.path().join("src/lib.rs"), "fn a() {}\n").expect("write");

    let mut registry = ops_extension::DataRegistry::new();
    let _ = registry.register(crate::DATA_PROVIDER_NAME, Box::new(RustLocProvider));

    let mut ctx = Context::test_context(dir.path().to_path_buf())
        .with_provider_budget(Some(std::time::Duration::from_nanos(1)));
    match registry.provide(crate::DATA_PROVIDER_NAME, &mut ctx) {
        Err(ops_extension::DataProviderError::TimedOut { provider, .. }) => {
            assert_eq!(provider, crate::DATA_PROVIDER_NAME);
        }
        other => panic!("expected a typed TimedOut from the walk, got {other:?}"),
    }

    let mut ctx = Context::test_context(dir.path().to_path_buf());
    let value = registry
        .provide(crate::DATA_PROVIDER_NAME, &mut ctx)
        .expect("the same tree must scan cleanly without a spent budget");
    assert_eq!(
        value.as_array().map(Vec::len),
        Some(1),
        "the control run must produce the one file's row"
    );
}

/// A deadline that has not expired must not perturb the walk.
#[test]
fn a_live_budget_leaves_the_rust_loc_walk_intact() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("lib.rs"), "fn a() {}\n").expect("write");
    let mut registry = ops_extension::DataRegistry::new();
    let _ = registry.register(crate::DATA_PROVIDER_NAME, Box::new(RustLocProvider));
    let mut ctx = Context::test_context(dir.path().to_path_buf())
        .with_provider_budget(Some(std::time::Duration::from_secs(600)));
    let value = registry
        .provide(crate::DATA_PROVIDER_NAME, &mut ctx)
        .expect("a live budget must not fail the walk");
    assert_eq!(value.as_array().map(Vec::len), Some(1));
}

/// Hands a test a real `Deadline`, which has no public constructor: register a
/// provider whose only job is to keep the handle the dispatch installs on it.
fn spent_deadline() -> ops_extension::Deadline {
    struct Capture(std::sync::Arc<std::sync::Mutex<Option<ops_extension::Deadline>>>);
    impl DataProvider for Capture {
        fn name(&self) -> &'static str {
            "deadline-capture"
        }
        fn provide(
            &self,
            ctx: &mut Context,
        ) -> Result<serde_json::Value, ops_extension::DataProviderError> {
            *self
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner) = ctx.deadline_handle();
            Ok(serde_json::Value::Array(Vec::new()))
        }
    }

    let slot = std::sync::Arc::new(std::sync::Mutex::new(None));
    let mut registry = ops_extension::DataRegistry::new();
    let _ = registry.register("deadline-capture", Box::new(Capture(slot.clone())));
    let mut ctx = Context::test_context(PathBuf::from("."))
        .with_provider_budget(Some(std::time::Duration::from_nanos(1)));
    // The dispatch itself times out on the post-check; only the handle matters.
    let _ = registry.provide("deadline-capture", &mut ctx);
    let deadline = slot
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone()
        .expect("a bounded dispatch installs a deadline");
    assert!(deadline.is_expired(), "a 1ns budget is spent on arrival");
    deadline
}

/// SEC-33 / TASK-2052: the streaming fallback polls the deadline too. The
/// per-entry check admits a file before reading it, and an over-cap file is
/// unbounded in size, so without a poll inside the read loop one huge file
/// could scan to EOF arbitrarily long after the budget was spent.
#[test]
fn an_expired_deadline_stops_the_streaming_count() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("big.rs");
    std::fs::write(&path, "fn a() {}\n\nfn b() {}\n").expect("write big.rs");

    let deadline = spent_deadline();
    assert!(
        crate::count_streaming(&path, Region::Main, Some(&deadline))
            .expect("the read itself must not fail")
            .is_none(),
        "an expired deadline must abandon the file instead of counting it"
    );

    let counts = crate::count_streaming(&path, Region::Main, None)
        .expect("the read itself must not fail")
        .expect("an unbounded count runs to EOF");
    assert_eq!(
        counts.non_empty().collect::<Vec<_>>(),
        vec![(
            Region::Main,
            Locs {
                code: 2,
                blanks: 1,
                ..Locs::default()
            }
        )],
        "the control run counts every line"
    );
}
