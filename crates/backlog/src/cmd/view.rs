//! `task view`: resolve one id across the lookup scopes, render plain or
//! JSON.

use std::io::Write;
use std::path::Path;

use anyhow::Context as _;

use crate::cmd::OutputFormat;
use crate::config::BacklogConfig;
use crate::render;
use crate::store::Store;

/// Options for `task view`.
#[derive(Debug, Clone, Default)]
pub struct ViewOptions {
    pub task_id: String,
    /// Which renderer to use; the CLI rejects `--plain --json` at parse
    /// time, so exactly one mode always reaches here.
    pub format: OutputFormat,
}

/// Show one task in the requested [`OutputFormat`].
///
/// # Errors
///
/// The task id resolves to nothing (the error names the id), a lookup or scan
/// directory cannot be read, or writing `out` failed.
pub fn run_view<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &ViewOptions,
    workspace_root: &Path,
    out: &mut W,
) -> anyhow::Result<()> {
    let entry = store
        .find(&opts.task_id)?
        .ok_or_else(|| anyhow::anyhow!("task {} not found", opts.task_id))?;
    let scanned = store.scan_tasks()?;
    // The same index `task list --json` builds, so the two agree.
    let index = render::StatusIndex::new(
        scanned.iter().map(|e| &e.doc),
        super::cleanup::terminal_status(&cfg.statuses),
    );
    let readiness = render::readiness_of(&entry.doc, &index);
    match opts.format {
        OutputFormat::Json => {
            render::view_json(out, &entry, workspace_root, &readiness, &index)
                .context("writing task view JSON")?;
        }
        OutputFormat::Plain => {
            render::view_plain(out, &entry, &readiness).context("writing task view")?;
        }
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
            &BacklogConfig::default(),
            &ViewOptions { task_id: "TASK-0001".to_string(), format: OutputFormat::Plain },
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
            &BacklogConfig::default(),
            &ViewOptions { task_id: "task-0001".to_string(), format: OutputFormat::Json },
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
            &BacklogConfig::default(),
            &ViewOptions { task_id: "TASK-4242".to_string(), format: OutputFormat::Plain },
            dir.path(),
            &mut out,
        )
        .expect_err("must fail");
        assert!(err.to_string().contains("TASK-4242"), "error names the id, got: {err:#}");
    }

    fn seed(dir: &tempfile::TempDir, name: &str, id: &str, status: &str, deps: &str) {
        let src = format!(
            "---\nid: {id}\ntitle: 't'\nstatus: {status}\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies:{deps}\n---\n"
        );
        std::fs::write(dir.path().join(".backlog/tasks").join(name), src).expect("seed");
    }

    fn view_json(
        store: &Store,
        cfg: &BacklogConfig,
        dir: &tempfile::TempDir,
        id: &str,
    ) -> serde_json::Value {
        let mut out = Vec::new();
        run_view(
            store,
            cfg,
            &ViewOptions { task_id: id.to_string(), format: OutputFormat::Json },
            dir.path(),
            &mut out,
        )
        .expect("view");
        serde_json::from_slice(&out).expect("json")
    }

    /// A dependency written in lowercase resolves to its live task, and a
    /// lowercase `done` status counts as complete: the task is ready.
    #[test]
    fn readiness_resolves_lowercase_ids_and_statuses() {
        let (dir, store) = scratch_with(TASK);
        seed(&dir, "task-0002 - dep.md", "TASK-0002", "done", " []");
        seed(&dir, "task-0003 - open-dep.md", "TASK-0003", "To Do", " []");
        seed(&dir, "task-0004 - w.md", "TASK-0004", "To Do", "\n  - task-0002");
        seed(&dir, "task-0005 - b.md", "TASK-0005", "To Do", "\n  - task-3");
        let cfg = BacklogConfig::default();
        let ready = view_json(&store, &cfg, &dir, "TASK-0004");
        assert_eq!(ready["task"]["readiness"]["isReady"], true, "{ready}");
        assert_eq!(ready["task"]["readiness"]["missingDependencies"], serde_json::json!([]));
        let blocked = view_json(&store, &cfg, &dir, "TASK-0005");
        assert_eq!(
            blocked["task"]["readiness"]["blockingDependencies"],
            serde_json::json!(["task-3"]),
            "a padded-differently dependency still resolves and blocks: {blocked}"
        );
    }

    /// The terminal status is the configured last column, not a literal
    /// `Done`.
    #[test]
    fn readiness_uses_the_configured_terminal_status() {
        let (dir, store) = scratch_with(TASK);
        seed(&dir, "task-0002 - dep.md", "TASK-0002", "Shipped", " []");
        seed(&dir, "task-0003 - w.md", "TASK-0003", "To Do", "\n  - TASK-0002");
        let cfg = BacklogConfig {
            statuses: vec!["To Do".to_string(), "Shipped".to_string()],
            ..BacklogConfig::default()
        };
        let value = view_json(&store, &cfg, &dir, "TASK-0003");
        assert_eq!(value["task"]["readiness"]["isReady"], true, "{value}");
        let done = view_json(&store, &cfg, &dir, "TASK-0002");
        assert_eq!(done["task"]["readiness"]["isReady"], false, "a complete task is not ready");
    }
}
