//! `task create`: allocate the next id, render the file, write it.

use std::io::Write;

use anyhow::Context as _;

use crate::clock::UtcStamp;
use crate::cmd::cleanup::terminal_status;
use crate::config::BacklogConfig;
use crate::model::{render_body, AcItem, Body, Frontmatter, TaskDoc, DEDUP_KEY};
use crate::store::{format_task_id, main_task_file_name, Store};

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
}

/// How many allocation attempts `run_create` makes before conceding that
/// the backlog tree is too contended to place a task.
const CREATE_ATTEMPTS: usize = 8;

/// Create one task and print `Created TASK-NNNN`.
///
/// With [`CreateOptions::unless_exists`], the key is stored in the task's
/// `dedup_key` frontmatter, and when an open task in `tasks/` (any status
/// but the configured terminal one) already carries the same key, nothing
/// is created and `Exists TASK-NNNN` is printed instead. The check and the
/// write happen under the store's allocation lock, so concurrent keyed
/// creates file exactly one task.
///
/// # Errors
///
/// The clock is unreadable (ERR-6), the key is blank, the allocation lock
/// cannot be taken, a task file does not parse during the key check, or the
/// file cannot be written — errors name the path.
pub fn run_create<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &CreateOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    match create_task(store, cfg, opts)? {
        Created::New(id) => writeln!(out, "Created {id}").context("printing the created task id"),
        Created::Exists(id) => {
            writeln!(out, "Exists {id}").context("printing the existing task id")
        }
    }
}

/// What [`create_task`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Created {
    /// A new task file was written under this id.
    New(String),
    /// `unless_exists` matched this open task; nothing was written.
    Exists(String),
}

/// The core of [`run_create`], returning the id instead of printing it so
/// composite commands (`wave create`) can act on the new task.
///
/// # Errors
///
/// As [`run_create`].
pub(crate) fn create_task(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &CreateOptions,
) -> anyhow::Result<Created> {
    let stamp = UtcStamp::now()?;
    // Held until this function returns: the key check and the file write
    // must be one critical section, or two runs both see "no task" and both
    // file.
    let _lock = match opts.unless_exists.as_deref() {
        Some(key) => {
            anyhow::ensure!(
                !key.trim().is_empty(),
                "--unless-exists needs a non-blank key"
            );
            let lock = store.lock_allocation()?;
            if let Some(id) = open_task_with_key(store, cfg, key)? {
                return Ok(Created::Exists(id));
            }
            Some(lock)
        }
        None => None,
    };
    // The create must be exclusive: two concurrent runs can scan the same
    // highest number and derive the same path, and a plain `fs::write`
    // would let the later run truncate the earlier task. `File::create_new`
    // refuses to clobber; on `AlreadyExists`, re-scan and take the next
    // number (the same invariant `create-review-tasks` documents).
    let mut attempts_left = CREATE_ATTEMPTS;
    loop {
        attempts_left = attempts_left.saturating_sub(1);
        let number = store.next_task_number();
        let id = format_task_id(&cfg.task_prefix, number, cfg.zero_padded_ids);
        let file_name = main_task_file_name(number, cfg.zero_padded_ids, &opts.title);
        let path = store.task_path(&file_name);

        let mut frontmatter = Frontmatter {
            id: id.clone(),
            title: opts.title.clone(),
            status: opts
                .status
                .clone()
                .unwrap_or_else(|| cfg.default_status.clone()),
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
        let ac = AcItem::unchecked_all(&opts.ac);
        let dod = AcItem::unchecked_all(&opts.dod);
        let body = render_body(
            opts.description.as_deref().unwrap_or(""),
            &ac,
            &dod,
            opts.plan.as_deref(),
            opts.notes.as_deref(),
        );
        let rendered = TaskDoc {
            frontmatter,
            body: Body { raw: body },
        }
        .render();

        let mut handle = match std::fs::File::create_new(&path) {
            Ok(handle) => handle,
            // Another run claimed this number between scan and create;
            // try the next one until the attempts run out.
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists && attempts_left > 0 => {
                continue;
            }
            Err(e) => {
                return Err(e).with_context(|| format!("creating {}", path.display()));
            }
        };
        handle
            .write_all(rendered.as_bytes())
            .with_context(|| format!("writing {}", path.display()))?;
        return Ok(Created::New(id));
    }
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
            dir.path()
                .join(".backlog/tasks/task-0001 - OWASP-1-something.md"),
        )
        .expect("read back");
        let doc = crate::model::TaskDoc::parse(&written).expect("re-parse");
        assert_eq!(doc.frontmatter.status, "Triage");
        assert_eq!(doc.frontmatter.priority.as_deref(), Some("high"));
        assert_eq!(doc.frontmatter.labels, vec!["code-review-rust", "security"]);
        assert_eq!(
            doc.frontmatter.modified_files,
            vec!["crates/foo/src/lib.rs"]
        );
        assert_eq!(doc.body.ac_items().len(), 2);
        assert!(written.contains("- [ ] #1 criterion one"));
    }

    #[test]
    fn create_allocates_past_existing_ids_and_defaults_status() {
        let (dir, store, cfg) = scratch();
        std::fs::write(
            dir.path().join(".backlog/tasks/task-0010 - old.md"),
            "---\nid: TASK-0010\ntitle: 't'\nstatus: To Do\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\n---\n",
        )
        .expect("seed");
        let opts = CreateOptions {
            title: "next".to_string(),
            ..CreateOptions::default()
        };
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
            .filter(|e| {
                e.as_ref()
                    .is_ok_and(|e| e.path().extension().is_some_and(|x| x == "md"))
            })
            .count()
    }

    const KEY: &str =
        "clippy::needless_pass_by_value|ops@0.65.0|crates/cli/src/x.rs:10:5|it's: msg";

    #[test]
    fn unless_exists_reports_the_open_task_instead_of_filing() {
        let (dir, store, cfg) = scratch();
        assert_eq!(
            create_to_string(&store, &cfg, &keyed("first", KEY)),
            "Created TASK-0001\n"
        );
        assert_eq!(
            create_to_string(&store, &cfg, &keyed("again", KEY)),
            "Exists TASK-0001\n"
        );
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
        assert_eq!(
            create_to_string(&store, &cfg, &keyed("first", KEY)),
            "Created TASK-0001\n"
        );
        let path = dir.path().join(".backlog/tasks/task-0001 - first.md");
        let src = std::fs::read_to_string(&path).expect("read");
        std::fs::write(&path, src.replace("status: Triage", "status: Done")).expect("mark done");
        assert_eq!(
            create_to_string(&store, &cfg, &keyed("second", KEY)),
            "Created TASK-0002\n"
        );
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
            handles
                .into_iter()
                .map(|h| h.join().expect("join"))
                .collect()
        });
        assert_eq!(task_count(&dir), 1);
        assert_eq!(
            outputs.iter().filter(|o| o.starts_with("Created ")).count(),
            1
        );
        assert_eq!(
            outputs
                .iter()
                .filter(|o| *o == "Exists TASK-0001\n")
                .count(),
            7
        );
    }
}
