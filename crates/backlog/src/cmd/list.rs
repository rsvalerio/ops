//! `task list`: scan `tasks/`, filter, render plain or JSON.

use std::io::Write;

use anyhow::Context as _;

use crate::cmd::OutputFormat;
use crate::config::BacklogConfig;
use crate::render;
use crate::store::Store;

/// Filters for `task list`.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    /// Statuses to keep (case-insensitive); empty = all.
    pub statuses: Vec<String>,
    /// Assignees to keep — a task matches when any assignee matches
    /// (case-insensitive); empty = all.
    pub assignees: Vec<String>,
    /// Labels every one of which the task must carry (case-insensitive,
    /// AND semantics — the backlog CLI's `--labels` contract); empty = all.
    pub labels: Vec<String>,
    /// Keep tasks whose `parent_task_id` matches (case-insensitive) —
    /// membership from the parent side.
    pub parent: Option<String>,
    /// Keep tasks whose `dependencies:` include this id (case-insensitive) —
    /// the reverse query ("dependents of X") the backlog CLI lacks.
    pub dependents: Option<String>,
    /// Which renderer to use; the CLI rejects `--plain --json` at parse
    /// time, so exactly one mode always reaches here.
    pub format: OutputFormat,
}

/// List tasks grouped by status (plain) or as the `task-list` JSON envelope.
///
/// # Errors
///
/// A task file in `tasks/` does not parse (the error names the file), or
/// writing `out` failed.
pub fn run_list<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &ListOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    let entries = store.scan_tasks()?;
    let filtered: Vec<_> = entries
        .into_iter()
        .filter(|e| status_matches(opts, &e.doc.frontmatter.status))
        .filter(|e| assignee_matches(opts, &e.doc.frontmatter.assignees))
        .filter(|e| labels_match(opts, &e.doc.frontmatter.labels))
        .filter(|e| parent_matches(opts, e))
        .filter(|e| dependents_match(opts, e))
        .collect();
    match opts.format {
        OutputFormat::Json => {
            render::list_json(out, &filtered, &cfg.statuses).context("writing task list JSON")?;
        }
        OutputFormat::Plain => {
            // `--plain` and the default render identically here: the
            // interactive board is out of scope, so plain is always the
            // shape.
            render::list_plain(out, &filtered, &cfg.statuses).context("writing task list")?;
        }
    }
    Ok(())
}

fn status_matches(opts: &ListOptions, status: &str) -> bool {
    opts.statuses.is_empty()
        || opts
            .statuses
            .iter()
            .any(|wanted| wanted.eq_ignore_ascii_case(status))
}

/// AND semantics, matching the backlog CLI: every requested label must be
/// present on the task (case-insensitive).
fn labels_match(opts: &ListOptions, labels: &[String]) -> bool {
    opts.labels.iter().all(|wanted| {
        labels
            .iter()
            .any(|label| label.eq_ignore_ascii_case(wanted))
    })
}

fn parent_matches(opts: &ListOptions, entry: &crate::store::TaskEntry) -> bool {
    opts.parent.as_ref().is_none_or(|wanted| {
        entry
            .doc
            .frontmatter
            .extra_scalar("parent_task_id")
            .is_some_and(|parent| parent.eq_ignore_ascii_case(wanted))
    })
}

fn dependents_match(opts: &ListOptions, entry: &crate::store::TaskEntry) -> bool {
    opts.dependents.as_ref().is_none_or(|wanted| {
        entry
            .doc
            .frontmatter
            .dependencies
            .iter()
            .any(|dep| dep.eq_ignore_ascii_case(wanted))
    })
}

fn assignee_matches(opts: &ListOptions, assignees: &[String]) -> bool {
    opts.assignees.is_empty()
        || opts.assignees.iter().any(|wanted| {
            assignees
                .iter()
                .any(|assignee| assignee.eq_ignore_ascii_case(wanted))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_with_tasks() -> (tempfile::TempDir, Store, BacklogConfig) {
        let dir = tempfile::tempdir().expect("tempdir");
        let tasks = dir.path().join(".backlog").join("tasks");
        std::fs::create_dir_all(&tasks).expect("tasks dir");
        let seed = |name: &str, id: &str, status: &str, assignee: &str, labels: &str| {
            std::fs::write(
                tasks.join(name),
                format!(
                    "---\nid: {id}\ntitle: 't'\nstatus: {status}\nassignee: {assignee}\ncreated_date: '2026-01-01 00:00'\nlabels: {labels}\ndependencies: []\n---\n"
                ),
            )
            .expect("seed");
        };
        seed(
            "task-0001 - a.md",
            "TASK-0001",
            "To Do",
            "\n  - code-review-wave",
            "\n  - code-review-wave",
        );
        seed(
            "task-0002 - b.md",
            "TASK-0002",
            "Done",
            "[]",
            "\n  - code-review-rust\n  - security",
        );
        seed(
            "task-0003 - c.md",
            "TASK-0003",
            "To Do",
            "[]",
            "\n  - code-review-rust",
        );
        // TASK-0001 depends on TASK-0002 (reverse-query fixture); TASK-0004
        // carries a structural parent link (parent-filter fixture).
        let seeded = std::fs::read_to_string(tasks.join("task-0001 - a.md")).expect("read 1");
        std::fs::write(
            tasks.join("task-0001 - a.md"),
            seeded.replace("dependencies: []", "dependencies:\n  - TASK-0002"),
        )
        .expect("reseed 1");
        std::fs::write(
            tasks.join("task-0004 - d.md"),
            "---\nid: TASK-0004\ntitle: 't'\nstatus: To Do\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\nparent_task_id: TASK-0002\n---\n",
        )
        .expect("seed 4");
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        (dir, store, BacklogConfig::default())
    }

    #[test]
    fn status_filter_is_case_insensitive() {
        let (_dir, store, cfg) = scratch_with_tasks();
        let mut out = Vec::new();
        run_list(
            &store,
            &cfg,
            &ListOptions {
                statuses: vec!["done".to_string()],
                format: OutputFormat::Plain,
                ..ListOptions::default()
            },
            &mut out,
        )
        .expect("list");
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("Done:"));
        assert!(text.contains("TASK-0002"));
        assert!(!text.contains("TASK-0001"));
    }

    #[test]
    fn assignee_filter_matches_any_assignee() {
        let (_dir, store, cfg) = scratch_with_tasks();
        let mut out = Vec::new();
        run_list(
            &store,
            &cfg,
            &ListOptions {
                assignees: vec!["code-review-wave".to_string()],
                format: OutputFormat::Plain,
                ..ListOptions::default()
            },
            &mut out,
        )
        .expect("list");
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("TASK-0001"));
        assert!(!text.contains("TASK-0002"));
    }

    /// AND semantics with case-insensitive matching, per the backlog CLI's
    /// `--labels` contract.
    #[test]
    fn labels_filter_requires_every_label_case_insensitively() {
        let (_dir, store, cfg) = scratch_with_tasks();
        let mut out = Vec::new();
        run_list(
            &store,
            &cfg,
            &ListOptions {
                labels: vec!["code-review-rust".to_string()],
                format: OutputFormat::Plain,
                ..ListOptions::default()
            },
            &mut out,
        )
        .expect("list");
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("TASK-0002"));
        assert!(text.contains("TASK-0003"));
        assert!(!text.contains("TASK-0001"));

        let mut out = Vec::new();
        run_list(
            &store,
            &cfg,
            &ListOptions {
                labels: vec!["code-review-rust".to_string(), "SECURITY".to_string()],
                format: OutputFormat::Plain,
                ..ListOptions::default()
            },
            &mut out,
        )
        .expect("list");
        let text = String::from_utf8_lossy(&out);
        assert!(
            text.contains("TASK-0002"),
            "both labels present on TASK-0002"
        );
        assert!(
            !text.contains("TASK-0003"),
            "TASK-0003 lacks the second label, AND semantics must drop it"
        );
    }

    /// `-p` keeps tasks whose `parent_task_id` matches — membership from the
    /// parent side, the query the assignee overload used to stand in for.
    #[test]
    fn parent_filter_matches_structural_link() {
        let (_dir, store, cfg) = scratch_with_tasks();
        let mut out = Vec::new();
        run_list(
            &store,
            &cfg,
            &ListOptions {
                parent: Some("TASK-0002".to_string()),
                format: OutputFormat::Plain,
                ..ListOptions::default()
            },
            &mut out,
        )
        .expect("list");
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("TASK-0004"));
        assert!(!text.contains("TASK-0001"));
    }

    /// `--dependents` is the reverse query: every task whose `dependencies:`
    /// includes the id — the members of a wave, from the wave's side.
    #[test]
    fn dependents_filter_answers_the_reverse_query() {
        let (_dir, store, cfg) = scratch_with_tasks();
        let mut out = Vec::new();
        run_list(
            &store,
            &cfg,
            &ListOptions {
                dependents: Some("TASK-0002".to_string()),
                format: OutputFormat::Plain,
                ..ListOptions::default()
            },
            &mut out,
        )
        .expect("list");
        let text = String::from_utf8_lossy(&out);
        assert!(text.contains("TASK-0001"), "TASK-0001 depends on TASK-0002");
        assert!(!text.contains("TASK-0003"));
        assert!(!text.contains("TASK-0004"));
    }

    #[test]
    fn json_envelope_parses() {
        let (_dir, store, cfg) = scratch_with_tasks();
        let mut out = Vec::new();
        run_list(
            &store,
            &cfg,
            &ListOptions {
                format: OutputFormat::Json,
                ..ListOptions::default()
            },
            &mut out,
        )
        .expect("list");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("valid json");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["kind"], "task-list");
        assert_eq!(value["tasks"].as_array().map(Vec::len), Some(4));
    }
}
