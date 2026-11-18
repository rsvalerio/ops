//! `task create`: allocate the next id, render the file, write it.

use std::io::Write;

use anyhow::Context as _;

use crate::clock::UtcStamp;
use crate::cmd::cleanup::terminal_status;
use crate::cmd::{atomic_write_noclobber, DestinationTaken};
use crate::config::BacklogConfig;
use crate::model::{
    render_body, validate_list_item, validate_priority, AcItem, Body, Frontmatter, TaskDoc,
    DEDUP_KEY,
};
use crate::store::{format_task_id, main_task_file_name, AllocationLock, Store};

/// Everything `task create` can set; clap fills this in the CLI crate.
#[derive(Debug, Clone, Default)]
pub struct CreateOptions {
    pub title: String,
    pub description: Option<String>,
    pub assignees: Vec<String>,
    pub status: Option<String>,
    pub labels: Vec<String>,
    pub priority: Option<String>,
    pub ac: Vec<String>,
    pub dod: Vec<String>,
    pub modified_files: Vec<String>,
    pub plan: Option<String>,
    pub notes: Option<String>,
    pub dependencies: Vec<String>,
    /// Identity key: when an open task already carries it, create nothing.
    pub unless_exists: Option<String>,
    /// Parent task id: written as the new task's `parent_task_id` — the
    /// structural member→parent link. The parent must exist; the id is
    /// resolved and stored in its canonical form.
    pub parent: Option<String>,
}

/// How many allocation attempts `run_create` makes before conceding that
/// the backlog tree is too contended to place a task.
const CREATE_ATTEMPTS: usize = 8;

/// Create one task and print `Created TASK-NNNN`.
///
/// With [`CreateOptions::unless_exists`], the key is stored in the task's
/// `dedup_key` frontmatter, and when an open task in `tasks/` (any status
/// but the configured terminal one) already carries the same key, nothing
/// is created and `Exists TASK-NNNN` is printed instead — with
/// [`CreateOptions::parent`] requested, the existing task's
/// `parent_task_id` is left untouched and the `Exists` line says so.
///
/// Every create — keyed or not — scans for the next number and publishes
/// the file under the store's allocation lock, so concurrent creates never
/// hand out one id twice and concurrent keyed creates file exactly one
/// task. The file is staged and published atomically, so a concurrent
/// scan never sees it empty or half-written.
///
/// # Errors
///
/// The clock is unreadable, the key is blank, a label, assignee,
/// dependency, modified file or the priority cannot be stored in
/// frontmatter (the error names the field), the parent id resolves to
/// nothing, the allocation lock cannot be taken, a task file does not parse
/// during the key check, or the file cannot be written — errors name the
/// path.
pub fn run_create<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &CreateOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    match create_task(store, cfg, opts, &UtcStamp::now()?)? {
        Created::New(id) => writeln!(out, "Created {id}").context("printing the created task id"),
        Created::Exists(id) => {
            if opts.parent.is_some() {
                writeln!(out, "Exists {id}; parent_task_id left unchanged")
                    .context("printing the existing task id")
            } else {
                writeln!(out, "Exists {id}").context("printing the existing task id")
            }
        }
    }
}

/// What [`create_task`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Created {
    /// A new task file was written under this id.
    New(String),
    /// `unless_exists` matched this open task; nothing was written.
    Exists(String),
}

/// The core of [`run_create`], returning the id instead of printing it so
/// composite commands can act on the new task. `created_date` comes from
/// `stamp`, read once at the command edge.
///
/// # Errors
///
/// As [`run_create`], except the clock, which the caller already read.
pub fn create_task(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &CreateOptions,
    stamp: &UtcStamp,
) -> anyhow::Result<Created> {
    // Held until this function returns: the number scan and the publish
    // must be one critical section for every create, not just keyed ones.
    // The filename carries the title slug, so two unlocked creates with
    // different titles derive different paths for the same number and the
    // no-clobber publish cannot tell them apart. Keyed creates also need it
    // around the key check, or two runs both see "no task" and both file.
    let lock = store.lock_allocation()?;
    create_task_locked(store, cfg, opts, stamp, &lock)
}

/// [`create_task`] for a caller already holding the store's allocation
/// lock — `wave create`, which must check and link its members in the same
/// critical section. The `_held` guard is the proof: taking the lock a
/// second time from one process would block on itself.
///
/// # Errors
///
/// As [`create_task`], except the lock, which the caller already holds.
pub(crate) fn create_task_locked(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &CreateOptions,
    stamp: &UtcStamp,
    _held: &AllocationLock,
) -> anyhow::Result<Created> {
    validate_frontmatter_inputs(opts)?;
    // Resolve --parent before anything is written (the dedup check
    // included): the link must name an existing task, stored in the
    // parent's canonical id — `find` normalizes case, prefix padding and
    // the bare `N` / `N.M` forms. A blank parent is ignored, matching
    // `task edit --parent`.
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
    if let Some(key) = opts.unless_exists.as_deref() {
        anyhow::ensure!(!key.trim().is_empty(), "--unless-exists needs a non-blank key");
    }
    if let Some(key) = opts.unless_exists.as_deref() {
        if let Some(id) = open_task_with_key(store, cfg, key)? {
            return Ok(Created::Exists(id));
        }
    }
    // Publish without clobbering: a writer that does not take the lock (a
    // hand-copied file, an older binary) can still own the derived name.
    // On `DestinationTaken`, re-scan and take the next number.
    let mut attempts_left = CREATE_ATTEMPTS;
    loop {
        attempts_left = attempts_left.saturating_sub(1);
        let number = store.next_task_number()?;
        let id = format_task_id(&cfg.task_prefix, number, cfg.zero_padded_ids);
        let file_name = main_task_file_name(number, cfg.zero_padded_ids, &opts.title);
        let path = store.task_path(&file_name);
        let rendered = render_new_task(opts, cfg, &id, stamp, parent_id.as_deref());

        match atomic_write_noclobber(&path, &rendered) {
            Ok(()) => return Ok(Created::New(id)),
            Err(e) if e.downcast_ref::<DestinationTaken>().is_some() && attempts_left > 0 => {}
            Err(e) => return Err(e),
        }
    }
}

/// Refuse every option that lands in frontmatter as a bare token — list
/// items and the priority — when it cannot be written and read back
/// unchanged, before anything is locked or written.
///
/// # Errors
///
/// The first offending value; the error names its field.
fn validate_frontmatter_inputs(opts: &CreateOptions) -> anyhow::Result<()> {
    let lists: [(&str, &[String]); 4] = [
        ("assignee", &opts.assignees),
        ("labels", &opts.labels),
        ("dependencies", &opts.dependencies),
        ("modified_files", &opts.modified_files),
    ];
    for (field, items) in lists {
        for item in items {
            validate_list_item(field, item)?;
        }
    }
    if let Some(priority) = &opts.priority {
        validate_priority(priority)?;
    }
    Ok(())
}

/// The rendered task file for a would-be `id`: the frontmatter with every
/// option applied (defaults from `cfg`, the `dedup_key` extra when
/// `unless_exists` is set, `parent_task_id` when `parent` was resolved)
/// around the body with description, acceptance criteria, definition of
/// done, plan and notes.
fn render_new_task(
    opts: &CreateOptions,
    cfg: &BacklogConfig,
    id: &str,
    stamp: &UtcStamp,
    parent_id: Option<&str>,
) -> String {
    let mut frontmatter = Frontmatter {
        id: id.to_string(),
        title: opts.title.clone(),
        status: opts.status.clone().unwrap_or_else(|| cfg.default_status.clone()),
        assignees: opts.assignees.clone(),
        created_date: format!("{} {}", stamp.date, stamp.minutes),
        updated_date: None,
        labels: opts.labels.clone(),
        dependencies: opts.dependencies.clone(),
        priority: Some(opts.priority.clone().unwrap_or_else(|| "low".to_string())),
        modified_files: opts.modified_files.clone(),
        // The ordinal the backlog CLI assigns a freshly created parent
        // task, pinned by the create-review-tasks golden tests.
        ordinal: Some("1000".to_string()),
        extras: Vec::new(),
    };
    if let Some(key) = &opts.unless_exists {
        frontmatter.set_extra_scalar(DEDUP_KEY, key);
    }
    if let Some(parent) = parent_id {
        frontmatter.set_extra_scalar("parent_task_id", parent);
    }
    let ac = AcItem::unchecked_all(&opts.ac);
    let dod = AcItem::unchecked_all(&opts.dod);
    let body = render_body(
        opts.description.as_deref().unwrap_or(""),
        &ac,
        &dod,
        opts.plan.as_deref(),
        opts.notes.as_deref(),
    );
    TaskDoc { frontmatter, body: Body { raw: body } }.render()
}

/// The id of an open task in `tasks/` whose `dedup_key` equals `key`.
///
/// "Open" is any status but the terminal one (the last configured column);
/// relocated tasks in `completed/` and the archives are never open.
fn open_task_with_key(
    store: &Store,
    cfg: &BacklogConfig,
    key: &str,
) -> anyhow::Result<Option<String>> {
    let terminal = terminal_status(&cfg.statuses);
    Ok(store
        .scan_tasks()?
        .into_iter()
        .find(|entry| {
            let fm = &entry.doc.frontmatter;
            fm.dedup_key() == Some(key)
                && !terminal.is_some_and(|t| fm.status.eq_ignore_ascii_case(t))
        })
        .map(|entry| entry.doc.frontmatter.id))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch() -> (tempfile::TempDir, Store, BacklogConfig) {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".backlog").join("tasks")).expect("tasks dir");
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        (dir, store, BacklogConfig::default())
    }

    #[test]
    fn create_writes_the_cli_shape_and_prints_the_id() {
        let (dir, store, cfg) = scratch();
        let opts = CreateOptions {
            title: "OWASP-1: something".to_string(),
            description: Some("**File**: `crates/foo/src/lib.rs:42`".to_string()),
            status: Some("Triage".to_string()),
            labels: vec!["code-review-rust".to_string(), "security".to_string()],
            priority: Some("high".to_string()),
            ac: vec!["criterion one".to_string(), "criterion two".to_string()],
            modified_files: vec!["crates/foo/src/lib.rs".to_string()],
            ..CreateOptions::default()
        };
        let mut out = Vec::new();
        run_create(&store, &cfg, &opts, &mut out).expect("create");
        assert_eq!(String::from_utf8_lossy(&out), "Created TASK-0001\n");
        let written = std::fs::read_to_string(
            dir.path().join(".backlog/tasks/task-0001 - OWASP-1-something.md"),
        )
        .expect("read back");
        let doc = crate::model::TaskDoc::parse(&written).expect("re-parse");
        assert_eq!(doc.frontmatter.status, "Triage");
        assert_eq!(doc.frontmatter.priority.as_deref(), Some("high"));
        assert_eq!(doc.frontmatter.labels, vec!["code-review-rust", "security"]);
        assert_eq!(doc.frontmatter.modified_files, vec!["crates/foo/src/lib.rs"]);
        assert_eq!(doc.body.ac_items().len(), 2);
        assert!(written.contains("- [ ] #1 criterion one"));
    }

    /// `created_date` is the injected stamp, exactly.
    #[test]
    fn create_stamps_created_date_from_the_injected_clock() {
        let (_dir, store, cfg) = scratch();
        // 2026-03-04 05:06 UTC.
        let stamp = UtcStamp::from_unix_secs(1_772_600_760).expect("stamp");
        let opts = CreateOptions { title: "stamped".to_string(), ..CreateOptions::default() };
        let Created::New(id) = create_task(&store, &cfg, &opts, &stamp).expect("create") else {
            panic!("an unkeyed create always files");
        };
        let entry = store.find(&id).expect("find").expect("created");
        assert_eq!(entry.doc.frontmatter.created_date, "2026-03-04 05:06");
    }

    #[test]
    fn create_allocates_past_existing_ids_and_defaults_status() {
        let (dir, store, cfg) = scratch();
        std::fs::write(
            dir.path().join(".backlog/tasks/task-0010 - old.md"),
            "---\nid: TASK-0010\ntitle: 't'\nstatus: To Do\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\n---\n",
        )
        .expect("seed");
        let opts = CreateOptions { title: "next".to_string(), ..CreateOptions::default() };
        let mut out = Vec::new();
        run_create(&store, &cfg, &opts, &mut out).expect("create");
        assert_eq!(String::from_utf8_lossy(&out), "Created TASK-0011\n");
        let written =
            std::fs::read_to_string(dir.path().join(".backlog/tasks/task-0011 - next.md"))
                .expect("read back");
        let doc = crate::model::TaskDoc::parse(&written).expect("re-parse");
        assert_eq!(doc.frontmatter.status, "Triage", "config default_status");
        assert_eq!(doc.frontmatter.priority.as_deref(), Some("low"));
    }

    fn keyed(title: &str, key: &str) -> CreateOptions {
        CreateOptions {
            title: title.to_string(),
            unless_exists: Some(key.to_string()),
            ..CreateOptions::default()
        }
    }

    fn create_to_string(store: &Store, cfg: &BacklogConfig, opts: &CreateOptions) -> String {
        let mut out = Vec::new();
        run_create(store, cfg, opts, &mut out).expect("create");
        String::from_utf8(out).expect("utf8")
    }

    fn task_count(dir: &tempfile::TempDir) -> usize {
        std::fs::read_dir(dir.path().join(".backlog/tasks"))
            .expect("tasks dir")
            .filter(|e| e.as_ref().is_ok_and(|e| e.path().extension().is_some_and(|x| x == "md")))
            .count()
    }

    const KEY: &str =
        "clippy::needless_pass_by_value|ops@0.65.0|crates/cli/src/x.rs:10:5|it's: msg";

    #[test]
    fn unless_exists_reports_the_open_task_instead_of_filing() {
        let (dir, store, cfg) = scratch();
        assert_eq!(create_to_string(&store, &cfg, &keyed("first", KEY)), "Created TASK-0001\n");
        assert_eq!(create_to_string(&store, &cfg, &keyed("again", KEY)), "Exists TASK-0001\n");
        assert_eq!(task_count(&dir), 1);
        // A different key files normally.
        assert_eq!(
            create_to_string(&store, &cfg, &keyed("other", "other-key")),
            "Created TASK-0002\n"
        );
        let written =
            std::fs::read_to_string(dir.path().join(".backlog/tasks/task-0001 - first.md"))
                .expect("read back");
        let doc = crate::model::TaskDoc::parse(&written).expect("re-parse");
        assert_eq!(doc.frontmatter.dedup_key(), Some(KEY), "key round-trips");
    }

    #[test]
    fn unless_exists_files_again_once_the_keyed_task_is_done() {
        let (dir, store, cfg) = scratch();
        assert_eq!(create_to_string(&store, &cfg, &keyed("first", KEY)), "Created TASK-0001\n");
        let path = dir.path().join(".backlog/tasks/task-0001 - first.md");
        let src = std::fs::read_to_string(&path).expect("read");
        std::fs::write(&path, src.replace("status: Triage", "status: Done")).expect("mark done");
        assert_eq!(create_to_string(&store, &cfg, &keyed("second", KEY)), "Created TASK-0002\n");
        assert_eq!(task_count(&dir), 2);
    }

    #[test]
    fn unless_exists_rejects_a_blank_key() {
        let (dir, store, cfg) = scratch();
        let mut out = Vec::new();
        let err = run_create(&store, &cfg, &keyed("t", "  "), &mut out).expect_err("blank key");
        assert!(format!("{err:#}").contains("non-blank"));
        assert_eq!(task_count(&dir), 0);
    }

    #[test]
    fn concurrent_keyed_creates_file_exactly_one_task() {
        let (dir, store, cfg) = scratch();
        let barrier = std::sync::Barrier::new(8);
        let outputs: Vec<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..8)
                .map(|i| {
                    let (store, cfg, barrier) = (&store, &cfg, &barrier);
                    scope.spawn(move || {
                        barrier.wait();
                        create_to_string(store, cfg, &keyed(&format!("racer {i}"), KEY))
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().expect("join")).collect()
        });
        assert_eq!(task_count(&dir), 1);
        assert_eq!(outputs.iter().filter(|o| o.starts_with("Created ")).count(), 1);
        assert_eq!(outputs.iter().filter(|o| *o == "Exists TASK-0001\n").count(), 7);
    }

    /// Unkeyed creates with distinct titles derive distinct filenames, so
    /// only the allocation lock keeps them from sharing a number.
    #[test]
    fn concurrent_unkeyed_creates_get_distinct_ids() {
        const RACERS: usize = 12;
        let (dir, store, cfg) = scratch();
        let barrier = std::sync::Barrier::new(RACERS);
        let ids: std::collections::BTreeSet<String> = std::thread::scope(|scope| {
            let handles: Vec<_> = (0..RACERS)
                .map(|i| {
                    let (store, cfg, barrier) = (&store, &cfg, &barrier);
                    scope.spawn(move || {
                        barrier.wait();
                        let opts = CreateOptions {
                            title: format!("title {i}"),
                            ..CreateOptions::default()
                        };
                        let stamp = UtcStamp::now().expect("clock");
                        match create_task(store, cfg, &opts, &stamp).expect("create") {
                            Created::New(id) => id,
                            Created::Exists(id) => panic!("unkeyed create reported {id}"),
                        }
                    })
                })
                .collect();
            handles.into_iter().map(|h| h.join().expect("join")).collect()
        });
        assert_eq!(ids.len(), RACERS, "every create got its own id: {ids:?}");
        assert_eq!(task_count(&dir), RACERS);
        let ids_on_disk: std::collections::BTreeSet<String> = store
            .scan_tasks()
            .expect("every published file parses")
            .into_iter()
            .map(|entry| entry.doc.frontmatter.id)
            .collect();
        assert_eq!(ids_on_disk, ids);
    }

    /// A list item or priority that would inject a key or leave an
    /// unparseable file is refused before anything is written.
    #[test]
    fn hostile_list_items_and_priority_are_refused_before_writing() {
        let (dir, store, cfg) = scratch();
        for opts in [
            CreateOptions {
                title: "y".to_string(),
                labels: vec!["a\nstatus: Done".to_string()],
                ..CreateOptions::default()
            },
            CreateOptions {
                title: "x".to_string(),
                priority: Some(String::new()),
                ..CreateOptions::default()
            },
        ] {
            let mut out = Vec::new();
            run_create(&store, &cfg, &opts, &mut out).expect_err("must refuse");
        }
        assert_eq!(task_count(&dir), 0);
        store.scan_tasks().expect("the backlog stays scannable");
    }

    fn seed_parent(dir: &tempfile::TempDir) {
        std::fs::write(
            dir.path().join(".backlog/tasks/task-0010 - parent.md"),
            "---\nid: TASK-0010\ntitle: 'parent'\nstatus: To Do\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\n---\n",
        )
        .expect("seed parent");
    }

    /// `--parent` writes `parent_task_id` in the new task's frontmatter,
    /// resolved to the parent's canonical id whatever form was passed.
    #[test]
    fn parent_links_the_new_task_in_the_same_call() {
        let (dir, store, cfg) = scratch();
        seed_parent(&dir);
        for (i, passed) in ["TASK-0010", "10"].iter().enumerate() {
            let opts = CreateOptions {
                title: format!("child {i}"),
                parent: Some((*passed).to_string()),
                ..CreateOptions::default()
            };
            let mut out = Vec::new();
            run_create(&store, &cfg, &opts, &mut out).expect("create");
            let number = 11 + i; // the seed parent holds number 10
            let written = std::fs::read_to_string(
                dir.path().join(format!(".backlog/tasks/task-{number:04} - child-{i}.md")),
            )
            .expect("read back");
            let doc = crate::model::TaskDoc::parse(&written).expect("re-parse");
            assert_eq!(
                doc.frontmatter.extra_scalar("parent_task_id"),
                Some("TASK-0010"),
                "the link lands in canonical form ({passed} passed)"
            );
        }
        assert_eq!(task_count(&dir), 3);
    }

    /// A parent id that resolves to nothing is refused before anything is
    /// written — no task file, not even with `--unless-exists` pending.
    #[test]
    fn an_unknown_parent_is_refused_before_anything_is_written() {
        let (dir, store, cfg) = scratch();
        seed_parent(&dir);
        for opts in [
            CreateOptions {
                title: "orphan".to_string(),
                parent: Some("TASK-9999".to_string()),
                ..CreateOptions::default()
            },
            CreateOptions {
                title: "orphan".to_string(),
                parent: Some("TASK-9999".to_string()),
                unless_exists: Some(KEY.to_string()),
                ..CreateOptions::default()
            },
        ] {
            let mut out = Vec::new();
            let err = run_create(&store, &cfg, &opts, &mut out).expect_err("must refuse");
            let msg = format!("{err:#}");
            assert!(
                msg.contains("TASK-9999") && msg.contains("not found"),
                "the error names the parent, got: {msg}"
            );
        }
        assert_eq!(task_count(&dir), 1, "only the seed parent exists");
    }

    /// With `--unless-exists`, an `Exists` hit re-parents nothing and says
    /// so on the `Exists` line.
    #[test]
    fn an_exists_hit_leaves_the_existing_tasks_parent_untouched() {
        let (dir, store, cfg) = scratch();
        seed_parent(&dir);
        // The keyed task already carries a different parent.
        std::fs::write(
            dir.path().join(".backlog/tasks/task-0011 - keyed.md"),
            "---\nid: TASK-0011\ntitle: 'keyed'\nstatus: To Do\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\ndedup_key: k\n---\n",
        )
        .expect("seed keyed");
        let opts = CreateOptions {
            title: "again".to_string(),
            unless_exists: Some("k".to_string()),
            parent: Some("TASK-0010".to_string()),
            ..CreateOptions::default()
        };
        let mut out = Vec::new();
        run_create(&store, &cfg, &opts, &mut out).expect("create");
        assert_eq!(
            String::from_utf8_lossy(&out),
            "Exists TASK-0011; parent_task_id left unchanged\n"
        );
        let written =
            std::fs::read_to_string(dir.path().join(".backlog/tasks/task-0011 - keyed.md"))
                .expect("read back");
        let doc = crate::model::TaskDoc::parse(&written).expect("re-parse");
        assert!(
            doc.frontmatter.extra_scalar("parent_task_id").is_none(),
            "the existing task's parent must stay unset"
        );
        assert_eq!(task_count(&dir), 2);
    }
}
