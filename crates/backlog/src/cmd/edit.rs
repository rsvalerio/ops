//! `task edit`: find one task, apply field updates, bump `updated_date`,
//! rewrite the file.

use std::io::Write;

use anyhow::Context as _;

use super::atomic_write;
use crate::clock::UtcStamp;
use crate::store::Store;

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
    /// Replace the definition-of-done list, like `ac` above.
    pub dod: Option<Vec<String>>,
    pub check_dod: Vec<usize>,
    pub uncheck_dod: Vec<usize>,
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
/// unreadable, an acceptance-criterion or definition-of-done index is out of
/// range, or the file cannot be written — write errors name the path.
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
        // Same case rule as the remove below: a dep differing only by case
        // must not slip past the duplicate check and then be removed twice.
        let already = fm
            .dependencies
            .iter()
            .any(|existing| existing.eq_ignore_ascii_case(dep));
        if !dep.trim().is_empty() && !already {
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
        doc.body.set_ac(&unchecked_items(ac));
    }
    for index in &opts.check_ac {
        doc.body.set_ac_checked(*index, true)?;
    }
    for index in &opts.uncheck_ac {
        doc.body.set_ac_checked(*index, false)?;
    }
    if let Some(dod) = &opts.dod {
        doc.body.set_dod(&unchecked_items(dod));
    }
    for index in &opts.check_dod {
        doc.body.set_dod_checked(*index, true)?;
    }
    for index in &opts.uncheck_dod {
        doc.body.set_dod_checked(*index, false)?;
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
        rename_to_new_slug(store, &entry.path, &doc.frontmatter.title, &rendered)?;
    } else {
        atomic_write(&entry.path, &rendered)?;
    }

    writeln!(out, "Updated {}", doc.frontmatter.id).context("printing the updated task id")?;
    Ok(())
}

/// Fresh, unchecked checkbox items — `--ac` and `--dod` both replace their
/// section wholesale, so a replacement always starts unchecked.
fn unchecked_items(texts: &[String]) -> Vec<crate::model::AcItem> {
    texts
        .iter()
        .map(|text| crate::model::AcItem {
            checked: false,
            text: text.clone(),
        })
        .collect()
}

/// Write the task under its new title slug and drop the old file. The id
/// portion of the old filename is carried over verbatim — re-deriving it
/// from the bare number would drop a dotted subtask suffix (`task-0042.03`)
/// and aim the write at the parent task's file, truncating it.
///
/// The new name is claimed before it is written: `hard_link` fails with
/// `EEXIST` when the slug is already taken by another file, the same
/// no-clobber claim `cleanup`'s move-to-completed makes. A crash between
/// the claim and the removal of the old name leaves both files carrying
/// the task — re-running the same edit rewrites the new slug and drops the
/// old name — recoverable, never destructive.
///
/// # Errors
///
/// The old filename carries no derivable id portion (the error names the
/// path), the new slug is already taken by another file (the error names
/// both paths), or the write/remove fails — each error names its path.
fn rename_to_new_slug(
    store: &Store,
    old_path: &std::path::Path,
    title: &str,
    rendered: &str,
) -> anyhow::Result<()> {
    let file_id = old_path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(extract_file_id)
        .ok_or_else(|| anyhow::anyhow!("cannot derive task id from {}", old_path.display()))?;
    let file_name = format!("{file_id} - {}.md", crate::model::file_slug(title));
    let new_path = store.task_path(&file_name);
    if new_path != old_path {
        if let Err(err) = std::fs::hard_link(old_path, &new_path) {
            if err.kind() == std::io::ErrorKind::AlreadyExists {
                anyhow::bail!(
                    "{} already exists; refusing to overwrite it with {}",
                    new_path.display(),
                    old_path.display()
                );
            }
            return Err(err).with_context(|| {
                format!("linking {} to {}", old_path.display(), new_path.display())
            });
        }
    }
    atomic_write(&new_path, rendered)?;
    if new_path != old_path {
        std::fs::remove_file(old_path)
            .with_context(|| format!("removing {}", old_path.display()))?;
    }
    Ok(())
}

/// The `task-<n>[.<mm>]` id portion of a task filename, verbatim — the
/// dotted subtask suffix included, so a rename can never retarget the
/// parent task's file.
fn extract_file_id(file_name: &str) -> Option<&str> {
    let stem = file_name.strip_suffix(".md")?;
    let (id, _) = stem.split_once(" - ").unwrap_or((stem, ""));
    Some(id)
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

    /// The rewrite is staged and renamed into place: no `.tmp` staging file
    /// survives an edit, in the plain-rewrite path or the rename path.
    #[test]
    fn edit_and_rename_leave_no_staging_file_behind() {
        let (dir, store) = scratch_with(TASK);
        let tasks = dir.path().join(".backlog").join("tasks");
        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions {
                status: Some("To Do".to_string()),
                ..opts()
            },
            &mut out,
        )
        .expect("plain edit");
        run_edit(
            &store,
            &EditOptions {
                title: Some("renamed title".to_string()),
                ..opts()
            },
            &mut out,
        )
        .expect("rename edit");
        let files: Vec<String> = std::fs::read_dir(&tasks)
            .expect("read tasks dir")
            .map(|entry| {
                entry
                    .expect("dir entry")
                    .file_name()
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(
            files,
            vec!["task-0001 - renamed-title.md".to_string()],
            "no staging leftovers in either path"
        );
    }

    /// The new slug is claimed before it is written: a title rename refuses
    /// to clobber an unrelated file that already owns the slug instead of
    /// truncating it.
    #[test]
    fn title_rename_refuses_to_clobber_an_existing_file() {
        let (dir, store) = scratch_with(TASK);
        let tasks = dir.path().join(".backlog").join("tasks");
        let squatter = TASK
            .replace("id: TASK-0001\n", "id: TASK-0009\n")
            .replace("title: 'original'", "title: 'taken'");
        std::fs::write(tasks.join("task-0001 - taken.md"), &squatter).expect("seed squatter");

        let mut out = Vec::new();
        let err = run_edit(
            &store,
            &EditOptions {
                title: Some("taken".to_string()),
                ..opts()
            },
            &mut out,
        )
        .expect_err("must refuse the taken slug");
        assert!(
            err.to_string().contains("refusing to overwrite"),
            "error must refuse, got: {err:#}"
        );
        assert_eq!(
            std::fs::read_to_string(tasks.join("task-0001 - taken.md")).expect("read squatter"),
            squatter,
            "the file owning the slug is untouched"
        );
        assert!(
            tasks.join("task-0001 - original.md").is_file(),
            "the source file survives the refused rename"
        );
    }

    /// A dotted subtask keeps its dotted file name through a title edit:
    /// re-deriving the name from the bare number would aim the write at the
    /// parent's `task-0001` file and truncate it.
    #[test]
    fn subtask_title_edit_keeps_the_dotted_file_name() {
        let (dir, store) = scratch_with(TASK);
        let tasks = dir.path().join(".backlog").join("tasks");
        let subtask = TASK
            .replace("id: TASK-0001\n", "id: TASK-0001.01\n")
            .replace("title: 'original'", "title: 'child'");
        std::fs::write(tasks.join("task-0001.01 - child.md"), subtask).expect("seed subtask");

        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions {
                task_id: "TASK-0001.01".to_string(),
                title: Some("grown-up".to_string()),
                ..EditOptions::default()
            },
            &mut out,
        )
        .expect("edit");

        assert!(
            tasks.join("task-0001.01 - grown-up.md").is_file(),
            "the subtask keeps its dotted id under the new slug"
        );
        assert!(
            !tasks.join("task-0001.01 - child.md").exists(),
            "old subtask file must be gone"
        );
        let parent = std::fs::read_to_string(tasks.join("task-0001 - original.md"))
            .expect("parent file must survive");
        assert!(
            parent.contains("title: 'original'"),
            "the parent task must not be truncated, got: {parent}"
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
        // Appending a dep that is already there must not duplicate it —
        // even when the casing differs (the remove rule is case-blind too).
        run_edit(
            &store,
            &EditOptions {
                add_dep: vec!["TASK-0010".to_string()],
                ..opts()
            },
            &mut out,
        )
        .expect("edit");
        run_edit(
            &store,
            &EditOptions {
                add_dep: vec!["task-0010".to_string()],
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

    #[test]
    fn dod_is_written_then_checked_by_index() {
        let (dir, store) = scratch_with(TASK);
        let path = dir.path().join(".backlog/tasks/task-0001 - original.md");
        let reparse = |path: &std::path::Path| {
            TaskDoc::parse(&std::fs::read_to_string(path).expect("read")).expect("re-parse")
        };

        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions {
                dod: Some(vec!["gate one".to_string(), "gate two".to_string()]),
                ..opts()
            },
            &mut out,
        )
        .expect("set dod");
        let doc = reparse(&path);
        assert_eq!(doc.body.dod_items().len(), 2);
        // A DoD write leaves the criteria section alone.
        assert_eq!(doc.body.ac_items().len(), 1);

        // Reopen: the store holds parsed copies, and the file just changed.
        let store = Store::open(&dir.path().join(".backlog")).expect("reopen");
        run_edit(
            &store,
            &EditOptions {
                check_dod: vec![2],
                ..opts()
            },
            &mut out,
        )
        .expect("check dod");
        let doc = reparse(&path);
        assert!(!doc.body.dod_items()[0].checked);
        assert!(doc.body.dod_items()[1].checked);
        assert!(!doc.body.ac_items()[0].checked);
    }

    #[test]
    fn dod_index_out_of_range_names_the_section() {
        let (_dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        let err = run_edit(
            &store,
            &EditOptions {
                check_dod: vec![1],
                ..opts()
            },
            &mut out,
        )
        .expect_err("no dod items");
        assert!(err.to_string().contains("no definition-of-done item #1"));
    }
}
