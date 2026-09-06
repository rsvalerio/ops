//! `task edit`: find one task, apply field updates, bump `updated_date`,
//! rewrite the file.

use std::io::Write;

use anyhow::Context as _;

use crate::clock::UtcStamp;
use crate::store::{main_task_file_name, Store};

/// Everything `task edit` can change. `assignees: None` leaves the list
/// untouched; `Some(vec![""])` (from `-a ""`) clears it.
#[derive(Debug, Clone, Default)]
pub struct EditOptions {
    pub task_id: String,
    pub status: Option<String>,
    pub assignees: Option<Vec<String>>,
    pub add_labels: Vec<String>,
    pub append_notes: Vec<String>,
    pub priority: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub ac: Option<Vec<String>>,
    pub check_ac: Vec<usize>,
    pub uncheck_ac: Vec<usize>,
    /// Set `parent_task_id` — the structural member→parent link the backlog
    /// CLI can only set at create time.
    pub parent: Option<String>,
    /// Remove `parent_task_id` entirely.
    pub clear_parent: bool,
    /// Append dependencies without replacing the list.
    pub add_dep: Vec<String>,
    /// Remove individual dependencies without restating the list.
    pub remove_dep: Vec<String>,
}

/// Apply the edits and print `Updated TASK-NNNN`.
///
/// # Errors
///
/// The task id resolves to nothing (the error names the id), the clock is
/// unreadable, an AC index is out of range, or the file cannot be written —
/// write errors name the path.
pub fn run_edit<W: Write>(store: &Store, opts: &EditOptions, out: &mut W) -> anyhow::Result<()> {
    let entry = store
        .find(&opts.task_id)
        .ok_or_else(|| anyhow::anyhow!("task {} not found", opts.task_id))?;
    let mut doc = entry.doc;
    let fm = &mut doc.frontmatter;

    if let Some(status) = &opts.status {
        fm.status.clone_from(status);
    }
    if let Some(assignees) = &opts.assignees {
        let cleared: Vec<String> = assignees
            .iter()
            .filter(|a| !a.trim().is_empty())
            .cloned()
            .collect();
        fm.assignees = cleared;
    }
    if !opts.add_labels.is_empty() {
        for label in &opts.add_labels {
            if !fm.labels.contains(label) {
                fm.labels.push(label.clone());
            }
        }
    }
    if let Some(parent) = &opts.parent {
        if !parent.trim().is_empty() {
            fm.set_extra_scalar("parent_task_id", parent);
        }
    }
    if opts.clear_parent {
        fm.remove_extra("parent_task_id");
    }
    for dep in &opts.add_dep {
        if !dep.trim().is_empty() && !fm.dependencies.contains(dep) {
            fm.dependencies.push(dep.clone());
        }
    }
    for dep in &opts.remove_dep {
        fm.dependencies.retain(|d| !d.eq_ignore_ascii_case(dep));
    }
    if let Some(priority) = &opts.priority {
        fm.priority = Some(priority.clone());
    }
    if let Some(title) = &opts.title {
        fm.title.clone_from(title);
    }
    if let Some(description) = &opts.description {
        doc.body.set_description(description);
    }
    if let Some(ac) = &opts.ac {
        let items: Vec<crate::model::AcItem> = ac
            .iter()
            .map(|text| crate::model::AcItem {
                checked: false,
                text: text.clone(),
            })
            .collect();
        doc.body.set_ac(&items);
    }
    for index in &opts.check_ac {
        doc.body.set_ac_checked(*index, true)?;
    }
    for index in &opts.uncheck_ac {
        doc.body.set_ac_checked(*index, false)?;
    }
    for note in &opts.append_notes {
        doc.body.append_notes(note);
    }

    let stamp = UtcStamp::now()?;
    doc.frontmatter.updated_date = Some(format!("{} {}", stamp.date, stamp.minutes));
    let rendered = doc.render();

    if opts.title.is_some() {
        // A title change moves the slug: write the new file and remove the
        // old one so only one task owns the id.
        let number = entry
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .and_then(extract_number)
            .ok_or_else(|| {
                anyhow::anyhow!("cannot derive task number from {}", entry.path.display())
            })?;
        let file_name = main_task_file_name(number, 4, &doc.frontmatter.title);
        let new_path = store.task_path(&file_name);
        if new_path == entry.path {
            std::fs::write(&new_path, rendered)
                .with_context(|| format!("writing {}", new_path.display()))?;
        } else {
            std::fs::write(&new_path, rendered)
                .with_context(|| format!("writing {}", new_path.display()))?;
            std::fs::remove_file(&entry.path)
                .with_context(|| format!("removing {}", entry.path.display()))?;
        }
    } else {
        std::fs::write(&entry.path, rendered)
            .with_context(|| format!("writing {}", entry.path.display()))?;
    }

    writeln!(out, "Updated {}", doc.frontmatter.id).context("printing the updated task id")?;
    Ok(())
}

/// The `task-<n>` number from a task filename.
fn extract_number(file_name: &str) -> Option<u32> {
    let stem = file_name.strip_suffix(".md")?;
    let (id, _) = stem.split_once(" - ").unwrap_or((stem, ""));
    let digits = id
        .strip_prefix("task-")?
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .filter(|d| !d.is_empty())?;
    digits.parse::<u32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::model::TaskDoc;

    fn scratch_with(task: &str) -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let tasks = dir.path().join(".backlog").join("tasks");
        std::fs::create_dir_all(&tasks).expect("tasks dir");
        std::fs::write(tasks.join("task-0001 - original.md"), task).expect("seed");
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        (dir, store)
    }

    const TASK: &str = "\
---
id: TASK-0001
title: 'original'
status: Triage
assignee: []
created_date: '2026-08-29 18:21'
labels: []
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
body text
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 criterion
<!-- AC:END -->
";

    fn opts() -> EditOptions {
        EditOptions {
            task_id: "TASK-0001".to_string(),
            ..EditOptions::default()
        }
    }

    #[test]
    fn edit_status_and_assignee_replace() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions {
                status: Some("To Do".to_string()),
                assignees: Some(vec!["code-review-wave".to_string()]),
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        assert_eq!(String::from_utf8_lossy(&out), "Updated TASK-0001\n");
        let doc = TaskDoc::parse(
            &std::fs::read_to_string(dir.path().join(".backlog/tasks/task-0001 - original.md"))
                .expect("read"),
        )
        .expect("re-parse");
        assert_eq!(doc.frontmatter.status, "To Do");
        assert_eq!(doc.frontmatter.assignees, vec!["code-review-wave"]);
        assert!(
            doc.frontmatter.updated_date.is_some(),
            "updated_date bumped"
        );
    }

    #[test]
    fn empty_assignee_clears_the_list() {
        let (dir, store) = scratch_with(&TASK.replacen("assignee: []", "assignee:\n  - wave", 1));
        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions {
                assignees: Some(vec![String::new()]),
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        let doc = TaskDoc::parse(
            &std::fs::read_to_string(dir.path().join(".backlog/tasks/task-0001 - original.md"))
                .expect("read"),
        )
        .expect("re-parse");
        assert!(doc.frontmatter.assignees.is_empty());
    }

    #[test]
    fn add_label_appends_without_duplicating() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        for _ in 0..2 {
            run_edit(
                &store,
                &EditOptions {
                    add_labels: vec!["code-review-wave".to_string()],
                    ..opts()
                },
                &mut out,
            )
            .expect("edit");
        }
        let doc = TaskDoc::parse(
            &std::fs::read_to_string(dir.path().join(".backlog/tasks/task-0001 - original.md"))
                .expect("read"),
        )
        .expect("re-parse");
        assert_eq!(doc.frontmatter.labels, vec!["code-review-wave"]);
    }

    #[test]
    fn append_notes_creates_then_extends_the_section() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions {
                append_notes: vec!["Overlaps: TASK-0119".to_string()],
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        run_edit(
            &store,
            &EditOptions {
                append_notes: vec!["Branch: code-review/TASK-0001".to_string()],
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        let doc = TaskDoc::parse(
            &std::fs::read_to_string(dir.path().join(".backlog/tasks/task-0001 - original.md"))
                .expect("read"),
        )
        .expect("re-parse");
        assert_eq!(
            doc.body.notes(),
            Some("Overlaps: TASK-0119\n\nBranch: code-review/TASK-0001")
        );
    }

    #[test]
    fn title_change_renames_the_file() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions {
                title: Some("renamed title".to_string()),
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        let tasks = dir.path().join(".backlog").join("tasks");
        assert!(
            tasks.join("task-0001 - renamed-title.md").is_file(),
            "new slug file must exist"
        );
        assert!(
            !tasks.join("task-0001 - original.md").exists(),
            "old file must be gone"
        );
    }

    /// `--parent` sets, `--clear-parent` removes — the structural link the
    /// backlog CLI cannot edit after create.
    #[test]
    fn parent_set_and_clear() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions {
                parent: Some("TASK-0099".to_string()),
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        let path = dir.path().join(".backlog/tasks/task-0001 - original.md");
        let doc = TaskDoc::parse(&std::fs::read_to_string(&path).expect("read")).expect("re-parse");
        assert_eq!(
            doc.frontmatter.extra_scalar("parent_task_id"),
            Some("TASK-0099")
        );

        run_edit(
            &store,
            &EditOptions {
                clear_parent: true,
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        let doc = TaskDoc::parse(&std::fs::read_to_string(&path).expect("read")).expect("re-parse");
        assert_eq!(doc.frontmatter.extra_scalar("parent_task_id"), None);
    }

    /// `--add-dep` appends without duplicating; `--remove-dep` unlinks
    /// individually without restating the list.
    #[test]
    fn add_and_remove_individual_dependencies() {
        let (dir, store) = scratch_with(TASK);
        let path = dir.path().join(".backlog/tasks/task-0001 - original.md");
        let mut out = Vec::new();
        for dep in ["TASK-0010", "TASK-0011"] {
            run_edit(
                &store,
                &EditOptions {
                    add_dep: vec![dep.to_string()],
                    ..opts()
                },
                &mut out,
            )
            .expect("edit");
        }
        // Appending a dep that is already there must not duplicate it.
        run_edit(
            &store,
            &EditOptions {
                add_dep: vec!["TASK-0010".to_string()],
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        let doc = TaskDoc::parse(&std::fs::read_to_string(&path).expect("read")).expect("re-parse");
        assert_eq!(doc.frontmatter.dependencies, vec!["TASK-0010", "TASK-0011"]);

        run_edit(
            &store,
            &EditOptions {
                remove_dep: vec!["task-0010".to_string()],
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        let doc = TaskDoc::parse(&std::fs::read_to_string(&path).expect("read")).expect("re-parse");
        assert_eq!(doc.frontmatter.dependencies, vec!["TASK-0011"]);
    }

    #[test]
    fn missing_task_is_an_error_naming_the_id() {
        let (_dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        let err = run_edit(
            &store,
            &EditOptions {
                task_id: "TASK-9999".to_string(),
                ..EditOptions::default()
            },
            &mut out,
        )
        .expect_err("must fail");
        assert!(
            err.to_string().contains("TASK-9999"),
            "error must name the id, got: {err:#}"
        );
    }

    #[test]
    fn check_and_uncheck_ac_by_index() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions {
                check_ac: vec![1],
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        let doc = TaskDoc::parse(
            &std::fs::read_to_string(dir.path().join(".backlog/tasks/task-0001 - original.md"))
                .expect("read"),
        )
        .expect("re-parse");
        assert!(doc.body.ac_items()[0].checked);
    }
}
