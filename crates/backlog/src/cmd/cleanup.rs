//! `cleanup`: move terminal-status tasks older than a cutoff from `tasks/`
//! to `completed/` — the non-interactive shape of the backlog CLI's cleanup
//! age menu, with its confirmation prompt kept.
//!
//! The npm CLI asks interactively for an age (1 day … 1 year) and confirms
//! before moving; interactive TUIs are out of scope here, so the age arrives
//! as `--older-than <days>` and the preview is `--dry-run`. The move itself
//! still asks — `Move N tasks to completed folder? [y/N]` on stdin, default
//! No, like the backlog CLI's confirm. Semantics otherwise match: the
//! terminal status is the last entry of the configured `statuses`, a task's
//! age reads `updated_date` with `created_date` as fallback, and the file is
//! moved unchanged into `completed/`. Git staging stays with the caller (the
//! skills own their `chore(backlog)` commits).

use std::io::Write;
use std::path::Path;

use anyhow::Context as _;
use chrono::{DateTime, Days, NaiveDateTime, TimeZone as _, Utc};

use crate::config::BacklogConfig;
use crate::store::{Store, TaskEntry};

/// Filters for `cleanup`.
#[derive(Debug, Clone)]
pub struct CleanupOptions {
    /// Move tasks whose date is strictly older than this many calendar days.
    pub older_than_days: u32,
    /// Report what would move without touching the tree.
    pub dry_run: bool,
}

/// The environment cleanup runs against: the clock the age cutoff reads and
/// the stream the confirmation answer comes from. Production reads the host
/// clock and stdin; tests inject fixed values through the same struct —
/// grouping them also keeps the handler within clippy's argument budget.
struct CleanupEnv<'a> {
    now: DateTime<Utc>,
    input: &'a mut dyn std::io::BufRead,
}

/// Run cleanup against the host clock and stdin.
///
/// # Errors
///
/// A task file in `tasks/` does not parse (the error names the file), or
/// writing `out` failed, or the confirmation answer could not be read from
/// stdin.
pub fn run_cleanup<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &CleanupOptions,
    out: &mut W,
) -> anyhow::Result<()> {
    // The workspace chrono carries no `clock` feature; the wall clock is read
    // as a SystemTime and converted, exactly like `clock::UtcStamp`.
    let now: DateTime<Utc> = std::time::SystemTime::now().into();
    let mut input = std::io::stdin().lock();
    let mut env = CleanupEnv {
        now,
        input: &mut input,
    };
    let result = cleanup_with(store, cfg, opts, &mut env, out);
    // Release the stdin lock before returning (significant_drop_tightening):
    // nothing after the handler needs it.
    drop(input);
    result
}

/// [`run_cleanup`] against an explicit environment; the seam the age-filter
/// and confirmation tests drive.
///
/// # Errors
///
/// As [`run_cleanup`].
fn cleanup_with<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    opts: &CleanupOptions,
    env: &mut CleanupEnv<'_>,
    out: &mut W,
) -> anyhow::Result<()> {
    // The terminal status is the last configured column (empty/blank entries
    // mean none), matched case-insensitively — the npm CLI's rule.
    let Some(terminal) = terminal_status(&cfg.statuses) else {
        writeln!(out, "No terminal status configured for cleanup.")
            .context("printing the no-terminal-status notice")?;
        return Ok(());
    };

    let entries = store.scan_tasks()?;
    let terminal_tasks: Vec<&TaskEntry> = entries
        .iter()
        .filter(|e| e.doc.frontmatter.status.eq_ignore_ascii_case(terminal))
        .collect();
    if terminal_tasks.is_empty() {
        writeln!(out, "No {terminal} tasks found to clean up.")
            .context("printing the no-terminal-tasks notice")?;
        return Ok(());
    }
    writeln!(
        out,
        "Found {} tasks marked as {terminal}.",
        terminal_tasks.len()
    )
    .context("printing the terminal-status count")?;

    let cutoff = env
        .now
        .checked_sub_days(Days::new(u64::from(opts.older_than_days)))
        .context("computing the cleanup cutoff")?;
    let aged: Vec<&&TaskEntry> = terminal_tasks
        .iter()
        .filter(|e| is_older_than(e, cutoff))
        .collect();
    if aged.is_empty() {
        writeln!(
            out,
            "No tasks found that are older than {} days.",
            opts.older_than_days
        )
        .context("printing the no-aged-tasks notice")?;
        return Ok(());
    }

    writeln!(
        out,
        "Found {} tasks older than {} days:",
        aged.len(),
        opts.older_than_days
    )
    .context("printing the aged-task count")?;
    for entry in &aged {
        let fm = &entry.doc.frontmatter;
        let date = fm.updated_date.as_deref().unwrap_or(&fm.created_date);
        writeln!(out, "  - {}: {} ({date})", fm.id, fm.title)
            .context("printing an aged-task row")?;
    }

    if opts.dry_run {
        writeln!(out, "Dry run: no files moved.").context("printing the dry-run notice")?;
        return Ok(());
    }

    if !confirm_move(aged.len(), env.input, out)? {
        writeln!(out, "Cleanup cancelled.").context("printing the cancellation notice")?;
        return Ok(());
    }

    let completed = store.completed_dir();
    std::fs::create_dir_all(&completed)
        .with_context(|| format!("creating {}", completed.display()))?;
    // Preflight every destination before the first rename: a collision found
    // only when its turn comes would leave the earlier files already moved —
    // a partial cleanup. [`move_to_completed`] re-checks each destination as
    // a guard against concurrent filesystem changes in between.
    for entry in &aged {
        let to = destination_for(&entry.path, &completed)?;
        ensure_destination_free(&entry.path, &to)?;
    }
    for entry in &aged {
        move_to_completed(&entry.path, &completed)?;
    }
    writeln!(out, "Moved {} tasks to completed folder.", aged.len())
        .context("printing the moved summary")?;
    Ok(())
}

/// The terminal status: the last configured column, when it carries a
/// non-blank name.
fn terminal_status(statuses: &[String]) -> Option<&str> {
    statuses
        .last()
        .map(String::as_str)
        .filter(|s| !s.trim().is_empty())
}

/// Ask `Move N tasks to completed folder? [y/N]` and read one answer line.
/// `y`/`yes` (case-insensitive) proceeds; empty input — including EOF on a
/// closed stdin — and anything else cancels. No is the default, matching the
/// backlog CLI's confirm prompt.
fn confirm_move<W: Write>(
    count: usize,
    input: &mut dyn std::io::BufRead,
    out: &mut W,
) -> anyhow::Result<bool> {
    write!(out, "Move {count} tasks to completed folder? [y/N] ")
        .context("printing the confirmation prompt")?;
    out.flush().context("flushing the confirmation prompt")?;
    let mut answer = String::new();
    input
        .read_line(&mut answer)
        .context("reading the confirmation answer")?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
}

/// A task's age reads `updated_date` with `created_date` as fallback — the
/// npm CLI's rule. A date that is absent or unparseable excludes the task
/// (there, an invalid Date compares false; here, `None` does).
fn is_older_than(entry: &TaskEntry, cutoff: DateTime<Utc>) -> bool {
    let fm = &entry.doc.frontmatter;
    let raw = fm
        .updated_date
        .as_deref()
        .unwrap_or(fm.created_date.as_str());
    parse_frontmatter_date(raw).is_some_and(|dt| dt < cutoff)
}

/// Parse a `'YYYY-MM-DD HH:MM'` frontmatter date, with or without the
/// seconds part the 24 oldest files carry, as UTC.
fn parse_frontmatter_date(raw: &str) -> Option<DateTime<Utc>> {
    ["%Y-%m-%d %H:%M:%S", "%Y-%m-%d %H:%M"]
        .iter()
        .find_map(|fmt| NaiveDateTime::parse_from_str(raw.trim(), fmt).ok())
        .map(|dt| Utc.from_utc_datetime(&dt))
}

/// The `completed/` path one task file would move to, under the same name.
///
/// # Errors
///
/// The task file name is not valid UTF-8.
fn destination_for(from: &Path, completed: &Path) -> anyhow::Result<std::path::PathBuf> {
    let name = from
        .file_name()
        .and_then(|n| n.to_str())
        .context("task file name is not valid UTF-8")?;
    Ok(completed.join(name))
}

/// Refuse a move whose destination already holds a file (the error names
/// both paths — the tree holds real id collisions between directories, and a
/// silent overwrite would destroy one of them).
///
/// # Errors
///
/// The destination exists, or its existence cannot be determined (the error
/// names the path).
fn ensure_destination_free(from: &Path, to: &Path) -> anyhow::Result<()> {
    if to
        .try_exists()
        .with_context(|| format!("checking {}", to.display()))?
    {
        anyhow::bail!(
            "{} already exists; refusing to overwrite it with {}",
            to.display(),
            from.display()
        );
    }
    Ok(())
}

/// Move one task file into `completed/` under the same name. The destination
/// is re-checked here even though the preflight pass already cleared it — a
/// guard against a file appearing in `completed/` between the two.
///
/// # Errors
///
/// As [`destination_for`] and [`ensure_destination_free`], or the rename
/// itself fails (the error names the path).
fn move_to_completed(from: &Path, completed: &Path) -> anyhow::Result<()> {
    let to = destination_for(from, completed)?;
    ensure_destination_free(from, &to)?;
    std::fs::rename(from, &to)
        .with_context(|| format!("moving {} to {}", from.display(), to.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_with(tasks: &[(&str, &str)]) -> (tempfile::TempDir, Store, BacklogConfig) {
        let dir = tempfile::tempdir().expect("tempdir");
        let tasks_dir = dir.path().join(".backlog").join("tasks");
        std::fs::create_dir_all(&tasks_dir).expect("tasks dir");
        for &(name, fm) in tasks {
            std::fs::write(tasks_dir.join(name), fm).expect("seed task");
        }
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        (dir, store, BacklogConfig::default())
    }

    /// Frontmatter with the given dates; `None` omits the key entirely
    /// (4 pre-history files carry no `updated_date`).
    fn fm(id: &str, status: &str, created: Option<&str>, updated: Option<&str>) -> String {
        use std::fmt::Write as _;
        let mut src = format!(
            "---\nid: {id}\ntitle: 'task {id}'\nstatus: {status}\nassignee: []\ncreated_date: '{}'\nlabels: []\ndependencies: []\n",
            created.unwrap_or("2026-01-01 00:00")
        );
        if let Some(updated) = updated {
            let _ = writeln!(src, "updated_date: '{updated}'");
        }
        src.push_str("---\n");
        src
    }

    /// Fixed clock: 2026-09-06 12:00 UTC.
    fn now() -> DateTime<Utc> {
        Utc.from_utc_datetime(
            &NaiveDateTime::parse_from_str("2026-09-06 12:00", "%Y-%m-%d %H:%M").expect("now"),
        )
    }

    fn run(
        store: &Store,
        cfg: &BacklogConfig,
        older_than: u32,
        dry_run: bool,
        answer: &str,
    ) -> String {
        let mut out = Vec::new();
        let mut input = answer.as_bytes();
        let mut env = CleanupEnv {
            now: now(),
            input: &mut input,
        };
        cleanup_with(
            store,
            cfg,
            &CleanupOptions {
                older_than_days: older_than,
                dry_run,
            },
            &mut env,
            &mut out,
        )
        .expect("cleanup");
        String::from_utf8(out).expect("utf8")
    }

    /// A Done task updated 31 days before the fixed clock moves; one updated
    /// today stays. 30 days = the default `--older-than` in the CLI.
    #[test]
    fn moves_terminal_tasks_older_than_the_cutoff() {
        let (dir, store, cfg) = scratch_with(&[
            (
                "task-0001 - old.md",
                &fm(
                    "TASK-0001",
                    "Done",
                    Some("2026-01-01 00:00"),
                    Some("2026-08-06 11:59"),
                ),
            ),
            (
                "task-0002 - fresh.md",
                &fm(
                    "TASK-0002",
                    "Done",
                    Some("2026-01-01 00:00"),
                    Some("2026-09-06 12:00"),
                ),
            ),
        ]);
        let text = run(&store, &cfg, 30, false, "y\n");
        assert!(text.contains("Found 2 tasks marked as Done."));
        assert!(text.contains("Found 1 tasks older than 30 days:"));
        assert!(text.contains("- TASK-0001: task TASK-0001 (2026-08-06 11:59)"));
        assert!(text.contains("Move 1 tasks to completed folder? [y/N]"));
        assert!(text.contains("Moved 1 tasks to completed folder."));
        let root = dir.path().join(".backlog");
        assert!(!root.join("tasks/task-0001 - old.md").exists());
        assert!(root.join("completed/task-0001 - old.md").exists());
        assert!(root.join("tasks/task-0002 - fresh.md").exists());
    }

    /// `updated_date` wins over an older `created_date`, and a task whose
    /// date is unparseable is excluded, never moved on a technicality.
    #[test]
    fn age_reads_updated_date_first_and_skips_unparseable_dates() {
        let (_dir, store, cfg) = scratch_with(&[
            // created long ago, updated yesterday: not old enough.
            (
                "task-0001 - refreshed.md",
                &fm(
                    "TASK-0001",
                    "Done",
                    Some("2020-01-01 00:00"),
                    Some("2026-09-05 12:00"),
                ),
            ),
            // no updated_date: falls back to created_date, 2026-01-01.
            (
                "task-0002 - stale.md",
                &fm("TASK-0002", "Done", Some("2026-01-01 00:00"), None),
            ),
            // seconds form parses too.
            (
                "task-0003 - seconds.md",
                &fm(
                    "TASK-0003",
                    "Done",
                    Some("2026-01-01 00:00"),
                    Some("2026-01-02 03:04:05"),
                ),
            ),
            // unparseable: excluded.
            (
                "task-0004 - odd.md",
                &fm("TASK-0004", "Done", Some("back then"), None),
            ),
        ]);
        let text = run(&store, &cfg, 30, false, "y\n");
        assert!(text.contains("TASK-0002"));
        assert!(text.contains("TASK-0003"));
        assert!(!text.contains("TASK-0001"));
        assert!(!text.contains("TASK-0004"));
    }

    /// Terminal status is the *last* configured column, case-insensitive —
    /// a "done" task matches a `Done` column.
    #[test]
    fn terminal_status_is_the_last_column_matched_case_insensitively() {
        let (_dir, store, _cfg) = scratch_with(&[(
            "task-0001 - done.md",
            &fm("TASK-0001", "done", Some("2026-01-01 00:00"), None),
        )]);
        let cfg = BacklogConfig {
            statuses: vec![
                "To Do".to_string(),
                "In Progress".to_string(),
                "Done".to_string(),
            ],
            ..BacklogConfig::default()
        };
        let text = run(&store, &cfg, 30, false, "y\n");
        assert!(text.contains("Moved 1 tasks to completed folder."));
    }

    /// Non-terminal tasks are never touched, whatever their age.
    #[test]
    fn non_terminal_statuses_are_never_moved() {
        let (dir, store, cfg) = scratch_with(&[(
            "task-0001 - triage.md",
            &fm("TASK-0001", "Triage", Some("2020-01-01 00:00"), None),
        )]);
        let text = run(&store, &cfg, 30, false, "");
        assert!(text.contains("No Done tasks found to clean up."));
        assert!(dir
            .path()
            .join(".backlog/tasks/task-0001 - triage.md")
            .exists());
    }

    /// Dry run names the same candidates and moves nothing.
    #[test]
    fn dry_run_moves_nothing() {
        let (dir, store, cfg) = scratch_with(&[(
            "task-0001 - old.md",
            &fm("TASK-0001", "Done", Some("2026-01-01 00:00"), None),
        )]);
        // No answer is needed: dry run never asks.
        let text = run(&store, &cfg, 30, true, "");
        assert!(text.contains("TASK-0001"));
        assert!(text.contains("Dry run: no files moved."));
        assert!(!text.contains("[y/N]"));
        assert!(dir
            .path()
            .join(".backlog/tasks/task-0001 - old.md")
            .exists());
        assert!(!dir.path().join(".backlog/completed").exists());
    }

    /// A same-name file already in `completed/` aborts the command naming
    /// both paths instead of overwriting either.
    #[test]
    fn same_name_collision_is_an_error_naming_both_paths() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join(".backlog");
        std::fs::create_dir_all(root.join("tasks")).expect("tasks dir");
        std::fs::create_dir_all(root.join("completed")).expect("completed dir");
        std::fs::write(
            root.join("tasks/task-0001 - old.md"),
            fm("TASK-0001", "Done", Some("2026-01-01 00:00"), None),
        )
        .expect("seed task");
        std::fs::write(root.join("completed/task-0001 - old.md"), "occupied").expect("seed target");

        let store = Store::open(&root).expect("open");
        let cfg = BacklogConfig::default();
        let mut out = Vec::new();
        let mut input: &[u8] = b"y\n";
        let mut env = CleanupEnv {
            now: now(),
            input: &mut input,
        };
        let err = cleanup_with(
            &store,
            &cfg,
            &CleanupOptions {
                older_than_days: 30,
                dry_run: false,
            },
            &mut env,
            &mut out,
        )
        .expect_err("must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("refusing to overwrite"),
            "error must state the refusal, got: {rendered}"
        );
        assert_eq!(
            std::fs::read_to_string(root.join("completed/task-0001 - old.md")).expect("target"),
            "occupied",
            "the existing completed file must be untouched"
        );
        assert!(
            root.join("tasks/task-0001 - old.md").exists(),
            "the source task must still be in tasks/"
        );
    }

    /// The collision preflight is all-or-nothing: when any destination is
    /// taken, no file moves at all — not even the ones before it in order.
    #[test]
    fn collision_preflight_moves_nothing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join(".backlog");
        std::fs::create_dir_all(root.join("tasks")).expect("tasks dir");
        std::fs::create_dir_all(root.join("completed")).expect("completed dir");
        // TASK-0001's destination is free; TASK-0002's is occupied.
        std::fs::write(
            root.join("tasks/task-0001 - free.md"),
            fm("TASK-0001", "Done", Some("2026-01-01 00:00"), None),
        )
        .expect("seed 1");
        std::fs::write(
            root.join("tasks/task-0002 - blocked.md"),
            fm("TASK-0002", "Done", Some("2026-01-01 00:00"), None),
        )
        .expect("seed 2");
        std::fs::write(root.join("completed/task-0002 - blocked.md"), "occupied")
            .expect("seed target");

        let store = Store::open(&root).expect("open");
        let cfg = BacklogConfig::default();
        let mut out = Vec::new();
        let mut input: &[u8] = b"y\n";
        let mut env = CleanupEnv {
            now: now(),
            input: &mut input,
        };
        let err = cleanup_with(
            &store,
            &cfg,
            &CleanupOptions {
                older_than_days: 30,
                dry_run: false,
            },
            &mut env,
            &mut out,
        )
        .expect_err("must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("refusing to overwrite"),
            "error must state the refusal, got: {rendered}"
        );
        assert!(
            root.join("tasks/task-0001 - free.md").exists(),
            "a later collision must keep the earlier task in tasks/ too"
        );
        assert!(
            root.join("tasks/task-0002 - blocked.md").exists(),
            "the colliding task must stay in tasks/"
        );
        assert!(
            !root.join("completed/task-0001 - free.md").exists(),
            "nothing may land in completed/ when the preflight aborts"
        );
    }

    /// An empty statuses list leaves nothing to clean up and says so.
    #[test]
    fn no_terminal_status_is_a_clean_no_op() {
        let (_dir, store, _cfg) = scratch_with(&[(
            "task-0001 - done.md",
            &fm("TASK-0001", "Done", Some("2026-01-01 00:00"), None),
        )]);
        let cfg = BacklogConfig {
            statuses: Vec::new(),
            ..BacklogConfig::default()
        };
        let text = run(&store, &cfg, 30, false, "");
        assert!(text.contains("No terminal status configured for cleanup."));
    }

    /// The prompt shows the count; answering n cancels without moving.
    #[test]
    fn declined_confirmation_cancels_without_moving() {
        let (dir, store, cfg) = scratch_with(&[(
            "task-0001 - old.md",
            &fm("TASK-0001", "Done", Some("2026-01-01 00:00"), None),
        )]);
        let text = run(&store, &cfg, 30, false, "n\n");
        assert!(text.contains("Move 1 tasks to completed folder? [y/N]"));
        assert!(text.contains("Cleanup cancelled."));
        assert!(!text.contains("Moved"));
        assert!(dir
            .path()
            .join(".backlog/tasks/task-0001 - old.md")
            .exists());
    }

    /// EOF / empty input defaults to No — a piped stdin with no answer never
    /// moves files, matching the backlog CLI's default-false confirm.
    #[test]
    fn empty_answer_defaults_to_no() {
        let (dir, store, cfg) = scratch_with(&[(
            "task-0001 - old.md",
            &fm("TASK-0001", "Done", Some("2026-01-01 00:00"), None),
        )]);
        let text = run(&store, &cfg, 30, false, "");
        assert!(text.contains("Cleanup cancelled."));
        assert!(dir
            .path()
            .join(".backlog/tasks/task-0001 - old.md")
            .exists());
    }

    /// `YES` in any casing is accepted; anything that is not y/yes is not.
    #[test]
    fn yes_is_case_insensitive_and_strict() {
        let (_dir, store, cfg) = scratch_with(&[(
            "task-0001 - old.md",
            &fm("TASK-0001", "Done", Some("2026-01-01 00:00"), None),
        )]);
        let text = run(&store, &cfg, 30, false, "YES\n");
        assert!(text.contains("Moved 1 tasks to completed folder."));

        let (dir, store, cfg) = scratch_with(&[(
            "task-0002 - old.md",
            &fm("TASK-0002", "Done", Some("2026-01-01 00:00"), None),
        )]);
        let text = run(&store, &cfg, 30, false, "yeah\n");
        assert!(text.contains("Cleanup cancelled."));
        assert!(dir
            .path()
            .join(".backlog/tasks/task-0002 - old.md")
            .exists());
    }
}
