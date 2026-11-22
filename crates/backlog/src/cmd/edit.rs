//! `task edit`: find one task, apply field updates, bump `updated_date`,
//! rewrite the file.

use std::io::Write;

use anyhow::Context as _;

use super::{atomic_write, link_noclobber};
use crate::clock::UtcStamp;
use crate::store::{same_task_id, Store, TaskFileName};

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
/// The read, the in-memory edit and the write run under the store's
/// allocation lock, so concurrent edits of one task compose instead of the
/// last rename silently discarding the others.
///
/// # Errors
///
/// A label, assignee, dependency or the priority cannot be stored in
/// frontmatter (the error names the field), the lock cannot be taken, the
/// task id or `--parent` resolves to nothing (the error names the id), a
/// lookup directory cannot be read, the clock is unreadable, an
/// acceptance-criterion or definition-of-done index is out of range, or the
/// file cannot be written — write errors name the path.
pub fn run_edit<W: Write>(store: &Store, opts: &EditOptions, out: &mut W) -> anyhow::Result<()> {
    edit_with(store, opts, &UtcStamp::now()?, out)
}

/// [`run_edit`] stamping `updated_date` from `stamp`, read once at the
/// command edge — the seam composite commands (`wave claim`/`park`) and the
/// fixed-clock tests drive.
///
/// # Errors
///
/// As [`run_edit`], except the clock, which the caller already read.
pub(crate) fn edit_with<W: Write>(
    store: &Store,
    opts: &EditOptions,
    stamp: &UtcStamp,
    out: &mut W,
) -> anyhow::Result<()> {
    validate_frontmatter_inputs(opts)?;
    let _lock = store.lock_allocation()?;
    let entry = store
        .find(&opts.task_id)?
        .ok_or_else(|| anyhow::anyhow!("task {} not found", opts.task_id))?;
    // Same contract as `create --parent`: the link must name an existing
    // task and is stored in the parent's canonical id, because consumers
    // match it against canonical ids. A blank parent is ignored.
    let parent_id = match opts.parent.as_deref() {
        Some(parent) if !parent.trim().is_empty() => Some(
            store
                .find(parent)?
                .ok_or_else(|| anyhow::anyhow!("parent task {parent} not found"))?
                .doc
                .frontmatter
                .id,
        ),
        _ => None,
    };
    let mut doc = entry.doc;
    apply_frontmatter_edits(&mut doc.frontmatter, opts, parent_id.as_deref());
    apply_body_edits(&mut doc, opts)?;

    doc.frontmatter.updated_date = Some(format!("{} {}", stamp.date, stamp.minutes));
    let rendered = doc.render();

    if opts.title.is_some() {
        // A title change moves the slug: write the new file and remove the
        // old one so only one task owns the id.
        rename_to_new_slug(&entry.path, &doc.frontmatter.title, &rendered)?;
    } else {
        atomic_write(&entry.path, &rendered)?;
    }

    writeln!(out, "Updated {}", doc.frontmatter.id).context("printing the updated task id")?;
    Ok(())
}

/// Refuse every option that lands in frontmatter as a bare token — added
/// list items and the priority — when it cannot be written and read back
/// unchanged. Blank assignees and dependencies are skipped: the edit drops
/// them rather than storing them.
///
/// # Errors
///
/// The first offending value; the error names its field.
fn validate_frontmatter_inputs(opts: &EditOptions) -> anyhow::Result<()> {
    use crate::model::{validate_list_item, validate_priority};
    for assignee in opts.assignees.iter().flatten().filter(|a| !a.trim().is_empty()) {
        validate_list_item("assignee", assignee)?;
    }
    for label in &opts.add_labels {
        validate_list_item("labels", label)?;
    }
    for dep in opts.add_dep.iter().filter(|d| !d.trim().is_empty()) {
        validate_list_item("dependencies", dep)?;
    }
    if let Some(priority) = &opts.priority {
        validate_priority(priority)?;
    }
    Ok(())
}

/// The frontmatter-level field edits: scalar and list mutations on the
/// document's header, in the historical option order. `parent_id` is the
/// already-resolved canonical `--parent`. Infallible — every branch is a
/// plain assignment or retain.
fn apply_frontmatter_edits(
    fm: &mut crate::model::Frontmatter,
    opts: &EditOptions,
    parent_id: Option<&str>,
) {
    if let Some(status) = &opts.status {
        fm.status.clone_from(status);
    }
    if let Some(assignees) = &opts.assignees {
        let cleared: Vec<String> =
            assignees.iter().filter(|a| !a.trim().is_empty()).cloned().collect();
        fm.assignees = cleared;
    }
    if !opts.add_labels.is_empty() {
        for label in &opts.add_labels {
            if !fm.labels.contains(label) {
                fm.labels.push(label.clone());
            }
        }
    }
    if let Some(parent) = parent_id {
        fm.set_extra_scalar("parent_task_id", parent);
    }
    if opts.clear_parent {
        fm.remove_extra("parent_task_id");
    }
    for dep in &opts.add_dep {
        // Same case rule as the remove below: a dep differing only by case
        // must not slip past the duplicate check and then be removed twice.
        let already = fm.dependencies.iter().any(|existing| same_task_id(existing, dep));
        if !dep.trim().is_empty() && !already {
            fm.dependencies.push(dep.clone());
        }
    }
    for dep in &opts.remove_dep {
        fm.dependencies.retain(|d| !same_task_id(d, dep));
    }
    if let Some(priority) = &opts.priority {
        fm.priority = Some(priority.clone());
    }
    if let Some(title) = &opts.title {
        fm.title.clone_from(title);
    }
}

/// The body-section edits: description, acceptance criteria, definition of
/// done, and notes. Fails when a check/uncheck index names an item the
/// section does not carry (the error names the section).
fn apply_body_edits(doc: &mut crate::model::TaskDoc, opts: &EditOptions) -> anyhow::Result<()> {
    if let Some(description) = &opts.description {
        doc.body.set_description(description);
    }
    if let Some(ac) = &opts.ac {
        doc.body.set_ac(&crate::model::AcItem::unchecked_all(ac));
    }
    for index in &opts.check_ac {
        doc.body.set_ac_checked(*index, true)?;
    }
    for index in &opts.uncheck_ac {
        doc.body.set_ac_checked(*index, false)?;
    }
    if let Some(dod) = &opts.dod {
        doc.body.set_dod(&crate::model::AcItem::unchecked_all(dod));
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
    Ok(())
}

/// Write the task under its new title slug, in the directory the task
/// already lives in (`completed/` and `archive/tasks/` included — a rename
/// must never resurrect a cleaned-up task into `tasks/`), and drop the old
/// file. The id
/// portion of the old filename is carried over verbatim — re-deriving it
/// from the bare number would drop a dotted subtask suffix (`task-0042.03`)
/// and aim the write at the parent task's file, truncating it.
///
/// The new name is claimed before it is written with [`link_noclobber`],
/// which refuses when the slug is already taken by another file — the same
/// claim `cleanup`'s move-to-completed makes. A crash between
/// the claim and the removal of the old name leaves both files carrying
/// the task — re-running the same edit rewrites the new slug and drops the
/// old name — recoverable, never destructive. A write that *fails* (as
/// opposed to a crash) drops the claim before returning, so only the old
/// file remains.
///
/// # Errors
///
/// The old filename carries no derivable id portion (the error names the
/// path), the new slug is already taken by another file (the error names
/// it), or the write/remove fails — each error names its path.
fn rename_to_new_slug(
    old_path: &std::path::Path,
    title: &str,
    rendered: &str,
) -> anyhow::Result<()> {
    let file_id = old_path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(TaskFileName::parse)
        .map(|parsed| parsed.id)
        .ok_or_else(|| anyhow::anyhow!("cannot derive task id from {}", old_path.display()))?;
    let file_name = format!("{file_id} - {}.md", crate::model::file_slug(title));
    let new_path = old_path.with_file_name(&file_name);
    if new_path != old_path {
        link_noclobber(old_path, &new_path)?;
    }
    if let Err(write_err) = atomic_write(&new_path, rendered) {
        if new_path != old_path {
            // The no-clobber claim above left a hard link at the new slug
            // aliasing the *old* content. Drop it so a failed write leaves
            // the store as it was found — only the old file — instead of
            // stranding a stale duplicate under the new name. Every failure
            // mode of `atomic_write` aborts before the rename, so the link
            // (not a half-written document) is what sits at `new_path`.
            // The write error is the actionable one; a failed cleanup is
            // appended to it rather than replacing it.
            if let Err(cleanup_err) = std::fs::remove_file(&new_path) {
                return Err(write_err).context(format!(
                    "also failed to remove the claimed {}: {cleanup_err}",
                    new_path.display()
                ));
            }
        }
        return Err(write_err);
    }
    if new_path != old_path {
        std::fs::remove_file(old_path)
            .with_context(|| format!("removing {}", old_path.display()))?;
    }
    Ok(())
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
        EditOptions { task_id: "TASK-0001".to_string(), ..EditOptions::default() }
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
        assert!(doc.frontmatter.updated_date.is_some(), "updated_date bumped");
    }

    #[test]
    fn empty_assignee_clears_the_list() {
        let (dir, store) = scratch_with(&TASK.replacen("assignee: []", "assignee:\n  - wave", 1));
        let mut out = Vec::new();
        run_edit(&store, &EditOptions { assignees: Some(vec![String::new()]), ..opts() }, &mut out)
            .expect("edit");
        let doc = TaskDoc::parse(
            &std::fs::read_to_string(dir.path().join(".backlog/tasks/task-0001 - original.md"))
                .expect("read"),
        )
        .expect("re-parse");
        assert!(
            doc.frontmatter.assignees.is_empty(),
            "expected empty: {:?}",
            doc.frontmatter.assignees
        );
    }

    #[test]
    fn add_label_appends_without_duplicating() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        for _ in 0..2 {
            run_edit(
                &store,
                &EditOptions { add_labels: vec!["code-review-wave".to_string()], ..opts() },
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
            &EditOptions { append_notes: vec!["Overlaps: TASK-0119".to_string()], ..opts() },
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
        assert_eq!(doc.body.notes(), Some("Overlaps: TASK-0119\n\nBranch: code-review/TASK-0001"));
    }

    #[test]
    fn title_change_renames_the_file() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions { title: Some("renamed title".to_string()), ..opts() },
            &mut out,
        )
        .expect("edit");
        let tasks = dir.path().join(".backlog").join("tasks");
        assert!(tasks.join("task-0001 - renamed-title.md").is_file(), "new slug file must exist");
        assert!(!tasks.join("task-0001 - original.md").exists(), "old file must be gone");
    }

    /// A title edit of a task living in `completed/` renames it there: the
    /// cleaned-up task must not be resurrected into `tasks/`.
    #[test]
    fn title_change_keeps_a_completed_task_in_completed() {
        let (dir, store) = scratch_with(TASK);
        let backlog = dir.path().join(".backlog");
        let completed = backlog.join("completed");
        std::fs::create_dir_all(&completed).expect("completed dir");
        std::fs::rename(
            backlog.join("tasks").join("task-0001 - original.md"),
            completed.join("task-0001 - original.md"),
        )
        .expect("move to completed");
        run_edit(
            &store,
            &EditOptions { title: Some("renamed title".to_string()), ..opts() },
            &mut Vec::new(),
        )
        .expect("edit");
        assert!(completed.join("task-0001 - renamed-title.md").is_file(), "renamed in place");
        assert!(!completed.join("task-0001 - original.md").exists(), "old name gone");
        let live: Vec<_> = std::fs::read_dir(backlog.join("tasks")).expect("read tasks").collect();
        assert!(live.is_empty(), "nothing may appear in tasks/, found {live:?}");
    }

    /// `updated_date` is the injected stamp, exactly.
    #[test]
    fn edit_stamps_updated_date_from_the_injected_clock() {
        let (dir, store) = scratch_with(TASK);
        // 2026-03-04 05:06 UTC.
        let stamp = UtcStamp::from_unix_secs(1_772_600_760).expect("stamp");
        edit_with(
            &store,
            &EditOptions { status: Some("Done".to_string()), ..opts() },
            &stamp,
            &mut Vec::new(),
        )
        .expect("edit");
        let path = dir.path().join(".backlog/tasks/task-0001 - original.md");
        let doc = TaskDoc::parse(&std::fs::read_to_string(path).expect("read")).expect("parse");
        assert_eq!(doc.frontmatter.updated_date.as_deref(), Some("2026-03-04 05:06"));
    }

    /// The rewrite is staged and renamed into place: no `.tmp` staging file
    /// survives an edit, in the plain-rewrite path or the rename path.
    #[test]
    fn edit_and_rename_leave_no_staging_file_behind() {
        let (dir, store) = scratch_with(TASK);
        let tasks = dir.path().join(".backlog").join("tasks");
        let mut out = Vec::new();
        run_edit(&store, &EditOptions { status: Some("To Do".to_string()), ..opts() }, &mut out)
            .expect("plain edit");
        run_edit(
            &store,
            &EditOptions { title: Some("renamed title".to_string()), ..opts() },
            &mut out,
        )
        .expect("rename edit");
        let files: Vec<String> = std::fs::read_dir(&tasks)
            .expect("read tasks dir")
            .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            files,
            vec!["task-0001 - renamed-title.md".to_string()],
            "no staging leftovers in either path"
        );
    }

    /// A task file named at the `NAME_MAX` limit (255 bytes) is editable: the
    /// staging name truncates and hashes instead of overflowing, so the
    /// write no longer fails with `File name too long`.
    #[test]
    fn edit_succeeds_when_the_file_name_is_at_name_max() {
        let (dir, store) = scratch_with(TASK);
        let tasks = dir.path().join(".backlog").join("tasks");
        // "task-0001 - " (12) + 240 filler bytes + ".md" (3) = 255.
        let name = format!("task-0001 - {}.md", "x".repeat(240));
        assert_eq!(name.len(), 255, "the file name itself sits at NAME_MAX");
        std::fs::rename(tasks.join("task-0001 - original.md"), tasks.join(&name))
            .expect("rename to the long name");
        let mut out = Vec::new();
        run_edit(&store, &EditOptions { status: Some("To Do".to_string()), ..opts() }, &mut out)
            .expect("edit at NAME_MAX");
        let doc = TaskDoc::parse(&std::fs::read_to_string(tasks.join(&name)).expect("read"))
            .expect("re-parse");
        assert_eq!(doc.frontmatter.status, "To Do");
        let files: Vec<String> = std::fs::read_dir(&tasks)
            .expect("read tasks dir")
            .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(files, vec![name], "no staging leftovers under a long name");
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
        let err =
            run_edit(&store, &EditOptions { title: Some("taken".to_string()), ..opts() }, &mut out)
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
        assert!(!tasks.join("task-0001.01 - child.md").exists(), "old subtask file must be gone");
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
        seed_parent_99(&dir);
        let mut out = Vec::new();
        run_edit(
            &store,
            &EditOptions { parent: Some("TASK-0099".to_string()), ..opts() },
            &mut out,
        )
        .expect("edit");
        let path = dir.path().join(".backlog/tasks/task-0001 - original.md");
        let doc = TaskDoc::parse(&std::fs::read_to_string(&path).expect("read")).expect("re-parse");
        assert_eq!(doc.frontmatter.extra_scalar("parent_task_id"), Some("TASK-0099"));

        run_edit(&store, &EditOptions { clear_parent: true, ..opts() }, &mut out).expect("edit");
        let doc = TaskDoc::parse(&std::fs::read_to_string(&path).expect("read")).expect("re-parse");
        assert_eq!(doc.frontmatter.extra_scalar("parent_task_id"), None);
    }

    fn seed_parent_99(dir: &tempfile::TempDir) {
        std::fs::write(
            dir.path().join(".backlog/tasks/task-0099 - parent.md"),
            TASK.replace("id: TASK-0001\n", "id: TASK-0099\n")
                .replace("title: 'original'", "title: 'parent'"),
        )
        .expect("seed parent");
    }

    /// `--parent` follows the `create --parent` contract: a bare number is
    /// stored in canonical form, an unknown id is refused and nothing is
    /// written.
    #[test]
    fn parent_is_resolved_to_its_canonical_id_or_refused() {
        let (dir, store) = scratch_with(TASK);
        seed_parent_99(&dir);
        let path = dir.path().join(".backlog/tasks/task-0001 - original.md");
        let mut out = Vec::new();
        run_edit(&store, &EditOptions { parent: Some("99".to_string()), ..opts() }, &mut out)
            .expect("edit");
        let doc = TaskDoc::parse(&std::fs::read_to_string(&path).expect("read")).expect("re-parse");
        assert_eq!(doc.frontmatter.extra_scalar("parent_task_id"), Some("TASK-0099"));

        let before = std::fs::read_to_string(&path).expect("read");
        let err = run_edit(
            &store,
            &EditOptions { parent: Some("TASK-4242".to_string()), ..opts() },
            &mut out,
        )
        .expect_err("unknown parent");
        let msg = format!("{err:#}");
        assert!(msg.contains("TASK-4242") && msg.contains("not found"), "got: {msg}");
        assert_eq!(std::fs::read_to_string(&path).expect("read"), before, "nothing written");
    }

    /// Concurrent edits of one task compose: every appended note survives.
    #[test]
    fn concurrent_note_appends_all_survive() {
        const EDITORS: usize = 8;
        let (dir, store) = scratch_with(TASK);
        let barrier = std::sync::Barrier::new(EDITORS);
        std::thread::scope(|scope| {
            for i in 0..EDITORS {
                let (store, barrier) = (&store, &barrier);
                scope.spawn(move || {
                    barrier.wait();
                    let edit = EditOptions { append_notes: vec![format!("note-{i}")], ..opts() };
                    run_edit(store, &edit, &mut std::io::sink()).expect("edit");
                });
            }
        });
        let path = dir.path().join(".backlog/tasks/task-0001 - original.md");
        let doc = TaskDoc::parse(&std::fs::read_to_string(&path).expect("read")).expect("re-parse");
        let notes = doc.body.notes().unwrap_or_default();
        for i in 0..EDITORS {
            assert!(notes.contains(&format!("note-{i}")), "note-{i} lost: {notes}");
        }
    }

    /// A hostile label or priority is refused before the file is touched.
    #[test]
    fn hostile_label_or_priority_is_refused() {
        let (dir, store) = scratch_with(TASK);
        let path = dir.path().join(".backlog/tasks/task-0001 - original.md");
        let mut out = Vec::new();
        for edit in [
            EditOptions { add_labels: vec!["a\nstatus: Done".to_string()], ..opts() },
            EditOptions { priority: Some(String::new()), ..opts() },
        ] {
            run_edit(&store, &edit, &mut out).expect_err("must refuse");
        }
        assert_eq!(std::fs::read_to_string(&path).expect("read"), TASK);
    }

    /// `--add-dep` appends without duplicating; `--remove-dep` unlinks
    /// individually without restating the list.
    #[test]
    fn add_and_remove_individual_dependencies() {
        let (dir, store) = scratch_with(TASK);
        let path = dir.path().join(".backlog/tasks/task-0001 - original.md");
        let mut out = Vec::new();
        for dep in ["TASK-0010", "TASK-0011"] {
            run_edit(&store, &EditOptions { add_dep: vec![dep.to_string()], ..opts() }, &mut out)
                .expect("edit");
        }
        // Appending a dep that is already there must not duplicate it —
        // even when the casing differs (the remove rule is case-blind too).
        run_edit(
            &store,
            &EditOptions { add_dep: vec!["TASK-0010".to_string()], ..opts() },
            &mut out,
        )
        .expect("edit");
        run_edit(
            &store,
            &EditOptions { add_dep: vec!["task-0010".to_string()], ..opts() },
            &mut out,
        )
        .expect("edit");
        let doc = TaskDoc::parse(&std::fs::read_to_string(&path).expect("read")).expect("re-parse");
        assert_eq!(doc.frontmatter.dependencies, vec!["TASK-0010", "TASK-0011"]);

        run_edit(
            &store,
            &EditOptions { remove_dep: vec!["task-0010".to_string()], ..opts() },
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
            &EditOptions { task_id: "TASK-9999".to_string(), ..EditOptions::default() },
            &mut out,
        )
        .expect_err("must fail");
        assert!(err.to_string().contains("TASK-9999"), "error must name the id, got: {err:#}");
    }

    #[test]
    fn check_and_uncheck_ac_by_index() {
        let (dir, store) = scratch_with(TASK);
        let mut out = Vec::new();
        run_edit(&store, &EditOptions { check_ac: vec![1], ..opts() }, &mut out).expect("edit");
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
        run_edit(&store, &EditOptions { check_dod: vec![2], ..opts() }, &mut out)
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
        let err = run_edit(&store, &EditOptions { check_dod: vec![1], ..opts() }, &mut out)
            .expect_err("no dod items");
        assert!(err.to_string().contains("no definition-of-done item #1"));
    }
}
