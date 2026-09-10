//! Tests for the column-offset table parser and the compatible/breaking
//! split. Kept apart from `exit_code_tests` so neither file grows past the
//! workspace's per-file size threshold.

use super::*;

// -- Upgrade table parser tests --

#[test]
fn parse_upgrade_table_basic() {
    let stdout = "\
name   old req compatible latest  new req
====   ======= ========== ======  =======
serde  1.0.100 1.0.228    1.0.228 1.0.228
tokio  1.35.0  1.38.0     1.38.0  1.38.0
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "serde");
    assert_eq!(entries[0].old_req, "1.0.100");
    assert_eq!(entries[0].new_req, "1.0.228");
    assert!(entries[0].note.is_none());
    assert_eq!(entries[1].name, "tokio");
}

/// A data row containing multi-byte UTF-8 (localised note text, non-ASCII
/// metadata) can put a separator-row byte offset mid-codepoint. Slicing
/// clamps to the nearest char boundary, so such a row either parses cleanly
/// or is dropped — never panics.
#[test]
fn parse_upgrade_table_non_ascii_row_does_not_panic() {
    // The note column contains a 4-byte char ("📦") that crosses the
    // separator's note-column boundary.
    let stdout = "\
name   old req compatible latest  new req note
====   ======= ========== ======  ======= ====
serde  1.0.100 1.0.228    1.0.228 1.0.228 📦📦📦📦📦
";
    let entries = parse_upgrade_table(stdout);
    // Must not panic; the basic 5 fields stay aligned (ASCII columns) and
    // the note clamps to the nearest char boundary.
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "serde");
}

#[test]
fn parse_upgrade_table_with_notes() {
    let stdout = "\
name   old req compatible latest  new req note
====   ======= ========== ======  ======= ====
clap   3.0.0   3.2.25     4.6.0   3.2.25  incompatible
serde  1.0.100 1.0.228    1.0.228 1.0.228
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "clap");
    assert_eq!(entries[0].note.as_deref(), Some("incompatible"));
    assert!(entries[1].note.is_none());
}

#[test]
fn parse_upgrade_table_empty() {
    let stdout = "";
    let entries = parse_upgrade_table(stdout);
    assert!(entries.is_empty());
}

#[test]
fn parse_upgrade_table_no_data_rows() {
    let stdout = "\
name   old req compatible latest  new req
====   ======= ========== ======  =======
";
    let entries = parse_upgrade_table(stdout);
    assert!(entries.is_empty());
}

/// cargo-edit's table format is not a stable API; if a
/// future release re-renders the header with different capitalisation
/// (`Name`, `Old Req`, `New Req`), the parser must still recognise it
/// rather than silently returning an empty Vec. The `====` separator row
/// is what actually drives column alignment, so as long as we recognise
/// the header AND find a separator we should produce entries.
#[test]
fn parse_upgrade_table_header_case_insensitive() {
    let stdout = "\
Name   Old Req Compatible Latest  New Req
====   ======= ========== ======  =======
serde  1.0.100 1.0.228    1.0.228 1.0.228
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(
        entries.len(),
        1,
        "case-flipped header must still be recognised; got: {entries:?}"
    );
    assert_eq!(entries[0].name, "serde");
}

/// When stdout carries body content but no `====` separator row is detected
/// (the header drifted hard enough that columns cannot be lined up), the
/// parser emits a `tracing::warn` breadcrumb. This pins only the breadcrumb;
/// the *fail-closed* contract lives one level up, in
/// `exit_code_tests::interpret_upgrade_output_bails_on_missing_separator`.
#[test]
fn parse_upgrade_table_warns_on_missing_separator() {
    // Hypothetical drifted format: no `====` row and an unrecognised header.
    let stdout = "\
Paquete  Versión actual  Última
serde    1.0.100         1.0.228
tokio    1.35.0          1.38.0
";

    let (logged, entries) =
        crate::test_support::capture_tracing(tracing::Level::WARN, || parse_upgrade_table(stdout));
    assert!(
        entries.is_empty(),
        "no column geometry means no entries can be sliced"
    );
    assert!(
        logged.contains("TASK-1026") && logged.contains("separator"),
        "expected a TASK-1026 separator-drift warn; got: {logged}"
    );
}

#[test]
fn categorize_upgrades_splits_correctly() {
    let entries = vec![
        UpgradeEntry {
            name: "serde".into(),
            old_req: "1.0.100".into(),
            compatible: "1.0.228".into(),
            latest: "1.0.228".into(),
            new_req: "1.0.228".into(),
            note: None,
        },
        UpgradeEntry {
            name: "clap".into(),
            old_req: "3.0.0".into(),
            compatible: "3.2.25".into(),
            latest: "4.6.0".into(),
            new_req: "3.2.25".into(),
            note: Some("incompatible".into()),
        },
    ];
    let result = categorize_upgrades(entries);
    assert_eq!(result.compatible.len(), 1);
    assert_eq!(result.compatible[0].name, "serde");
    assert_eq!(result.incompatible.len(), 1);
    assert_eq!(result.incompatible[0].name, "clap");
}

/// Any note text containing "incompatible" (case-insensitive)
/// classifies as incompatible. Guards against future cargo-edit wording
/// drift like "incompatible (semver bump)" silently flipping breaking
/// upgrades into the compatible bucket.
#[test]
fn categorize_upgrades_matches_incompatible_substring() {
    let mk = |name: &str, note: Option<&str>| UpgradeEntry {
        name: name.into(),
        old_req: String::new(),
        compatible: String::new(),
        latest: String::new(),
        new_req: String::new(),
        note: note.map(str::to_string),
    };
    let entries = vec![
        mk("a", Some("incompatible")),
        mk("b", Some("Incompatible (semver bump)")),
        mk("c", Some("INCOMPATIBLE")),
        mk("d", Some("pinned by parent")),
        mk("e", None),
    ];
    let result = categorize_upgrades(entries);
    let inc_names: Vec<_> = result
        .incompatible
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    let cmp_names: Vec<_> = result.compatible.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(inc_names, vec!["a", "b", "c"]);
    assert_eq!(cmp_names, vec!["d", "e"]);
}

/// The canonical cases for the allocation-free `contains_ascii_ci` scan
/// behind the classification: a fully upper-case match, an embedded match
/// after additional words, and a non-match that shares a prefix
/// ("compatible" vs. "incompatible"). A rewrite of the helper cannot
/// silently flip classification without failing here.
#[test]
fn categorize_upgrades_perf3_parity_after_alloc_free_scan() {
    let mk = |name: &str, note: Option<&str>| UpgradeEntry {
        name: name.into(),
        old_req: String::new(),
        compatible: String::new(),
        latest: String::new(),
        new_req: String::new(),
        note: note.map(str::to_string),
    };
    let entries = vec![
        mk("upper", Some("INCOMPATIBLE")),
        mk("embedded", Some("semver incompatible")),
        mk("compat_only", Some("compatible")),
    ];
    let result = categorize_upgrades(entries);
    let inc_names: Vec<_> = result
        .incompatible
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    let cmp_names: Vec<_> = result.compatible.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(inc_names, vec!["upper", "embedded"]);
    assert_eq!(cmp_names, vec!["compat_only"]);
}

// -- Upgrade table parser edge cases --

#[test]
fn parse_upgrade_table_lines_before_header_ignored() {
    let stdout = "\
some preamble text
another line
name   old req compatible latest  new req
====   ======= ========== ======  =======
serde  1.0.100 1.0.228    1.0.228 1.0.228
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "serde");
}

#[test]
fn parse_upgrade_table_row_too_few_columns_skipped() {
    let stdout = "\
name   old req compatible latest  new req
====   ======= ========== ======  =======
serde  1.0.100 1.0.228    1.0.228 1.0.228
bad    1.0.0   1.0.1
tokio  1.35.0  1.38.0     1.38.0  1.38.0
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "serde");
    assert_eq!(entries[1].name, "tokio");
}

/// cargo-edit sizes the `=` run to the *header token's* length, so `new req`
/// gets a 7-wide separator while values like `1.10.100` are 8 chars. The
/// last fixed column reads to the end of the data row rather than being
/// clamped to the separator's length, which would decode `1.10.100` as
/// `1.10.10` — a version that does not exist, printed as an ordinary answer
/// and persisted into the cached `DepsReport`.
#[test]
fn parse_upgrade_table_last_column_wider_than_its_header_token() {
    let stdout = "\
name   old req compatible latest  new req
====   ======= ========== ======  =======
serde  1.0.100 1.0.228    1.0.228 1.10.100
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 1);
    assert_eq!(
        entries[0].new_req, "1.10.100",
        "the last fixed column must read to the end of the data row"
    );
    assert!(entries[0].note.is_none());
}

/// With a `note` column present, `new req` is an *interior* column and still
/// stops at the note's start: the end-of-row rule for the last fixed column
/// must not let it swallow the note text.
#[test]
fn parse_upgrade_table_note_column_bounds_the_new_req_column() {
    let stdout = "\
name   old req compatible latest  new req note
====   ======= ========== ======  ======= ====
clap   3.0.0   3.2.25     4.6.0   3.2.25  pinned by parent
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].new_req, "3.2.25");
    assert_eq!(entries[0].note.as_deref(), Some("pinned by parent"));
}

#[test]
fn parse_upgrade_table_multi_word_note() {
    let stdout = "\
name   old req compatible latest  new req note
====   ======= ========== ======  ======= ====
clap   3.0.0   3.2.25     4.6.0   3.2.25  pinned by user
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].note.as_deref(), Some("pinned by user"));
}

#[test]
fn classify_header() {
    assert!(matches!(
        classify_upgrade_line("name   old req compatible latest  new req"),
        UpgradeLine::Header
    ));
    assert!(matches!(
        classify_upgrade_line("Name   Old Req Compatible Latest  New Req"),
        UpgradeLine::Header
    ));
}

#[test]
fn classify_separator() {
    assert!(matches!(
        classify_upgrade_line("====   ======= ========== ======  ======="),
        UpgradeLine::Separator
    ));
}

#[test]
fn classify_body() {
    assert!(matches!(
        classify_upgrade_line("serde  1.0.100 1.0.228    1.0.228 1.0.228"),
        UpgradeLine::Body
    ));
    assert!(matches!(
        classify_upgrade_line("Updating crates.io index"),
        UpgradeLine::Body
    ));
}
