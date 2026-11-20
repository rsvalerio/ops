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
use super::create::{create_task_locked, CreateOptions, Created};
use super::wave::{is_member, is_wave};
use crate::clock::UtcStamp;
use crate::config::BacklogConfig;
use crate::store::{same_task_id, Store, TaskEntry};

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

/// Create the wave parent and link every member, then print
/// `Created TASK-NNNN` and one `Linked TASK-MMMM` line per member.
///
/// The output is written only after every write has landed, so an output
/// failure (a closed pipe) is still returned but never cuts the linking
/// short.
///
/// Every check runs before the first write: each member must be a task in
/// `tasks/` (resolved with [`same_task_id`], so bare and padded forms work),
/// must not already belong to another wave — a `parent_task_id`, or any
/// link [`is_member`] honours to an open wave — and must not itself be a
/// wave ([`is_wave`]). A failure there writes nothing and names every
/// offending id. Only an I/O failure after that point can leave a partial
/// result, and its error says which members were linked and which were not.
///
/// The check, the parent's creation and the member rewrites run under the
/// store's allocation lock — the one `task edit` holds — so two concurrent
/// wave creates cannot both claim one member, and a member edited
/// meanwhile is rewritten from its current content, never a stale copy.
///
/// # Errors
///
/// No members were given, a member is missing / already in a wave / itself
/// a wave, a task file in `tasks/` does not parse, the clock is unreadable,
/// the lock cannot be taken, or a file cannot be written — errors name the
/// ids or paths involved.
pub fn run_wave_create<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &WaveCreateOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    wave_create_with(store, cfg, opts, &UtcStamp::now()?, out)
}

/// [`run_wave_create`] stamping every write from `stamp`, read once at the
/// command edge.
///
/// # Errors
///
/// As [`run_wave_create`], except the clock, which the caller already read.
fn wave_create_with<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &WaveCreateOptions,
    stamp: &UtcStamp,
    out: &mut W,
) -> anyhow::Result<()> {
    anyhow::ensure!(!opts.title.trim().is_empty(), "wave create needs a non-blank title");
    anyhow::ensure!(
        !opts.member_status.trim().is_empty(),
        "wave create needs a non-blank member status"
    );
    let wanted = dedup_ids(&opts.members);
    anyhow::ensure!(!wanted.is_empty(), "wave create needs at least one member");

    // Held across check → create → link: the snapshot the members are
    // checked and rewritten from stays current until the last write.
    let lock = store.lock_allocation()?;
    let entries = store.scan_tasks()?;
    let terminal = super::cleanup::terminal_status(&cfg.statuses);
    let members = resolve_members(entries, &wanted, &opts.marker, terminal)?;

    let member_ids: Vec<String> = members.iter().map(|m| m.doc.frontmatter.id.clone()).collect();
    let parent = CreateOptions {
        title: opts.title.clone(),
        description: opts.description.clone(),
        status: Some(opts.status.clone().unwrap_or_else(|| DEFAULT_WAVE_STATUS.to_string())),
        labels: vec![opts.marker.clone()],
        priority: opts.priority.clone(),
        plan: opts.plan.clone(),
        notes: opts.notes.clone(),
        modified_files: opts.modified_files.clone(),
        dependencies: member_ids.clone(),
        ..CreateOptions::default()
    };
    let wave_id = match create_task_locked(store, cfg, &parent, stamp, &lock)? {
        Created::New(id) => id,
        // `unless_exists` is never set above, so no existing task can match.
        Created::Exists(id) => anyhow::bail!("wave create unexpectedly matched {id}"),
    };

    // Every rewrite lands before the first progress line: a closed stdout
    // (`wave create … | head -1`) must not leave the parent listing members
    // that were never linked back to it.
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
    }
    writeln!(out, "Created {wave_id}").context("printing the created wave id")?;
    for id in &member_ids {
        writeln!(out, "Linked {id}").context("printing the linked member id")?;
    }
    Ok(())
}

/// `ids` trimmed, blank entries dropped, duplicates ([`same_task_id`])
/// collapsed to their first spelling, order kept.
fn dedup_ids(ids: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    for id in ids.iter().map(|id| id.trim()).filter(|id| !id.is_empty()) {
        if !kept.iter().any(|k| same_task_id(k, id)) {
            kept.push(id.to_string());
        }
    }
    kept
}

/// Resolve every wanted id to its entry in `tasks/`, or fail naming every
/// id that is missing, already in a wave, or a wave itself — all problems at
/// once, so one retry fixes them.
///
/// "Already in a wave" is a `parent_task_id`, or membership of an open wave
/// (any status but `terminal`) by the same links `wave members` reads
/// ([`is_member`]); "a wave" is [`is_wave`]. Sharing both predicates keeps
/// this check identical to what `wave list` / `wave members` report.
fn resolve_members(
    entries: Vec<TaskEntry>,
    wanted: &[String],
    marker: &str,
    terminal: Option<&str>,
) -> anyhow::Result<Vec<TaskEntry>> {
    let open_waves: Vec<&TaskEntry> = entries
        .iter()
        .filter(|entry| is_wave(entry, marker))
        .filter(|entry| {
            terminal.is_none_or(|t| !entry.doc.frontmatter.status.eq_ignore_ascii_case(t))
        })
        .collect();
    let mut found: Vec<usize> = Vec::with_capacity(wanted.len());
    let mut problems = Vec::new();
    for id in wanted {
        let Some((index, entry)) =
            entries.iter().enumerate().find(|(_, e)| same_task_id(&e.doc.frontmatter.id, id))
        else {
            problems.push(format!("{id}: not found in tasks/"));
            continue;
        };
        let fm = &entry.doc.frontmatter;
        if let Some(parent) = fm.extra_scalar("parent_task_id").filter(|p| !p.trim().is_empty()) {
            problems.push(format!("{}: already a member of {parent}", fm.id));
            continue;
        }
        if let Some(wave) = open_waves.iter().find(|wave| is_member(entry, &wave.doc)) {
            problems.push(format!("{}: already a member of {}", fm.id, wave.doc.frontmatter.id));
            continue;
        }
        if is_wave(entry, marker) {
            problems.push(format!("{}: is itself a wave", fm.id));
            continue;
        }
        if !found.contains(&index) {
            found.push(index);
        }
    }
    if !problems.is_empty() {
        anyhow::bail!("wave create refused, nothing written:\n  {}", problems.join("\n  "));
    }
    let mut pool: Vec<Option<TaskEntry>> = entries.into_iter().map(Some).collect();
    Ok(found.into_iter().filter_map(|index| pool.get_mut(index).and_then(Option::take)).collect())
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
        assert_eq!(wave.frontmatter.dependencies, vec!["TASK-0001", "TASK-0002"]);
        assert_eq!(wave.frontmatter.status, DEFAULT_WAVE_STATUS);

        for name in ["task-0001 - a.md", "task-0002 - b.md"] {
            let doc = doc_of(&dir, name);
            assert_eq!(doc.frontmatter.extra_scalar("parent_task_id"), Some("TASK-0003"));
            assert_eq!(doc.frontmatter.status, "To Do");
            assert!(doc.frontmatter.updated_date.is_some(), "updated_date bumped");
        }
    }

    #[test]
    fn any_bad_member_refuses_before_writing_anything() {
        let seeded = [
            ("task-0001 - a.md", member("TASK-0001", "")),
            ("task-0002 - b.md", member("TASK-0002", "parent_task_id: TASK-0009\n")),
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
        assert!(msg.contains("TASK-0002: already a member of TASK-0009"), "{msg}");
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
        let err =
            run_wave_create(&store, &BacklogConfig::default(), &opts(&[" ", ""]), &mut Vec::new())
                .expect_err("must refuse");
        assert!(format!("{err:#}").contains("at least one member"));
        assert!(file_names(&dir).is_empty());
    }

    /// The shared predicates: a marker label in another case is still a
    /// wave, a task listed in an open wave's `dependencies:` is already a
    /// member (no `parent_task_id` needed), and members resolve by the bare
    /// and padded forms `Store::find` accepts.
    #[test]
    fn membership_and_wave_checks_match_wave_list_and_members() {
        let seeded = [
            ("task-0001 - a.md", member("TASK-0001", "")),
            ("task-0002 - b.md", member("TASK-0002", "")),
            (
                "task-0003 - w.md",
                member("TASK-0003", "").replace("labels: []", "labels:\n  - Code-Review-Wave"),
            ),
            (
                "task-0004 - open-wave.md",
                member("TASK-0004", "")
                    .replace("labels: []", "labels:\n  - code-review-wave")
                    .replace("dependencies: []", "dependencies:\n  - TASK-0002"),
            ),
        ];
        let (dir, store) = scratch(&seeded);
        let before = file_names(&dir);
        let err = run_wave_create(
            &store,
            &BacklogConfig::default(),
            &opts(&["1", "task-2", "TASK-3"]),
            &mut Vec::new(),
        )
        .expect_err("must refuse");
        let msg = format!("{err:#}");
        assert!(msg.contains("TASK-0002: already a member of TASK-0004"), "{msg}");
        assert!(msg.contains("TASK-0003: is itself a wave"), "{msg}");
        assert!(!msg.contains("not found"), "bare/padded ids resolve: {msg}");
        assert_eq!(file_names(&dir), before, "nothing written");

        let mut out = Vec::new();
        run_wave_create(&store, &BacklogConfig::default(), &opts(&["1"]), &mut out)
            .expect("bare id resolves");
        assert_eq!(String::from_utf8(out).expect("utf8"), "Created TASK-0005\nLinked TASK-0001\n");
    }

    /// Accepts the first line, then fails every write like a closed pipe.
    struct ClosesAfterFirstLine {
        written: Vec<u8>,
    }

    impl Write for ClosesAfterFirstLine {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            if self.written.contains(&b'\n') {
                return Err(std::io::Error::from(std::io::ErrorKind::BrokenPipe));
            }
            self.written.extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// A progress line that cannot be written is reported, but every member
    /// is still linked: the output is cosmetic, the links are not.
    #[test]
    fn a_failing_writer_still_links_every_member() {
        let (dir, store) = scratch(&[
            ("task-0001 - a.md", member("TASK-0001", "")),
            ("task-0002 - b.md", member("TASK-0002", "")),
            ("task-0003 - c.md", member("TASK-0003", "")),
        ]);
        let mut out = ClosesAfterFirstLine { written: Vec::new() };
        let err = run_wave_create(
            &store,
            &BacklogConfig::default(),
            &opts(&["TASK-0001", "TASK-0002", "TASK-0003"]),
            &mut out,
        )
        .expect_err("the output failure is returned");
        assert!(format!("{err:#}").contains("printing the linked member id"), "{err:#}");
        assert_eq!(String::from_utf8(out.written).expect("utf8"), "Created TASK-0004\n");
        for name in ["task-0001 - a.md", "task-0002 - b.md", "task-0003 - c.md"] {
            let doc = doc_of(&dir, name);
            assert_eq!(doc.frontmatter.extra_scalar("parent_task_id"), Some("TASK-0004"), "{name}");
            assert_eq!(doc.frontmatter.status, "To Do", "{name}");
        }
    }

    /// Every write is stamped from the clock read once at the edge.
    #[test]
    fn parent_and_members_carry_the_injected_stamp() {
        let (dir, store) = scratch(&[("task-0001 - a.md", member("TASK-0001", ""))]);
        // 2026-03-04 05:06 UTC.
        let stamp = UtcStamp::from_unix_secs(1_772_600_760).expect("stamp");
        wave_create_with(
            &store,
            &BacklogConfig::default(),
            &opts(&["TASK-0001"]),
            &stamp,
            &mut Vec::new(),
        )
        .expect("wave create");
        let wave = doc_of(&dir, "task-0002 - code-review-plan-wave1.md");
        assert_eq!(wave.frontmatter.created_date, "2026-03-04 05:06");
        let linked = doc_of(&dir, "task-0001 - a.md");
        assert_eq!(linked.frontmatter.updated_date.as_deref(), Some("2026-03-04 05:06"));
    }

    /// Wave create and a member edit racing: whichever goes first, the
    /// member carries both its wave link and the appended note.
    #[test]
    fn racing_member_edit_and_wave_create_both_survive() {
        for _ in 0..20 {
            let (dir, store) = scratch(&[("task-0001 - a.md", member("TASK-0001", ""))]);
            let barrier = std::sync::Barrier::new(2);
            std::thread::scope(|scope| {
                scope.spawn(|| {
                    barrier.wait();
                    let edit = crate::cmd::edit::EditOptions {
                        task_id: "TASK-0001".to_string(),
                        append_notes: vec!["raced note".to_string()],
                        ..crate::cmd::edit::EditOptions::default()
                    };
                    crate::cmd::edit::run_edit(&store, &edit, &mut std::io::sink()).expect("edit");
                });
                barrier.wait();
                run_wave_create(
                    &store,
                    &BacklogConfig::default(),
                    &opts(&["TASK-0001"]),
                    &mut std::io::sink(),
                )
                .expect("wave create");
            });
            let doc = doc_of(&dir, "task-0001 - a.md");
            assert_eq!(doc.frontmatter.extra_scalar("parent_task_id"), Some("TASK-0002"));
            let notes = doc.body.notes().unwrap_or_default();
            assert!(notes.contains("raced note"), "the edit was lost: {notes}");
        }
    }
}
