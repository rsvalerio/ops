//! Corpus invariants over COPIES of real task files from this repository's
//! own `.backlog` tree — the read-compatibility contract in executable form.
//!
//! The fixtures are snapshots, insulated from live-tree edits: a task moving
//! to `completed/` in the real tree must not break this suite. Refresh them
//! deliberately when a new shape appears in the wild that the parser should
//! accept.
//!
//! One exception to "from this tree": the Definition of Done fixture was
//! written by the backlog CLI v1.51.0 itself, because no task here had ever
//! carried that section when `ops backlog` learned to read it.

use ops_backlog::model::TaskDoc;

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");

/// The exact fixture set, pinned: a fixture deleted from the tree must fail
/// the corpus test, not silently shrink coverage. Refresh deliberately
/// alongside the fixtures when a new shape joins the corpus.
const EXPECTED_FIXTURES: &[&str] = &[
    "archive-tasks/task-0059 - TQ-5-config-merge.rs-has-zero-test-coverage-for-core-merge-logic.md",
    "completed/task-0001 - Box-leak-for-dynamic-command-strings-is-intentional-but-undocumented-lifetime.md",
    "completed/task-0059 - Improve-test-coverage-for-ops-about-crate.md",
    "completed/task-0100 - code-review-plan-wave5.md",
    "tasks/task-0005 - TEST-9-definition-of-done-shape.md",
    "tasks/task-1692 - Run-the-code-review-rust-skill-against-the-crate-ops.md",
    "tasks/task-1834 - SEC-11-provider-supplied-target-names-reach-YAML-frontmatter-and-stdout-unvalidated-and-yaml_single_quoted-cannot-encode-a-newline.md",
    "tasks/task-2039 - SEC-anchor-duckdb-ingest-staging-to-a-verified-directory-handle.md",
    "tasks/task-2069 - DUP-3-six-more-hand-rolled-tracing-capture-scaffolds-outside-the-TASK-2058-enumeration.md",
];

/// Every fixture parses, and re-rendering then re-parsing is a fixed point:
/// nothing the writer emits can confuse the reader.
#[test]
fn every_fixture_parses_and_round_trips() {
    let mut seen: Vec<String> = Vec::new();
    for dir in ["tasks", "completed", "archive-tasks"] {
        for fixture in fixture_files(dir) {
            let src = std::fs::read_to_string(&fixture).expect("read fixture");
            let doc = TaskDoc::parse(&src)
                .unwrap_or_else(|e| panic!("{} must parse: {e:#}", fixture.display()));
            let once = doc.render();
            let twice = TaskDoc::parse(&once)
                .unwrap_or_else(|e| panic!("{} re-render must parse: {e:#}", fixture.display()))
                .render();
            assert_eq!(
                once,
                twice,
                "{}: render must be a fixed point",
                fixture.display()
            );
            let name = fixture
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default();
            seen.push(format!("{dir}/{name}"));
        }
    }
    seen.sort_unstable();
    assert_eq!(
        seen,
        EXPECTED_FIXTURES
            .iter()
            .map(std::string::ToString::to_string)
            .collect::<Vec<_>>(),
        "the fixture set is pinned; refresh EXPECTED_FIXTURES deliberately"
    );
}

/// task-1834: YAML-looking text inside a fenced code block must not confuse
/// the frontmatter split — only the first `---` pair bounds it.
#[test]
fn fenced_yaml_block_is_inert() {
    let src = fixture("tasks", |n| n.starts_with("task-1834"));
    let doc = parse(&src);
    assert!(
        doc.body.description().is_some_and(|d| d.contains("```")),
        "the fenced block lives in the description, got: {:?}",
        doc.body.description().map(str::len)
    );
}

/// task-0001 (completed): the oldest files carry `:SS` seconds in
/// `created_date`; task-0059 predates `updated_date` entirely and must read
/// without it.
#[test]
fn oldest_files_keep_seconds_dates_and_missing_update() {
    let src = fixture("completed", |n| n.starts_with("task-0001 "));
    let doc = parse(&src);
    assert_eq!(
        doc.frontmatter.created_date.len(),
        "YYYY-MM-DD HH:MM:SS".len(),
        "seconds precision on the oldest created_date"
    );

    let src = fixture("completed", |n| n.starts_with("task-0059 "));
    let doc = parse(&src);
    assert!(doc.frontmatter.updated_date.is_none());
}

/// task-1692: one of the 29 frontmatter-only `Run-the-code-review…` files —
/// no body at all.
#[test]
fn frontmatter_only_file_has_empty_body() {
    let src = fixture("tasks", |n| n.starts_with("task-1692"));
    let doc = parse(&src);
    assert!(doc.body.raw.is_empty());
}

/// task-2069: block-list `modified_files`, labels, AC checkboxes.
#[test]
fn modified_files_and_ac_counts_survive() {
    let src = fixture("tasks", |n| n.starts_with("task-2069"));
    let doc = parse(&src);
    assert!(!doc.frontmatter.modified_files.is_empty());
    let ac = doc.body.ac_items();
    assert_eq!(ac.len(), 3);
    assert!(ac.iter().all(|item| item.checked));
    assert_eq!(
        doc.frontmatter.labels,
        vec!["code-review-rust", "duplication"]
    );
}

/// task-0100 (completed): a bare unquoted title — the old style ~239 files
/// carry.
#[test]
fn bare_scalar_title_parses() {
    let src = fixture("completed", |n| n.starts_with("task-0100"));
    let doc = parse(&src);
    assert_eq!(doc.frontmatter.title, "code-review-plan-wave5");
}

// Panicking helpers outside `#[test]` fns are not covered by the
// workspace's allow-in-tests clippy keys, so each helper below carries the
// narrow exception it needs (AGENTS.md: narrowest scope, reason beside it).
#[allow(clippy::expect_used)]
fn fixture_files(dir: &str) -> Vec<std::path::PathBuf> {
    let root = std::path::Path::new(FIXTURES).join(dir);
    let mut files: Vec<std::path::PathBuf> = std::fs::read_dir(&root)
        .expect("fixtures dir must be readable")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "md"))
        .collect();
    files.sort();
    files
}

#[allow(clippy::panic)]
fn fixture(dir: &str, matches: impl Fn(&str) -> bool) -> String {
    let files = fixture_files(dir);
    let found = files
        .iter()
        .find(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(&matches))
        .cloned()
        .unwrap_or_default();
    let name = found
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("<missing fixture>");
    std::fs::read_to_string(&found).unwrap_or_else(|e| panic!("{name}: {e}"))
}

#[allow(clippy::panic)]
fn parse(src: &str) -> TaskDoc {
    TaskDoc::parse(src).unwrap_or_else(|e| panic!("fixture must parse: {e:#}"))
}

/// The Definition of Done fixture, written by the backlog CLI v1.51.0: both
/// checkbox sections parse with their own numbering, and the checked states
/// survive a render round trip.
#[test]
fn definition_of_done_fixture_parses_both_checkbox_sections() {
    let path =
        std::path::Path::new(FIXTURES).join("tasks/task-0005 - TEST-9-definition-of-done-shape.md");
    let src = std::fs::read_to_string(&path).expect("read fixture");
    let doc = TaskDoc::parse(&src).expect("must parse");

    let ac = doc.body.ac_items();
    let dod = doc.body.dod_items();
    assert_eq!(ac.len(), 2);
    assert_eq!(dod.len(), 2);
    assert!(!ac[0].checked && ac[1].checked, "criteria states");
    assert!(
        dod[0].checked && !dod[1].checked,
        "definition-of-done states"
    );
    assert_eq!(dod[0].text, "cargo nextest run is green");

    let rendered = doc.render();
    let reparsed = TaskDoc::parse(&rendered).expect("re-parse");
    assert_eq!(reparsed.body.dod_items(), dod);
    assert_eq!(reparsed.body.ac_items(), ac);
}
