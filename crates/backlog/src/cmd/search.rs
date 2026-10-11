//! `search`: keyword search over `tasks/`, with the `--modified-file`
//! substring filter the triage skill relies on.

use std::io::Write;

use anyhow::Context as _;

use crate::cmd::OutputFormat;
use crate::render;
use crate::store::Store;

/// Options for `search`.
#[derive(Debug, Clone, Default)]
pub struct SearchOptions {
    /// Query tokens are matched by containment against id, title, labels,
    /// description, and notes. Empty = list everything (as hits with no
    /// score suffix).
    pub query: Option<String>,
    /// Keep tasks where any `modified_files` entry contains one of these as
    /// a substring.
    pub modified_file: Vec<String>,
    /// Drop tasks whose status matches (case-insensitive).
    pub exclude_status: Vec<String>,
    /// The renderer: human-readable rows (also the default) or the
    /// versioned `kind: search` JSON document.
    pub format: OutputFormat,
}

/// Search and render `Tasks:` rows.
///
/// # Errors
///
/// A task file in `tasks/` does not parse (the error names the file), or
/// writing `out` failed.
pub fn run_search<W: Write>(
    store: &Store,
    opts: &SearchOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    let entries = store.scan_tasks()?;
    let query = opts.query.as_deref().unwrap_or("").trim().to_string();
    let mut hits: Vec<render::SearchHit<'_>> = entries
        .iter()
        .filter(|e| modified_file_matches(opts, &e.doc.frontmatter.modified_files))
        .filter(|e| !excluded_status(opts, &e.doc.frontmatter.status))
        .filter(|e| query.is_empty() || render::score_entry(e, &query) > 0.0)
        .map(|e| render::SearchHit {
            entry: e,
            score: render::score_entry(e, &query),
        })
        .collect();
    render::sort_hits(&mut hits);
    match opts.format {
        OutputFormat::Json => {
            render::search_json(out, &hits, !query.is_empty()).context("writing search results")
        }
        OutputFormat::Plain => {
            render::search_plain(out, &hits, !query.is_empty()).context("writing search results")
        }
    }
}

fn modified_file_matches(opts: &SearchOptions, modified_files: &[String]) -> bool {
    opts.modified_file.is_empty()
        || opts.modified_file.iter().any(|wanted| {
            modified_files
                .iter()
                .any(|file| file.contains(wanted.as_str()))
        })
}

fn excluded_status(opts: &SearchOptions, status: &str) -> bool {
    opts.exclude_status
        .iter()
        .any(|excluded| excluded.eq_ignore_ascii_case(status))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_with_tasks() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let tasks = dir.path().join(".backlog").join("tasks");
        std::fs::create_dir_all(&tasks).expect("tasks dir");
        let seed = |name: &str, id: &str, title: &str, status: &str, modified: &str| {
            std::fs::write(
                tasks.join(name),
                format!(
                    "---\nid: {id}\ntitle: '{title}'\nstatus: {status}\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\nmodified_files:\n  - {modified}\n---\n"
                ),
            )
            .expect("seed");
        };
        seed(
            "task-0001 - a.md",
            "TASK-0001",
            "DUP-3: duplicated scaffold",
            "Done",
            "crates/foo/src/lib.rs",
        );
        seed(
            "task-0002 - b.md",
            "TASK-0002",
            "ERR-5: unwrap in handler",
            "To Do",
            "crates/bar/src/main.rs",
        );
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        (dir, store)
    }

    #[test]
    fn exact_finding_id_ranks_first_with_full_score() {
        let (_dir, store) = scratch_with_tasks();
        let mut out = Vec::new();
        run_search(
            &store,
            &SearchOptions {
                query: Some("DUP-3".to_string()),
                format: OutputFormat::Plain,
                ..SearchOptions::default()
            },
            &mut out,
        )
        .expect("search");
        let text = String::from_utf8_lossy(&out);
        assert!(
            text.contains("TASK-0001 - DUP-3: duplicated scaffold (Done) [score 0.300]"),
            "a finding id in the title scores the title weight, got: {text}"
        );
        assert!(!text.contains("TASK-0002"), "non-matching task excluded");
    }

    #[test]
    fn modified_file_substring_filter() {
        let (_dir, store) = scratch_with_tasks();
        let mut out = Vec::new();
        run_search(
            &store,
            &SearchOptions {
                modified_file: vec!["crates/foo".to_string()],
                format: OutputFormat::Plain,
                ..SearchOptions::default()
            },
            &mut out,
        )
        .expect("search");
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("TASK-0001"));
        assert!(!text.contains("TASK-0002"));
        assert!(
            !text.contains("[score"),
            "no query means no score suffix, got: {text}"
        );
    }

    /// TASK-2282: pins the `search --json` document shape.
    #[test]
    fn json_output_pins_the_document_shape() {
        let (_dir, store) = scratch_with_tasks();
        let mut out = Vec::new();
        run_search(
            &store,
            &SearchOptions {
                query: Some("DUP-3".to_string()),
                format: OutputFormat::Json,
                ..SearchOptions::default()
            },
            &mut out,
        )
        .expect("search");
        let text = String::from_utf8(out).expect("utf8");
        assert_eq!(
            text,
            "{\n  \"schemaVersion\": 1,\n  \"kind\": \"search\",\n  \"tasks\": [\n    {\n      \
             \"id\": \"TASK-0001\",\n      \"title\": \"DUP-3: duplicated scaffold\",\n      \
             \"status\": \"Done\",\n      \"priority\": null,\n      \"labels\": [],\n      \
             \"modifiedFiles\": [\n        \"crates/foo/src/lib.rs\"\n      ],\n      \
             \"score\": 0.300\n    }\n  ]\n}\n"
        );
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(value["schemaVersion"], 1);
    }

    #[test]
    fn json_output_without_query_has_null_scores_and_empty_list_is_valid() {
        let (_dir, store) = scratch_with_tasks();
        let mut out = Vec::new();
        run_search(
            &store,
            &SearchOptions {
                modified_file: vec!["crates/bar".to_string()],
                format: OutputFormat::Json,
                ..SearchOptions::default()
            },
            &mut out,
        )
        .expect("search");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("valid json");
        assert_eq!(value["tasks"][0]["id"], "TASK-0002");
        assert!(value["tasks"][0]["score"].is_null());

        let mut out = Vec::new();
        run_search(
            &store,
            &SearchOptions {
                query: Some("no-such-token".to_string()),
                format: OutputFormat::Json,
                ..SearchOptions::default()
            },
            &mut out,
        )
        .expect("search");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("valid json");
        assert_eq!(value["tasks"], serde_json::json!([]));
    }

    #[test]
    fn exclude_status_drops_done_for_dedup_checks() {
        let (_dir, store) = scratch_with_tasks();
        let mut out = Vec::new();
        run_search(
            &store,
            &SearchOptions {
                query: Some("DUP-3".to_string()),
                exclude_status: vec!["done".to_string()],
                format: OutputFormat::Plain,
                ..SearchOptions::default()
            },
            &mut out,
        )
        .expect("search");
        let text = String::from_utf8_lossy(&out);
        assert_eq!(
            text, "Tasks:\n",
            "the only hit was Done and must be excluded"
        );
    }
}
