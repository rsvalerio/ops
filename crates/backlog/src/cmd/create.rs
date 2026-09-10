//! `task create`: allocate the next id, render the file, write it.

use std::io::Write;

use anyhow::Context as _;

use crate::clock::UtcStamp;
use crate::config::BacklogConfig;
use crate::model::{render_body, AcItem, Body, Frontmatter, TaskDoc};
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
}

/// How many allocation attempts `run_create` makes before conceding that
/// the backlog tree is too contended to place a task.
const CREATE_ATTEMPTS: usize = 8;

/// Create one task and print `Created TASK-NNNN`.
///
/// # Errors
///
/// The clock is unreadable (ERR-6), or the file cannot be written — the
/// error names the path.
pub fn run_create<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &CreateOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    let stamp = UtcStamp::now()?;
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

        let frontmatter = Frontmatter {
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
        writeln!(out, "Created {id}").context("printing the created task id")?;
        return Ok(());
    }
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
}
