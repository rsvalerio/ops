//! `task view`: resolve one id across the lookup scopes, render plain or
//! JSON.

use std::io::Write;
use std::path::Path;

use anyhow::Context as _;

use crate::render;
use crate::store::Store;

/// Options for `task view`.
#[derive(Debug, Clone, Default)]
pub struct ViewOptions {
    pub task_id: String,
    pub plain: bool,
    pub json: bool,
}

/// Show one task. `--plain` and `--json` are mutually exclusive — the CLI
/// enforces it; both set here is a caller bug rendered as plain.
///
/// # Errors
///
/// The task id resolves to nothing (the error names the id), a scan of
/// `tasks/` fails, or writing `out` failed.
pub fn run_view<W: Write>(
    store: &Store,
    opts: &ViewOptions,
    workspace_root: &Path,
    out: &mut W,
) -> anyhow::Result<()> {
    let entry = store
        .find(&opts.task_id)
        .ok_or_else(|| anyhow::anyhow!("task {} not found", opts.task_id))?;
    let statuses = store.scan_tasks()?;
    let lookup = |id: &str| -> Option<String> {
        statuses
            .iter()
            .find(|e| e.doc.frontmatter.id == id)
            .map(|e| e.doc.frontmatter.status.clone())
    };
    let readiness = render::readiness_of(&entry.doc, &lookup);
    if opts.json {
        render::view_json(out, &entry, workspace_root, &readiness, &lookup)
            .context("writing task view JSON")?;
    } else {
        render::view_plain(out, &entry, &readiness).context("writing task view")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_with(task: &str) -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let tasks = dir.path().join(".backlog").join("tasks");
        std::fs::create_dir_all(&tasks).expect("tasks dir");
        std::fs::write(tasks.join("task-0001 - x.md"), task).expect("seed");
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        (dir, store)
    }

    const TASK: &str = "\
---
id: TASK-0001
title: 'view me'
status: To Do
assignee: []
created_date: '2026-08-29 18:21'
labels: []
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
the body
<!-- SECTION:DESCRIPTION:END -->
";

    #[test]
    fn plain_view_leads_with_the_file_line() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        run_view(
            &store,
            &ViewOptions {
                task_id: "TASK-0001".to_string(),
                plain: true,
                ..ViewOptions::default()
            },
            dir.path(),
            &mut out,
        )
        .expect("view");
        let text = String::from_utf8_lossy(&out);
        assert!(text.starts_with("File: /"), "first line is the File: path");
        assert!(text.contains("Task TASK-0001 - view me"));
    }

    #[test]
    fn json_view_names_the_relative_path() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        run_view(
            &store,
            &ViewOptions {
                task_id: "task-0001".to_string(),
                json: true,
                ..ViewOptions::default()
            },
            dir.path(),
            &mut out,
        )
        .expect("view");
        let value: serde_json::Value = serde_json::from_slice(&out).expect("json");
        assert_eq!(value["kind"], "task-view");
        assert_eq!(value["task"]["path"], ".backlog/tasks/task-0001 - x.md");
    }

    #[test]
    fn missing_task_names_the_id() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        let err = run_view(
            &store,
            &ViewOptions {
                task_id: "TASK-4242".to_string(),
                plain: true,
                ..ViewOptions::default()
            },
            dir.path(),
            &mut out,
        )
        .expect_err("must fail");
        assert!(
            err.to_string().contains("TASK-4242"),
            "error names the id, got: {err:#}"
        );
    }
}
