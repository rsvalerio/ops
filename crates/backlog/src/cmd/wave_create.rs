//! `wave create`: create a wave parent and link its members in one step.
//!
//! One invocation writes every link of the wave convention (see
//! [`super::wave`]) at once: the parent's marker label and `dependencies:`,
//! and each member's `parent_task_id` and status. Every check runs before
//! the first write, so a refused create leaves the tree untouched (see
//! [`run_wave_create`]).

use std::io::Write;

use anyhow::Context as _;

use super::atomic_write;
use super::create::{create_task, CreateOptions, Created};
use crate::clock::UtcStamp;
use crate::config::BacklogConfig;
use crate::store::{Store, TaskEntry};

/// The status a new wave parent takes when none is given: a wave is
/// created ready to be claimed.
pub const DEFAULT_WAVE_STATUS: &str = "To Do";

/// Arguments of `wave create`.
#[derive(Debug, Clone, Default)]
pub struct WaveCreateOptions {
    /// The wave parent's title (e.g. `code-review-plan-wave29`).
    pub title: String,
    /// Member task ids; duplicates (case-insensitive) collapse to one.
    pub members: Vec<String>,
    /// The marker label identifying a wave.
    pub marker: String,
    /// The wave parent's status; [`DEFAULT_WAVE_STATUS`] when unset.
    pub status: Option<String>,
    /// The status every member is flipped to.
    pub member_status: String,
    pub description: Option<String>,
    pub priority: Option<String>,
    pub plan: Option<String>,
    pub notes: Option<String>,
    pub modified_files: Vec<String>,
}

/// Create the wave parent and link every member, printing
/// `Created TASK-NNNN` and one `Linked TASK-MMMM` line per member.
///
/// Every check runs before the first write: each member must be a task in
/// `tasks/`, must not already belong to another wave, and must not itself be
/// a wave. A failure there writes nothing and names every offending id. Only
/// an I/O failure after that point can leave a partial result, and its error
/// says which members were linked and which were not.
///
/// # Errors
///
/// No members were given, a member is missing / already parented / itself a
/// wave, a task file in `tasks/` does not parse, the clock is unreadable, or
/// a file cannot be written — errors name the ids or paths involved.
pub fn run_wave_create<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &WaveCreateOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    anyhow::ensure!(
        !opts.title.trim().is_empty(),
        "wave create needs a non-blank title"
    );
    anyhow::ensure!(
        !opts.member_status.trim().is_empty(),
        "wave create needs a non-blank member status"
    );
    let wanted = dedup_ids(&opts.members);
    anyhow::ensure!(!wanted.is_empty(), "wave create needs at least one member");

    let entries = store.scan_tasks()?;
    let members = resolve_members(entries, &wanted, &opts.marker)?;
    let stamp = UtcStamp::now()?;

    let member_ids: Vec<String> = members
        .iter()
        .map(|m| m.doc.frontmatter.id.clone())
        .collect();
    let parent = CreateOptions {
        title: opts.title.clone(),
        description: opts.description.clone(),
        status: Some(
            opts.status
                .clone()
                .unwrap_or_else(|| DEFAULT_WAVE_STATUS.to_string()),
        ),
        labels: vec![opts.marker.clone()],
        priority: opts.priority.clone(),
        plan: opts.plan.clone(),
        notes: opts.notes.clone(),
        modified_files: opts.modified_files.clone(),
        dependencies: member_ids.clone(),
        ..CreateOptions::default()
    };
    let wave_id = match create_task(store, cfg, &parent)? {
        Created::New(id) => id,
        // `unless_exists` is never set above, so no existing task can match.
        Created::Exists(id) => anyhow::bail!("wave create unexpectedly matched {id}"),
    };
    writeln!(out, "Created {wave_id}").context("printing the created wave id")?;

    let updated = format!("{} {}", stamp.date, stamp.minutes);
    for (done, member) in members.into_iter().enumerate() {
        let mut doc = member.doc;
        let id = doc.frontmatter.id.clone();
        doc.frontmatter.set_extra_scalar("parent_task_id", &wave_id);
        doc.frontmatter.status.clone_from(&opts.member_status);
        doc.frontmatter.updated_date = Some(updated.clone());
        atomic_write(&member.path, &doc.render()).with_context(|| {
            let (linked, unlinked) = member_ids.split_at(done.min(member_ids.len()));
            format!(
                "wave {wave_id} was created, but linking {id} failed; linked: [{}], not linked: [{}]",
                linked.join(", "),
                unlinked.join(", ")
            )
        })?;
        writeln!(out, "Linked {id}").context("printing the linked member id")?;
    }
    Ok(())
}

/// `ids` trimmed, blank entries dropped, case-insensitive duplicates
/// collapsed to their first spelling, order kept.
fn dedup_ids(ids: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for id in ids.iter().map(|id| id.trim()).filter(|id| !id.is_empty()) {
        if !kept.iter().any(|k| k.eq_ignore_ascii_case(id)) {
            kept.push(id.to_string());
        }
    }
    kept
}

/// Resolve every wanted id to its entry in `tasks/`, or fail naming every
/// id that is missing, already in a wave, or a wave itself — all problems at
/// once, so one retry fixes them.
fn resolve_members(
    entries: Vec<TaskEntry>,
    wanted: &[String],
    marker: &str,
) -> anyhow::Result<Vec<TaskEntry>> {
    let mut pool: Vec<Option<TaskEntry>> = entries.into_iter().map(Some).collect();
    let mut found = Vec::with_capacity(wanted.len());
    let mut problems = Vec::new();
    for id in wanted {
        let slot = pool.iter_mut().find(|slot| {
            slot.as_ref()
                .is_some_and(|e| e.doc.frontmatter.id.eq_ignore_ascii_case(id))
        });
        let Some(entry) = slot.and_then(Option::take) else {
            problems.push(format!("{id}: not found in tasks/"));
            continue;
        };
        let fm = &entry.doc.frontmatter;
        if let Some(parent) = fm
            .extra_scalar("parent_task_id")
            .filter(|p| !p.trim().is_empty())
        {
            problems.push(format!("{}: already a member of {parent}", fm.id));
            continue;
        }
        if fm.labels.iter().any(|l| l == marker) || fm.assignees.iter().any(|a| a == marker) {
            problems.push(format!("{}: is itself a wave", fm.id));
            continue;
        }
        found.push(entry);
    }
    if !problems.is_empty() {
        anyhow::bail!(
            "wave create refused, nothing written:\n  {}",
            problems.join("\n  ")
        );
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cmd::wave::DEFAULT_WAVE_MARKER;
    use crate::model::TaskDoc;

    fn member(id: &str, extra: &str) -> String {
        format!(
            "---\nid: {id}\ntitle: 'finding {id}'\nstatus: Triage\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\n{extra}---\n"
        )
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

    fn doc_of(dir: &tempfile::TempDir, name: &str) -> TaskDoc {
        let path = dir.path().join(".backlog").join("tasks").join(name);
        TaskDoc::parse(&std::fs::read_to_string(path).expect("read")).expect("parse")
    }

    fn opts(members: &[&str]) -> WaveCreateOptions {
        WaveCreateOptions {
            title: "code-review-plan-wave1".to_string(),
            members: members.iter().map(|m| (*m).to_string()).collect(),
            marker: DEFAULT_WAVE_MARKER.to_string(),
            member_status: "To Do".to_string(),
            ..WaveCreateOptions::default()
        }
    }

    fn file_names(dir: &tempfile::TempDir) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir.path().join(".backlog/tasks"))
            .expect("read tasks")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn creates_the_parent_and_links_every_member() {
        let (dir, store) = scratch(&[
            ("task-0001 - a.md", member("TASK-0001", "")),
            ("task-0002 - b.md", member("TASK-0002", "")),
        ]);
        let mut out = Vec::new();
        // Lower-case and duplicated ids resolve to the canonical id once.
        run_wave_create(
            &store,
            &BacklogConfig::default(),
            &opts(&["task-0001", "TASK-0002", "TASK-0001"]),
            &mut out,
        )
        .expect("wave create");
        assert_eq!(
            String::from_utf8(out).expect("utf8"),
            "Created TASK-0003\nLinked TASK-0001\nLinked TASK-0002\n"
        );

        let wave = doc_of(&dir, "task-0003 - code-review-plan-wave1.md");
        assert_eq!(wave.frontmatter.labels, vec![DEFAULT_WAVE_MARKER]);
        assert_eq!(
            wave.frontmatter.dependencies,
            vec!["TASK-0001", "TASK-0002"]
        );
        assert_eq!(wave.frontmatter.status, DEFAULT_WAVE_STATUS);

        for name in ["task-0001 - a.md", "task-0002 - b.md"] {
            let doc = doc_of(&dir, name);
            assert_eq!(
                doc.frontmatter.extra_scalar("parent_task_id"),
                Some("TASK-0003")
            );
            assert_eq!(doc.frontmatter.status, "To Do");
            assert!(
                doc.frontmatter.updated_date.is_some(),
                "updated_date bumped"
            );
        }
    }

    #[test]
    fn any_bad_member_refuses_before_writing_anything() {
        let seeded = [
            ("task-0001 - a.md", member("TASK-0001", "")),
            (
                "task-0002 - b.md",
                member("TASK-0002", "parent_task_id: TASK-0009\n"),
            ),
            (
                "task-0003 - w.md",
                member("TASK-0003", "").replace("labels: []", "labels:\n  - code-review-wave"),
            ),
        ];
        let (dir, store) = scratch(&seeded);
        let before = file_names(&dir);
        let mut out = Vec::new();
        let err = run_wave_create(
            &store,
            &BacklogConfig::default(),
            &opts(&["TASK-0001", "TASK-0002", "TASK-0003", "TASK-0042"]),
            &mut out,
        )
        .expect_err("must refuse");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("TASK-0002: already a member of TASK-0009"),
            "{msg}"
        );
        assert!(msg.contains("TASK-0003: is itself a wave"), "{msg}");
        assert!(msg.contains("TASK-0042: not found"), "{msg}");
        assert!(out.is_empty(), "nothing printed");
        assert_eq!(file_names(&dir), before, "no parent file created");
        for (name, body) in &seeded {
            let now = std::fs::read_to_string(dir.path().join(".backlog/tasks").join(name))
                .expect("read");
            assert_eq!(&now, body, "{name} untouched");
        }
    }

    #[test]
    fn no_members_is_refused() {
        let (dir, store) = scratch(&[]);
        let err = run_wave_create(
            &store,
            &BacklogConfig::default(),
            &opts(&[" ", ""]),
            &mut Vec::new(),
        )
        .expect_err("must refuse");
        assert!(format!("{err:#}").contains("at least one member"));
        assert!(file_names(&dir).is_empty());
    }
}
