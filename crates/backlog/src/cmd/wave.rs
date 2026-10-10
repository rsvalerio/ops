//! `wave`: the code-review wave convention as first-class commands.
//!
//! A wave is a parent task that groups the findings a review run fixes
//! together. The convention it is expressed in used to overload `assignee` —
//! the wave parent carried the literal assignee `code-review-wave`, and each
//! member's assignee was its wave's task id — which left no room for a real
//! assignee on either side. It is now structural:
//!
//! | Concept | Representation |
//! |---|---|
//! | a wave | task carrying the marker label (`code-review-wave`) |
//! | wave → members | the wave's `dependencies:` list |
//! | member → wave | the member's `parent_task_id` |
//!
//! [`run_wave_migrate`] converts a tree written the old way; `list` and
//! `members` read either shape, so a half-migrated tree still answers
//! correctly.

use std::io::Write;

use anyhow::Context as _;

use crate::clock::UtcStamp;
use crate::config::BacklogConfig;
use crate::model::TaskDoc;
use crate::render;
use crate::store::{Store, TaskEntry};

/// The label (and, before migration, the assignee) that marks a wave parent.
pub const DEFAULT_WAVE_MARKER: &str = "code-review-wave";

/// Filters for `wave list`.
#[derive(Debug, Clone)]
pub struct WaveListOptions {
    /// The marker label identifying a wave.
    pub marker: String,
    /// Statuses to keep (case-insensitive); empty = all.
    pub statuses: Vec<String>,
    pub json: bool,
}

/// Arguments of `wave members`.
#[derive(Debug, Clone)]
pub struct WaveMembersOptions {
    pub wave_id: String,
    pub json: bool,
}

/// Arguments of `wave migrate`.
#[derive(Debug, Clone)]
pub struct WaveMigrateOptions {
    /// The marker to migrate off the assignee field and onto the label.
    pub marker: String,
    /// Report what would change without writing anything.
    pub dry_run: bool,
}

/// List wave parents, grouped by status like `task list`.
///
/// A task counts as a wave when it carries the marker as a label **or** as
/// an assignee — the pre-migration shape — so this answers correctly before
/// and after [`run_wave_migrate`].
///
/// In the plain listing, waves in the terminal status (the last configured
/// column) collapse to a one-line total — a tree accumulates finished review
/// waves far faster than open ones, and listing them buries the work left to
/// do. Asking for the terminal status explicitly (`--status Done`) lists
/// them again; the JSON output always carries every row.
///
/// # Errors
///
/// A task file in `tasks/` does not parse (the error names the file), or
/// writing `out` failed.
pub fn run_wave_list<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &WaveListOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    let entries = store.scan_tasks()?;
    let waves: Vec<TaskEntry> = entries
        .into_iter()
        .filter(|entry| is_wave(entry, &opts.marker))
        .filter(|entry| {
            opts.statuses.is_empty()
                || opts
                    .statuses
                    .iter()
                    .any(|wanted| wanted.eq_ignore_ascii_case(&entry.doc.frontmatter.status))
        })
        .collect();
    if opts.json {
        return render::list_json(out, &waves, &cfg.statuses).context("writing wave list JSON");
    }
    // Done waves are collapsed unless the terminal status was asked for by
    // name — an explicit filter means the user wants those rows after all.
    let terminal = super::cleanup::terminal_status(&cfg.statuses).filter(|t| {
        !opts
            .statuses
            .iter()
            .any(|wanted| wanted.eq_ignore_ascii_case(t))
    });
    let (done, open): (Vec<TaskEntry>, Vec<TaskEntry>) = match terminal {
        Some(terminal) => waves
            .into_iter()
            .partition(|entry| entry.doc.frontmatter.status.eq_ignore_ascii_case(terminal)),
        None => (Vec::new(), waves),
    };
    render::list_plain(out, &open, &cfg.statuses).context("writing wave list")?;
    if let Some(terminal) = terminal.filter(|_| !done.is_empty()) {
        writeln!(
            out,
            "{terminal}: {} waves, hidden (pass --status {terminal} to list them)",
            done.len()
        )
        .context("printing the done-waves total")?;
    }
    Ok(())
}

/// List one wave's members: the union of its `dependencies:`, every task
/// whose `parent_task_id` names it, and every task still carrying the wave's
/// id as an assignee, grouped by status like `task list`.
///
/// The union is deliberate — the links are written at different times
/// (dependencies at wave creation, `parent_task_id` per member) and the
/// assignee is the pre-migration form of the same link, so any one of them
/// alone under-reports on a partly written or unmigrated wave. Reading all
/// three is what lets a runner work a tree [`run_wave_migrate`] has not
/// touched yet.
///
/// # Errors
///
/// The wave id resolves to nothing (the error names the id), a lookup or scan
/// directory cannot be read, a task file does not parse, or writing `out`
/// failed.
pub fn run_wave_members<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &WaveMembersOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    let wave = store
        .find(&opts.wave_id)?
        .ok_or_else(|| anyhow::anyhow!("task {} not found", opts.wave_id))?;

    let entries = store.scan_tasks()?;
    let deps = &wave.doc.frontmatter.dependencies;
    let missing: Vec<String> = deps
        .iter()
        .filter(|dep| {
            !entries
                .iter()
                .any(|e| e.doc.frontmatter.id.eq_ignore_ascii_case(dep))
        })
        .cloned()
        .collect();
    let members: Vec<TaskEntry> = entries
        .into_iter()
        .filter(|entry| is_member(entry, &wave.doc))
        .collect();

    if opts.json {
        render::list_json(out, &members, &cfg.statuses).context("writing wave member JSON")?;
    } else {
        render::list_plain(out, &members, &cfg.statuses).context("writing wave members")?;
        if !missing.is_empty() {
            // A dependency naming a task that is not in `tasks/` any more:
            // reported, never silently dropped.
            writeln!(out, "Missing dependencies: {}", missing.join(", "))
                .context("printing the missing-dependency line")?;
        }
    }
    Ok(())
}

/// Is `entry` a member of the wave `wave`? Any of the three links counts:
/// listed in the wave's `dependencies:`, a `parent_task_id` naming the wave,
/// or — the pre-migration link — the wave's id as an assignee.
fn is_member(entry: &TaskEntry, wave: &TaskDoc) -> bool {
    let fm = &entry.doc.frontmatter;
    let wave_id = &wave.frontmatter.id;
    let is_dependency = wave
        .frontmatter
        .dependencies
        .iter()
        .any(|dep| dep.eq_ignore_ascii_case(&fm.id));
    let links_here = fm
        .extra_scalar("parent_task_id")
        .is_some_and(|parent| parent.eq_ignore_ascii_case(wave_id));
    let assigned_here = has_assignee(&entry.doc, wave_id);
    is_dependency || links_here || assigned_here
}

/// Arguments of `wave overlap`.
#[derive(Debug, Clone)]
pub struct WaveOverlapOptions {
    /// Waves to report on; empty = every open wave.
    pub wave_ids: Vec<String>,
    /// The marker label identifying a wave.
    pub marker: String,
    pub json: bool,
}

/// Report each wave's file scope (the union of its members'
/// `modified_files`), the paths it shares with every other open wave, and a
/// suggested merge order.
///
/// "Open" is any status but the terminal one (the last configured column).
/// A wave named explicitly is reported whatever its status, and is compared
/// against every open wave plus the other named ones.
///
/// The merge order puts the least-overlapping wave first — fewest shared
/// paths summed over every other wave — so the waves most likely to
/// conflict land last, onto a base that already carries their neighbours.
/// Ties break on the task id's numeric order, so the order is deterministic.
///
/// # Errors
///
/// A named wave id resolves to no task in `tasks/`, a task file does not
/// parse, or writing `out` failed.
pub fn run_wave_overlap<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &WaveOverlapOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    let entries = store.scan_tasks()?;
    let terminal = super::cleanup::terminal_status(&cfg.statuses);
    let is_open = |entry: &TaskEntry| {
        terminal.is_none_or(|t| !entry.doc.frontmatter.status.eq_ignore_ascii_case(t))
    };

    let mut selected: Vec<&TaskEntry> = Vec::new();
    for id in &opts.wave_ids {
        let wave = entries
            .iter()
            .find(|e| e.doc.frontmatter.id.eq_ignore_ascii_case(id))
            .ok_or_else(|| anyhow::anyhow!("task {id} not found"))?;
        if !selected.iter().any(|w| std::ptr::eq(*w, wave)) {
            selected.push(wave);
        }
    }
    let open_waves = entries
        .iter()
        .filter(|entry| is_wave(entry, &opts.marker) && is_open(entry));
    let mut compared: Vec<&TaskEntry> = selected.clone();
    for wave in open_waves {
        if !compared.iter().any(|w| std::ptr::eq(*w, wave)) {
            compared.push(wave);
        }
    }
    if opts.wave_ids.is_empty() {
        selected.clone_from(&compared);
    }

    let scopes: Vec<(&TaskEntry, std::collections::BTreeSet<&str>)> = compared
        .iter()
        .map(|wave| (*wave, file_scope(&entries, &wave.doc)))
        .collect();
    let mut rows: Vec<(usize, render::WaveOverlapRow)> = selected
        .iter()
        .map(|wave| {
            let own = scopes
                .iter()
                .find(|(w, _)| std::ptr::eq(*w, *wave))
                .map(|(_, scope)| scope.clone())
                .unwrap_or_default();
            let mut overlaps: Vec<(String, Vec<String>)> = scopes
                .iter()
                .filter(|(other, _)| !std::ptr::eq(*other, *wave))
                .filter_map(|(other, scope)| {
                    let shared: Vec<String> =
                        own.intersection(scope).map(|p| (*p).to_string()).collect();
                    (!shared.is_empty()).then(|| (other.doc.frontmatter.id.clone(), shared))
                })
                .collect();
            overlaps.sort_by(|a, b| id_order(&a.0, &b.0));
            let weight = overlaps.iter().map(|(_, shared)| shared.len()).sum();
            let fm = &wave.doc.frontmatter;
            (
                weight,
                render::WaveOverlapRow {
                    id: fm.id.clone(),
                    title: fm.title.clone(),
                    status: fm.status.clone(),
                    files: own.iter().map(|p| (*p).to_string()).collect(),
                    overlaps,
                },
            )
        })
        .collect();
    rows.sort_by(|(wa, a), (wb, b)| wa.cmp(wb).then_with(|| id_order(&a.id, &b.id)));
    let rows: Vec<render::WaveOverlapRow> = rows.into_iter().map(|(_, row)| row).collect();

    if opts.json {
        render::wave_overlap_json(out, &rows).context("writing wave overlap JSON")
    } else {
        render::wave_overlap_plain(out, &rows).context("writing wave overlap")
    }
}

/// The union of the wave's members' `modified_files`, trimmed, blanks
/// dropped.
fn file_scope<'a>(entries: &'a [TaskEntry], wave: &TaskDoc) -> std::collections::BTreeSet<&'a str> {
    entries
        .iter()
        .filter(|entry| is_member(entry, wave))
        .flat_map(|entry| entry.doc.frontmatter.modified_files.iter())
        .map(|path| path.trim())
        .filter(|path| !path.is_empty())
        .collect()
}

/// Task ids in numeric order (`TASK-999` before `TASK-1000`), falling back
/// to a case-insensitive string order for ids without a numeric suffix.
fn id_order(a: &str, b: &str) -> std::cmp::Ordering {
    fn key(id: &str) -> (String, Option<u64>, String) {
        match id.rsplit_once('-') {
            Some((prefix, number)) => (
                prefix.to_ascii_lowercase(),
                number.parse::<u64>().ok(),
                id.to_ascii_lowercase(),
            ),
            None => (id.to_ascii_lowercase(), None, id.to_ascii_lowercase()),
        }
    }
    key(a).cmp(&key(b))
}

/// One task the migration rewrites.
struct PendingWrite {
    path: std::path::PathBuf,
    doc: TaskDoc,
}

/// What one wave's migration changes.
struct WavePlan {
    id: String,
    title: String,
    add_label: bool,
    drop_assignee: bool,
    members: Vec<String>,
}

/// Retire the assignee overload: move the marker onto the wave's labels and
/// each member's wave link into `parent_task_id`, leaving `assignee` free
/// for real people.
///
/// Idempotent — a tree that is already migrated reports nothing to do.
///
/// # Errors
///
/// A task file does not parse, or a member cannot be relinked safely — it
/// already carries a *different* `parent_task_id`, names a wave that does not
/// exist, or is claimed by two waves at once. Those three abort before
/// anything is written, naming the tasks involved. Also when the clock is
/// unreadable, the confirmation answer cannot be read, or a write failed —
/// the error names the failing path and every path already written before
/// it.
pub fn run_wave_migrate<W: Write>(
    store: &Store,
    opts: &WaveMigrateOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    let mut input = std::io::stdin().lock();
    let result = migrate_with(store, opts, &mut input, out);
    // Release the stdin lock before returning, as cleanup does.
    drop(input);
    result
}

/// [`run_wave_migrate`] against an explicit input stream; the seam the
/// confirmation tests drive.
///
/// # Errors
///
/// As [`run_wave_migrate`].
fn migrate_with<W: Write>(
    store: &Store,
    opts: &WaveMigrateOptions,
    input: &mut dyn std::io::BufRead,
    out: &mut W,
) -> anyhow::Result<()> {
    let entries = store.scan_tasks()?;
    let wave_ids: Vec<String> = entries
        .iter()
        .filter(|entry| is_wave(entry, &opts.marker))
        .map(|entry| entry.doc.frontmatter.id.clone())
        .collect();

    // Preflight every failure mode over the whole tree before planning any
    // write: a half-applied migration would leave membership split across
    // two conventions with no record of which tasks were done.
    preflight(&entries, &wave_ids, &opts.marker)?;

    let (plans, mut writes) = plan_migration(&entries, &wave_ids, &opts.marker);

    if plans.is_empty() {
        writeln!(
            out,
            "Nothing to migrate: no wave carries the assignee overload."
        )
        .context("printing the nothing-to-do notice")?;
        return Ok(());
    }

    let member_count: usize = plans.iter().map(|plan| plan.members.len()).sum();
    report_plan(&plans, out)?;

    if opts.dry_run {
        writeln!(out, "Dry run: no files changed.").context("printing the dry-run notice")?;
        return Ok(());
    }

    if !crate::cmd::confirm(
        &format!("Migrate {} waves / {member_count} members?", plans.len()),
        input,
        out,
    )? {
        writeln!(out, "Migration cancelled.").context("printing the cancellation notice")?;
        return Ok(());
    }

    let stamp = UtcStamp::now()?;
    let updated = format!("{} {}", stamp.date, stamp.minutes);
    let total = writes.len();
    let mut written: Vec<std::path::PathBuf> = Vec::new();
    for write in &mut writes {
        write.doc.frontmatter.updated_date = Some(updated.clone());
        let rendered = write.doc.render();
        if let Err(err) = crate::cmd::atomic_write(&write.path, &rendered) {
            // Every file is swapped in whole or not at all, so a stopped
            // migration is a clean split: report exactly which files landed
            // so it can be repaired — the preflight above exists to prevent
            // the split, this reports it.
            return Err(err).with_context(|| {
                format!(
                    "migration stopped after {} of {total} writes; already written: {}",
                    written.len(),
                    if written.is_empty() {
                        "none".to_string()
                    } else {
                        written
                            .iter()
                            .map(|path| path.display().to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                )
            });
        }
        written.push(write.path.clone());
    }
    writeln!(
        out,
        "Migrated {} waves and {member_count} members.",
        plans.len()
    )
    .context("printing the migrated summary")?;
    Ok(())
}

/// Build every rewrite the migration would apply, without touching disk:
/// the wave parents whose marker moves from assignee to label, and the
/// members whose wave link moves from assignee to `parent_task_id`.
///
/// A wave with nothing left to change is left out, which is what makes a
/// second run a no-op.
fn plan_migration(
    entries: &[TaskEntry],
    wave_ids: &[String],
    marker: &str,
) -> (Vec<WavePlan>, Vec<PendingWrite>) {
    let mut plans: Vec<WavePlan> = Vec::new();
    let mut writes: Vec<PendingWrite> = Vec::new();
    for wave_id in wave_ids {
        let Some(wave) = entries
            .iter()
            .find(|e| e.doc.frontmatter.id.eq_ignore_ascii_case(wave_id))
        else {
            continue;
        };
        let mut wave_doc = wave.doc.clone();
        let add_label = !has_label(&wave_doc, marker);
        let drop_assignee = has_assignee(&wave_doc, marker);
        if add_label {
            wave_doc.frontmatter.labels.push(marker.to_string());
        }
        if drop_assignee {
            retain_assignees_except(&mut wave_doc, marker);
        }

        let mut member_ids: Vec<String> = Vec::new();
        for entry in entries.iter().filter(|e| has_assignee(&e.doc, wave_id)) {
            let member_id = &entry.doc.frontmatter.id;
            let mut member_doc = entry.doc.clone();
            member_doc
                .frontmatter
                .set_extra_scalar("parent_task_id", wave_id);
            retain_assignees_except(&mut member_doc, wave_id);
            // Keep both directions in agreement: a member discovered through
            // the assignee overload but absent from the wave's dependency
            // list would vanish from `wave members` once the assignee goes.
            let deps = &mut wave_doc.frontmatter.dependencies;
            if !deps.iter().any(|dep| dep.eq_ignore_ascii_case(member_id)) {
                deps.push(member_id.clone());
            }
            member_ids.push(member_id.clone());
            writes.push(PendingWrite {
                path: entry.path.clone(),
                doc: member_doc,
            });
        }

        if !add_label && !drop_assignee && member_ids.is_empty() {
            continue;
        }
        plans.push(WavePlan {
            id: wave_doc.frontmatter.id.clone(),
            title: wave_doc.frontmatter.title.clone(),
            add_label,
            drop_assignee,
            members: member_ids,
        });
        writes.push(PendingWrite {
            path: wave.path.clone(),
            doc: wave_doc,
        });
    }
    (plans, writes)
}

/// Narrate the plan, one row per wave — the same "list, then confirm" shape
/// `cleanup` uses.
///
/// # Errors
///
/// Writing `out` failed.
fn report_plan<W: Write>(plans: &[WavePlan], out: &mut W) -> anyhow::Result<()> {
    writeln!(out, "Found {} waves to migrate:", plans.len()).context("printing the wave count")?;
    for plan in plans {
        let mut changes: Vec<String> = Vec::new();
        if plan.add_label {
            changes.push("+label".to_string());
        }
        if plan.drop_assignee {
            changes.push("-assignee".to_string());
        }
        changes.push(format!("{} members", plan.members.len()));
        writeln!(
            out,
            "  - {}: {} ({})",
            plan.id,
            plan.title,
            changes.join(", ")
        )
        .context("printing a wave row")?;
    }
    Ok(())
}

/// Abort before any write when a member cannot be relinked safely.
///
/// # Errors
///
/// A task carrying a wave id as its assignee already has a different
/// `parent_task_id`, or names an id that is not a wave in this tree.
fn preflight(entries: &[TaskEntry], wave_ids: &[String], marker: &str) -> anyhow::Result<()> {
    for entry in entries {
        let fm = &entry.doc.frontmatter;
        let mut claimed_by: Vec<&String> = Vec::new();
        for assignee in &fm.assignees {
            if assignee.eq_ignore_ascii_case(marker) || !looks_like_task_id(assignee) {
                continue;
            }
            let Some(wave_id) = wave_ids.iter().find(|id| id.eq_ignore_ascii_case(assignee)) else {
                anyhow::bail!(
                    "{} is assigned to {assignee}, which is not a wave in this tree — \
                     fix that assignee by hand, then re-run",
                    fm.id
                );
            };
            if let Some(parent) = fm.extra_scalar("parent_task_id") {
                if !parent.eq_ignore_ascii_case(wave_id) {
                    anyhow::bail!(
                        "{} is assigned to wave {wave_id} but already has parent_task_id {parent} \
                         — resolve the conflict by hand, then re-run",
                        fm.id
                    );
                }
            }
            claimed_by.push(wave_id);
        }
        // `parent_task_id` holds one wave, so a member claimed by two cannot
        // be expressed after the migration. Planning it anyway would queue two
        // writes to the same file: the later one wins, and the earlier wave's
        // assignee survives on a task now parented elsewhere.
        if claimed_by.len() > 1 {
            anyhow::bail!(
                "{} is assigned to {} waves ({}) — a member can belong to one wave, so \
                 drop the assignees that no longer apply, then re-run",
                fm.id,
                claimed_by.len(),
                claimed_by
                    .iter()
                    .map(|id| id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
        }
    }
    Ok(())
}

/// Does this look like a task id (`TASK-0042`) rather than a person?
fn looks_like_task_id(value: &str) -> bool {
    let Some((_, number)) = value.rsplit_once('-') else {
        return false;
    };
    !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit())
}

/// A wave carries the marker as a label (migrated) or as an assignee (the
/// pre-migration overload).
pub(crate) fn is_wave(entry: &TaskEntry, marker: &str) -> bool {
    has_label(&entry.doc, marker) || has_assignee(&entry.doc, marker)
}

fn has_label(doc: &TaskDoc, marker: &str) -> bool {
    doc.frontmatter
        .labels
        .iter()
        .any(|label| label.eq_ignore_ascii_case(marker))
}

fn has_assignee(doc: &TaskDoc, wanted: &str) -> bool {
    doc.frontmatter
        .assignees
        .iter()
        .any(|assignee| assignee.eq_ignore_ascii_case(wanted))
}

fn retain_assignees_except(doc: &mut TaskDoc, unwanted: &str) {
    doc.frontmatter
        .assignees
        .retain(|assignee| !assignee.eq_ignore_ascii_case(unwanted));
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::fmt::Write as _;

    /// A wave in the pre-migration shape: assignee marker, members carrying
    /// the wave id as their assignee.
    fn task(id: &str, assignees: &[&str], labels: &[&str], deps: &[&str]) -> String {
        // Block lists when non-empty, `[]` when empty — the only two shapes
        // the parser accepts, and the ones the backlog CLI writes.
        fn list(src: &mut String, key: &str, items: &[&str]) {
            if items.is_empty() {
                let _ = writeln!(src, "{key}: []");
                return;
            }
            let _ = writeln!(src, "{key}:");
            for item in items {
                let _ = writeln!(src, "  - {item}");
            }
        }

        let mut src = format!("---\nid: {id}\ntitle: 'task {id}'\nstatus: To Do\n");
        list(&mut src, "assignee", assignees);
        let _ = writeln!(src, "created_date: '2026-01-01 00:00'");
        list(&mut src, "labels", labels);
        list(&mut src, "dependencies", deps);
        src.push_str("---\n");
        src
    }

    fn scratch(tasks: &[(&str, String)]) -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().expect("tempdir");
        let tasks_dir = dir.path().join(".backlog").join("tasks");
        std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
        for (name, body) in tasks {
            std::fs::write(tasks_dir.join(name), body).expect("seed");
        }
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        (dir, store)
    }

    fn cfg() -> BacklogConfig {
        BacklogConfig::default()
    }

    fn reopen(dir: &tempfile::TempDir) -> Store {
        Store::open(&dir.path().join(".backlog")).expect("reopen")
    }

    fn doc_of(dir: &tempfile::TempDir, name: &str) -> TaskDoc {
        let path = dir.path().join(".backlog").join("tasks").join(name);
        TaskDoc::parse(&std::fs::read_to_string(path).expect("read")).expect("parse")
    }

    fn migrate(store: &Store, dry_run: bool, answer: &str) -> anyhow::Result<String> {
        let mut out = Vec::new();
        let mut input = answer.as_bytes();
        migrate_with(
            store,
            &WaveMigrateOptions {
                marker: DEFAULT_WAVE_MARKER.to_string(),
                dry_run,
            },
            &mut input,
            &mut out,
        )?;
        Ok(String::from_utf8(out).expect("utf8"))
    }

    /// The old shape: wave parent marked by assignee, one member pointing
    /// back through its own assignee.
    fn old_shape() -> Vec<(&'static str, String)> {
        vec![
            (
                "task-0119 - wave.md",
                task("TASK-0119", &["code-review-wave"], &[], &["TASK-0120"]),
            ),
            (
                "task-0120 - member.md",
                task("TASK-0120", &["TASK-0119"], &[], &[]),
            ),
            // A member linked to the wave but missing from its dependency
            // list — migration must add it, or it would vanish.
            (
                "task-0121 - stray.md",
                task("TASK-0121", &["TASK-0119"], &[], &[]),
            ),
        ]
    }

    #[test]
    fn migrate_moves_marker_to_label_and_membership_to_parent() {
        let (dir, store) = scratch(&old_shape());
        let text = migrate(&store, false, "y\n").expect("migrate");
        assert!(text.contains("Found 1 waves to migrate:"));
        assert!(text.contains("Migrated 1 waves and 2 members."));

        let wave = doc_of(&dir, "task-0119 - wave.md");
        assert!(wave.frontmatter.assignees.is_empty());
        assert_eq!(
            wave.frontmatter.labels,
            vec!["code-review-wave".to_string()]
        );
        assert_eq!(
            wave.frontmatter.dependencies,
            vec!["TASK-0120".to_string(), "TASK-0121".to_string()],
            "a member found only through the assignee joins the dependency list"
        );

        for name in ["task-0120 - member.md", "task-0121 - stray.md"] {
            let member = doc_of(&dir, name);
            assert!(member.frontmatter.assignees.is_empty());
            assert_eq!(
                member.frontmatter.extra_scalar("parent_task_id"),
                Some("TASK-0119")
            );
        }
    }

    #[test]
    fn migrate_is_idempotent() {
        let (dir, store) = scratch(&old_shape());
        migrate(&store, false, "y\n").expect("first run");
        let text = migrate(&reopen(&dir), false, "y\n").expect("second run");
        assert!(text.contains("Nothing to migrate"));
    }

    #[test]
    fn dry_run_and_decline_write_nothing() {
        let (dir, store) = scratch(&old_shape());
        let before = doc_of(&dir, "task-0119 - wave.md");

        let text = migrate(&store, true, "").expect("dry run");
        assert!(text.contains("Dry run: no files changed."));
        assert_eq!(doc_of(&dir, "task-0119 - wave.md"), before);

        let text = migrate(&reopen(&dir), false, "n\n").expect("declined");
        assert!(text.contains("Migration cancelled."));
        assert_eq!(doc_of(&dir, "task-0119 - wave.md"), before);
    }

    #[test]
    fn conflicting_parent_aborts_before_any_write() {
        let mut tasks = old_shape();
        let mut member = task("TASK-0120", &["TASK-0119"], &[], &[]);
        member = member.replacen(
            "dependencies: []\n",
            "dependencies: []\nparent_task_id: TASK-0999\n",
            1,
        );
        tasks[1] = ("task-0120 - member.md", member);
        let (dir, store) = scratch(&tasks);
        let before = doc_of(&dir, "task-0119 - wave.md");

        let err = migrate(&store, false, "y\n").expect_err("conflict");
        let message = err.to_string();
        assert!(message.contains("TASK-0120"), "{message}");
        assert!(message.contains("TASK-0999"), "{message}");
        assert_eq!(
            doc_of(&dir, "task-0119 - wave.md"),
            before,
            "preflight aborts before the first write"
        );
    }

    /// A write that fails mid-migration reports exactly which files already
    /// landed: every write is atomic, so the listed files are whole and the
    /// unlisted ones are untouched.
    #[test]
    fn a_failing_write_reports_what_already_landed() {
        let (dir, store) = scratch(&old_shape());
        // Writes run member-first: task-0120, task-0121, then the wave. A
        // directory squatting on the second write's staging path — the
        // predictable `.<name>.<pid>.tmp` this process stages under — makes
        // its exclusive `create_new` fail after the first member landed.
        let staging_squat = format!(".task-0121 - stray.md.{}.tmp", std::process::id());
        std::fs::create_dir(dir.path().join(".backlog/tasks").join(staging_squat))
            .expect("block the staging path");

        let err = migrate(&store, false, "y\n").expect_err("staging blocked");
        let message = format!("{err:#}");
        assert!(
            message.contains("already written"),
            "the report names what landed, got: {message}"
        );
        assert!(
            message.contains("task-0120 - member.md"),
            "the first member is named as written, got: {message}"
        );
        // The named file landed whole; the blocked and later ones did not.
        let member = doc_of(&dir, "task-0120 - member.md");
        assert_eq!(
            member.frontmatter.extra_scalar("parent_task_id"),
            Some("TASK-0119")
        );
        assert!(member.frontmatter.assignees.is_empty());
        let stray = doc_of(&dir, "task-0121 - stray.md");
        assert_eq!(
            stray.frontmatter.assignees,
            vec!["TASK-0119".to_string()],
            "the blocked file is untouched"
        );
    }

    #[test]
    fn assignee_naming_a_non_wave_task_aborts() {
        let tasks = vec![(
            "task-0120 - member.md",
            task("TASK-0120", &["TASK-0777"], &[], &[]),
        )];
        let (_dir, store) = scratch(&tasks);
        let err = migrate(&store, false, "y\n").expect_err("dangling wave");
        assert!(err.to_string().contains("TASK-0777"));
    }

    #[test]
    fn list_finds_waves_marked_either_way() {
        let (dir, store) = scratch(&old_shape());
        let render = |store: &Store| {
            let mut out = Vec::new();
            run_wave_list(
                store,
                &cfg(),
                &WaveListOptions {
                    marker: DEFAULT_WAVE_MARKER.to_string(),
                    statuses: vec!["To Do".to_string()],
                    json: false,
                },
                &mut out,
            )
            .expect("list");
            String::from_utf8(out).expect("utf8")
        };

        let before = render(&store);
        assert!(before.contains("TASK-0119"), "pre-migration: {before}");
        assert!(!before.contains("TASK-0120"), "members are not waves");

        migrate(&reopen(&dir), false, "y\n").expect("migrate");
        let after = render(&reopen(&dir));
        assert!(after.contains("TASK-0119"), "post-migration: {after}");
    }

    /// The plain listing totals done waves instead of listing them; an
    /// explicit `--status Done` asks for the rows back, and JSON always
    /// carries every row.
    #[test]
    fn list_collapses_done_waves_to_a_total() {
        let (_dir, store) = scratch(&[
            ("task-10 - open.md", wave("TASK-10", "To Do", &["TASK-1"])),
            ("task-11 - done.md", wave("TASK-11", "Done", &["TASK-2"])),
            ("task-12 - done.md", wave("TASK-12", "Done", &["TASK-3"])),
            ("task-1 - m.md", member("TASK-1", &["a.rs"])),
            ("task-2 - m.md", member("TASK-2", &["b.rs"])),
            ("task-3 - m.md", member("TASK-3", &["c.rs"])),
        ]);
        let run = |statuses: Vec<String>, json: bool| {
            let mut out = Vec::new();
            run_wave_list(
                &store,
                &cfg(),
                &WaveListOptions {
                    marker: DEFAULT_WAVE_MARKER.to_string(),
                    statuses,
                    json,
                },
                &mut out,
            )
            .expect("list");
            String::from_utf8(out).expect("utf8")
        };

        let plain = run(vec![], false);
        assert!(plain.contains("TASK-10"), "open waves are listed: {plain}");
        assert!(
            plain.contains("Done: 2 waves, hidden (pass --status Done to list them)"),
            "done waves collapse to a total: {plain}"
        );
        assert!(!plain.contains("TASK-11"), "{plain}");
        assert!(!plain.contains("TASK-12"), "{plain}");

        let filtered = run(vec!["Done".to_string()], false);
        assert!(filtered.contains("TASK-11"), "{filtered}");
        assert!(filtered.contains("TASK-12"), "{filtered}");
        assert!(
            !filtered.contains("waves, hidden"),
            "no total line when the rows are shown: {filtered}"
        );

        let json = run(vec![], true);
        assert!(json.contains("TASK-11"), "json keeps every row: {json}");
        assert!(json.contains("TASK-12"), "{json}");
    }

    #[test]
    fn members_unions_dependencies_and_parent_links() {
        let (dir, store) = scratch(&old_shape());
        let render = |store: &Store| {
            let mut out = Vec::new();
            run_wave_members(
                store,
                &cfg(),
                &WaveMembersOptions {
                    wave_id: "TASK-0119".to_string(),
                    json: false,
                },
                &mut out,
            )
            .expect("members");
            String::from_utf8(out).expect("utf8")
        };

        // Before migration TASK-0120 is a dependency and TASK-0121 is linked
        // only by the legacy assignee: both are members, and a runner working
        // an unmigrated tree must see both.
        let before = render(&store);
        assert!(before.contains("TASK-0120"), "{before}");
        assert!(before.contains("TASK-0121"), "{before}");

        migrate(&reopen(&dir), false, "y\n").expect("migrate");
        let after = render(&reopen(&dir));
        assert!(after.contains("TASK-0120") && after.contains("TASK-0121"));
    }

    /// `parent_task_id` holds one wave, so a member two waves claim through
    /// the assignee overload cannot be migrated: without this check both
    /// waves queue a write to the same file, the later one wins, and the
    /// earlier wave's assignee survives on a task parented elsewhere.
    #[test]
    fn member_claimed_by_two_waves_aborts_before_any_write() {
        let tasks = vec![
            (
                "task-0119 - wave-a.md",
                task("TASK-0119", &["code-review-wave"], &[], &[]),
            ),
            (
                "task-0130 - wave-b.md",
                task("TASK-0130", &["code-review-wave"], &[], &[]),
            ),
            (
                "task-0120 - member.md",
                task("TASK-0120", &["TASK-0119", "TASK-0130"], &[], &[]),
            ),
        ];
        let (dir, store) = scratch(&tasks);
        let before = doc_of(&dir, "task-0120 - member.md");

        let err = migrate(&store, false, "y\n").expect_err("two waves claim one member");
        let message = err.to_string();
        assert!(message.contains("TASK-0120"), "{message}");
        assert!(message.contains("TASK-0119"), "{message}");
        assert!(message.contains("TASK-0130"), "{message}");

        assert_eq!(
            doc_of(&dir, "task-0120 - member.md"),
            before,
            "preflight aborts before the first write"
        );
        assert_eq!(
            doc_of(&dir, "task-0119 - wave-a.md").frontmatter.assignees,
            vec!["code-review-wave".to_string()],
            "no wave is half-migrated either"
        );
    }

    #[test]
    fn members_reports_a_dependency_that_no_longer_exists() {
        let tasks = vec![(
            "task-0119 - wave.md",
            task("TASK-0119", &[], &["code-review-wave"], &["TASK-0404"]),
        )];
        let (_dir, store) = scratch(&tasks);
        let mut out = Vec::new();
        run_wave_members(
            &store,
            &cfg(),
            &WaveMembersOptions {
                wave_id: "TASK-0119".to_string(),
                json: false,
            },
            &mut out,
        )
        .expect("members");
        let text = String::from_utf8(out).expect("utf8");
        assert!(text.contains("Missing dependencies: TASK-0404"));
    }

    #[test]
    fn members_of_an_unknown_wave_names_the_id() {
        let (_dir, store) = scratch(&old_shape());
        let mut out = Vec::new();
        let err = run_wave_members(
            &store,
            &cfg(),
            &WaveMembersOptions {
                wave_id: "TASK-9999".to_string(),
                json: false,
            },
            &mut out,
        )
        .expect_err("unknown wave");
        assert!(err.to_string().contains("TASK-9999"));
    }

    #[test]
    fn task_id_shape_is_told_apart_from_a_person() {
        assert!(looks_like_task_id("TASK-0119"));
        assert!(!looks_like_task_id("code-review-wave"));
        assert!(!looks_like_task_id("rodrigo"));
    }

    /// A wave (marker label, members as dependencies) with a status.
    fn wave(id: &str, status: &str, members: &[&str]) -> String {
        task(id, &[], &["code-review-wave"], members)
            .replace("status: To Do", &format!("status: {status}"))
    }

    /// A member task touching `files`.
    fn member(id: &str, files: &[&str]) -> String {
        let mut src = task(id, &[], &[], &[]);
        src.truncate(src.len().saturating_sub("---\n".len()));
        src.push_str("modified_files:\n");
        for file in files {
            let _ = writeln!(src, "  - {file}");
        }
        src.push_str("---\n");
        src
    }

    fn overlap_tree() -> (tempfile::TempDir, Store) {
        scratch(&[
            (
                "task-10 - w.md",
                wave("TASK-10", "To Do", &["TASK-1", "TASK-2"]),
            ),
            ("task-11 - w.md", wave("TASK-11", "To Do", &["TASK-3"])),
            ("task-12 - w.md", wave("TASK-12", "To Do", &["TASK-4"])),
            ("task-13 - w.md", wave("TASK-13", "Done", &["TASK-5"])),
            ("task-1 - m.md", member("TASK-1", &["a.rs", "b.rs"])),
            ("task-2 - m.md", member("TASK-2", &["b.rs", "c.rs"])),
            ("task-3 - m.md", member("TASK-3", &["c.rs", "d.rs"])),
            ("task-4 - m.md", member("TASK-4", &["z.rs"])),
            // A closed wave's scope is never compared against.
            ("task-5 - m.md", member("TASK-5", &["a.rs"])),
        ])
    }

    fn overlap(store: &Store, ids: &[&str], json: bool) -> String {
        let mut out = Vec::new();
        run_wave_overlap(
            store,
            &cfg(),
            &WaveOverlapOptions {
                wave_ids: ids.iter().map(ToString::to_string).collect(),
                marker: DEFAULT_WAVE_MARKER.to_string(),
                json,
            },
            &mut out,
        )
        .expect("overlap");
        String::from_utf8(out).expect("utf8")
    }

    /// AC #1 and #2: every open wave's scope, the shared paths with each
    /// other open wave (closed waves ignored), and a least-overlapping-first
    /// merge order with ties broken by numeric id.
    #[test]
    fn overlap_reports_scope_shared_paths_and_merge_order() {
        let (_dir, store) = overlap_tree();
        let text = overlap(&store, &[], false);
        assert!(text.contains("  files: a.rs, b.rs, c.rs\n"), "got: {text}");
        assert!(text.contains("  overlaps TASK-11: c.rs\n"), "got: {text}");
        assert!(text.contains("  overlaps TASK-10: c.rs\n"), "got: {text}");
        assert!(!text.contains("TASK-13"), "closed wave compared: {text}");
        assert!(
            text.ends_with("Suggested merge order: TASK-12, TASK-10, TASK-11\n"),
            "got: {text}"
        );
    }

    /// An explicitly named wave is reported alone but still compared with
    /// every open wave.
    #[test]
    fn overlap_of_a_named_wave_compares_against_all_open_waves() {
        let (_dir, store) = overlap_tree();
        let text = overlap(&store, &["task-11"], false);
        assert!(text.starts_with("TASK-11 - "), "got: {text}");
        assert!(text.contains("  overlaps TASK-10: c.rs\n"), "got: {text}");
        assert!(
            text.ends_with("Suggested merge order: TASK-11\n"),
            "got: {text}"
        );
    }

    /// AC #3: the JSON envelope is versioned and parses.
    #[test]
    fn overlap_json_carries_a_schema_version() {
        let (_dir, store) = overlap_tree();
        let text = overlap(&store, &[], true);
        let value: serde_json::Value = serde_json::from_str(&text).expect("valid JSON");
        assert_eq!(value["schemaVersion"], 1);
        assert_eq!(value["kind"], "wave-overlap");
        assert_eq!(
            value["mergeOrder"],
            serde_json::json!(["TASK-12", "TASK-10", "TASK-11"])
        );
        assert_eq!(value["waves"][0]["overlaps"], serde_json::json!([]));
        assert_eq!(
            value["waves"][1]["overlaps"][0],
            serde_json::json!({"id": "TASK-11", "files": ["c.rs"]})
        );
    }

    #[test]
    fn overlap_of_an_unknown_wave_names_the_id() {
        let (_dir, store) = overlap_tree();
        let err = run_wave_overlap(
            &store,
            &cfg(),
            &WaveOverlapOptions {
                wave_ids: vec!["TASK-404".to_string()],
                marker: DEFAULT_WAVE_MARKER.to_string(),
                json: false,
            },
            &mut Vec::new(),
        )
        .expect_err("unknown");
        assert!(err.to_string().contains("TASK-404"));
    }

    #[test]
    fn id_order_is_numeric() {
        assert_eq!(id_order("TASK-999", "TASK-1000"), std::cmp::Ordering::Less);
        assert_eq!(id_order("task-2", "TASK-2"), std::cmp::Ordering::Equal);
    }
}
