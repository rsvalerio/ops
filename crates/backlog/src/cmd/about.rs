//! `ops about backlog`: a read-only overview of the whole backlog tree.
//!
//! Per-status totals across `tasks/`, `completed/`, and the archives (which
//! no listing ever surfaces), plus health metrics over the live set.
//! Archiving is a file-location concept — the status inside the file is
//! unchanged by a move — so the matrix counts by frontmatter status and
//! splits by directory. "Done in the last N days" is an approximation: it
//! reads `updated_date` with `created_date` as fallback (cleanup's rule),
//! and `updated_date` bumps on any edit, not only completion.

use std::collections::HashMap;
use std::io::Write;

use anyhow::Context as _;
use chrono::{DateTime, Days, Utc};

use crate::config::BacklogConfig;
use crate::render::{priority_rank, readiness_of};
use crate::store::{LocatedTask, Store, TaskLocation};

use super::cleanup::{parse_frontmatter_date, terminal_status};

/// One row of the status matrix: counts by location for one status.
struct StatusRow {
    status: String,
    live: usize,
    completed: usize,
    archived: usize,
}

impl StatusRow {
    const fn total(&self) -> usize {
        self.live
            .saturating_add(self.completed)
            .saturating_add(self.archived)
    }
}

/// The oldest live, non-terminal task by `created_date`.
struct OldestOpen {
    id: String,
    title: String,
    /// `YYYY-MM-DD` — the first 10 chars of the raw frontmatter value.
    created_day: String,
    days_old: i64,
}

/// Everything the page renders, computed in one pass so the render code has
/// no data dependencies and tests can assert on plain numbers.
struct BacklogOverview {
    total: usize,
    live: usize,
    completed: usize,
    archived: usize,
    /// The configured terminal status; `None` when the config lists none.
    terminal: Option<String>,
    /// Tasks whose status case-insensitively equals the terminal status,
    /// across all directories.
    complete: usize,
    rows: Vec<StatusRow>,
    ready: usize,
    blocked: usize,
    unassigned: usize,
    oldest_open: Option<OldestOpen>,
    /// `critical`, `high`, `medium`, `low`, `unset` counts over live tasks.
    priority: Vec<(&'static str, usize)>,
    created_7d: usize,
    created_30d: usize,
    done_7d: usize,
    done_30d: usize,
    /// Live labels, count descending then name ascending, at most 5.
    top_labels: Vec<(String, usize)>,
}

/// Run `about backlog` against the host clock.
///
/// # Errors
///
/// A task file anywhere in the tree does not parse (the error names the
/// file), or writing `out` failed.
pub fn run_about_backlog<W: Write>(
    store: &Store,
    cfg: &BacklogConfig,
    out: &mut W,
) -> anyhow::Result<()> {
    let tasks = store.scan_all_tasks()?;
    // The workspace chrono carries no `clock` feature; the wall clock is read
    // as a SystemTime and converted, exactly like `run_cleanup`.
    let now: DateTime<Utc> = std::time::SystemTime::now().into();
    about_with(&tasks, cfg, now, out)
}

/// [`run_about_backlog`] against an explicit clock; the seam the metric
/// tests drive.
///
/// # Errors
///
/// Writing `out` failed.
fn about_with<W: Write>(
    tasks: &[LocatedTask],
    cfg: &BacklogConfig,
    now: DateTime<Utc>,
    out: &mut W,
) -> anyhow::Result<()> {
    let data = overview(tasks, cfg, now);
    if data.total == 0 {
        writeln!(out, "No backlog tasks found.").context("printing the empty-backlog notice")?;
        return Ok(());
    }
    render(&data, out)
}

/// Aggregate every metric over the scanned tree.
fn overview(tasks: &[LocatedTask], cfg: &BacklogConfig, now: DateTime<Utc>) -> BacklogOverview {
    let terminal = terminal_status(&cfg.statuses).map(str::to_string);
    let is_terminal = |status: &str| {
        terminal
            .as_deref()
            .is_some_and(|t| status.eq_ignore_ascii_case(t))
    };

    let live: Vec<&LocatedTask> = tasks.iter().filter(|t| t.location.is_live()).collect();
    let live_open: Vec<&LocatedTask> = live
        .iter()
        .copied()
        .filter(|t| !is_terminal(&t.doc.frontmatter.status))
        .collect();
    let (ready, blocked) = readiness_counts(&live_open, &live);
    let (created_7d, created_30d, done_7d, done_30d) = activity_counts(tasks, &is_terminal, now);

    BacklogOverview {
        total: tasks.len(),
        live: live.len(),
        completed: tasks
            .iter()
            .filter(|t| matches!(t.location, TaskLocation::Completed))
            .count(),
        archived: tasks
            .iter()
            .filter(|t| {
                matches!(
                    t.location,
                    TaskLocation::ArchiveTasks | TaskLocation::ArchiveCompleted
                )
            })
            .count(),
        complete: tasks
            .iter()
            .filter(|t| is_terminal(&t.doc.frontmatter.status))
            .count(),
        terminal,
        rows: status_rows(tasks, cfg),
        ready,
        blocked,
        unassigned: live
            .iter()
            .filter(|t| t.doc.frontmatter.assignees.is_empty())
            .count(),
        oldest_open: oldest_open(&live_open, now),
        priority: priority_counts(&live),
        created_7d,
        created_30d,
        done_7d,
        done_30d,
        top_labels: top_label_counts(&live),
    }
}

/// Status rows: configured statuses in config order, then unknown statuses
/// in first-seen order (the rule `task list` groups by), empty groups
/// skipped.
fn status_rows(tasks: &[LocatedTask], cfg: &BacklogConfig) -> Vec<StatusRow> {
    let mut order: Vec<String> = cfg.statuses.clone();
    for task in tasks {
        let status = &task.doc.frontmatter.status;
        if !order.contains(status) {
            order.push(status.clone());
        }
    }
    order
        .into_iter()
        .filter_map(|status| {
            let mut row = StatusRow {
                status,
                live: 0,
                completed: 0,
                archived: 0,
            };
            let mut seen = false;
            for task in tasks {
                if task.doc.frontmatter.status == row.status {
                    seen = true;
                    match task.location {
                        TaskLocation::Tasks => row.live = row.live.saturating_add(1),
                        TaskLocation::Completed => {
                            row.completed = row.completed.saturating_add(1);
                        }
                        TaskLocation::ArchiveTasks | TaskLocation::ArchiveCompleted => {
                            row.archived = row.archived.saturating_add(1);
                        }
                    }
                }
            }
            seen.then_some(row)
        })
        .collect()
}

/// Ready and blocked counts over the live non-terminal tasks. Readiness is
/// answered against live tasks only: live ids are unique, while `completed/`
/// and `archive/tasks/` hold real id collisions whose statuses must not flip
/// a dependency between ready and blocked.
fn readiness_counts(live_open: &[&LocatedTask], live: &[&LocatedTask]) -> (usize, usize) {
    // Ids resolve case-insensitively everywhere else in the store
    // (`Store::find` lowercases both sides); the lookup here must too, or a
    // dependency written `task-0003` would miss live `TASK-0003`, count as
    // missing — non-blocking — and flip a blocked task to ready.
    let live_statuses: HashMap<String, &str> = live
        .iter()
        .map(|t| {
            (
                t.doc.frontmatter.id.to_ascii_lowercase(),
                t.doc.frontmatter.status.as_str(),
            )
        })
        .collect();
    let lookup = |id: &str| {
        live_statuses
            .get(&id.to_ascii_lowercase())
            .map(|s| (*s).to_string())
    };
    let mut ready: usize = 0;
    let mut blocked: usize = 0;
    for task in live_open {
        // Unresolved dependencies are missing, not blocking (readiness_of's
        // rule, shared with `task list`).
        if readiness_of(&task.doc, &lookup).blocking.is_empty() {
            ready = ready.saturating_add(1);
        } else {
            blocked = blocked.saturating_add(1);
        }
    }
    (ready, blocked)
}

/// The oldest live non-terminal task by `created_date`; tasks with absent or
/// unparseable dates are excluded.
fn oldest_open(live_open: &[&LocatedTask], now: DateTime<Utc>) -> Option<OldestOpen> {
    live_open
        .iter()
        .filter_map(|t| parse_frontmatter_date(&t.doc.frontmatter.created_date).map(|dt| (*t, dt)))
        .min_by_key(|(_, dt)| *dt)
        .map(|(t, dt)| OldestOpen {
            id: t.doc.frontmatter.id.clone(),
            title: t.doc.frontmatter.title.clone(),
            created_day: t.doc.frontmatter.created_date.chars().take(10).collect(),
            days_old: now.signed_duration_since(dt).num_days(),
        })
}

/// `critical`/`high`/`medium`/`low`/`unset` counts over live tasks; an
/// unrecognized priority lands in `unset`, exactly as `priority_rank`
/// ranks it in listings.
fn priority_counts(live: &[&LocatedTask]) -> Vec<(&'static str, usize)> {
    let count = |name: &str| {
        live.iter()
            .filter(|t| {
                t.doc
                    .frontmatter
                    .priority
                    .as_deref()
                    .is_some_and(|p| p.eq_ignore_ascii_case(name))
            })
            .count()
    };
    let unset = live
        .iter()
        .filter(|t| priority_rank(t.doc.frontmatter.priority.as_deref()) == 0)
        .count();
    vec![
        ("critical", count("critical")),
        ("high", count("high")),
        ("medium", count("medium")),
        ("low", count("low")),
        ("unset", unset),
    ]
}

/// Created / done counts for the 7- and 30-day windows. Created counts every
/// directory by `created_date`; done counts terminal-status tasks by
/// `updated_date` with `created_date` fallback (cleanup's rule).
fn activity_counts(
    tasks: &[LocatedTask],
    is_terminal: &dyn Fn(&str) -> bool,
    now: DateTime<Utc>,
) -> (usize, usize, usize, usize) {
    // A clock so far in the future that the subtraction overflows has no
    // meaningful "last N days"; counting everything as outside the window
    // (cutting off at `now`) is the least surprising degradation.
    let cutoff = |days: u64| now.checked_sub_days(Days::new(days)).unwrap_or(now);
    let in_window =
        |raw: &str, from: DateTime<Utc>| parse_frontmatter_date(raw).is_some_and(|dt| dt >= from);
    let mut created_7d: usize = 0;
    let mut created_30d: usize = 0;
    let mut done_7d: usize = 0;
    let mut done_30d: usize = 0;
    for task in tasks {
        let fm = &task.doc.frontmatter;
        if in_window(&fm.created_date, cutoff(7)) {
            created_7d = created_7d.saturating_add(1);
        }
        if in_window(&fm.created_date, cutoff(30)) {
            created_30d = created_30d.saturating_add(1);
        }
        if is_terminal(&fm.status) {
            let last = fm
                .updated_date
                .as_deref()
                .unwrap_or(fm.created_date.as_str());
            if in_window(last, cutoff(7)) {
                done_7d = done_7d.saturating_add(1);
            }
            if in_window(last, cutoff(30)) {
                done_30d = done_30d.saturating_add(1);
            }
        }
    }
    (created_7d, created_30d, done_7d, done_30d)
}

/// Live label counts, count descending then name ascending, at most 5.
fn top_label_counts(live: &[&LocatedTask]) -> Vec<(String, usize)> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for task in live {
        for label in &task.doc.frontmatter.labels {
            let slot = counts.entry(label.as_str()).or_default();
            *slot = slot.saturating_add(1);
        }
    }
    let mut top: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(label, count)| (label.to_string(), count))
        .collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    top.truncate(5);
    top
}

/// usize → f64 without a silent `as` cast (workspace policy): counts
/// saturate at `u32::MAX`, far beyond any real backlog.
fn count_as_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Render the overview page: headline, status matrix, live-work, priority,
/// activity, and label sections.
///
/// # Errors
///
/// Writing `out` failed.
fn render<W: Write>(data: &BacklogOverview, out: &mut W) -> anyhow::Result<()> {
    let completion = match &data.terminal {
        Some(_) => format!(
            "{} complete ({:.1}%)",
            data.complete,
            100.0 * count_as_f64(data.complete) / count_as_f64(data.total)
        ),
        None => "completion n/a (no terminal status)".to_string(),
    };
    writeln!(out, "Backlog overview — {} tasks, {completion}", data.total)
        .context("printing the overview headline")?;
    writeln!(out).context("printing the overview headline")?;
    render_matrix(data, out)?;

    if data.live > 0 {
        writeln!(out).context("printing the live-work header")?;
        writeln!(out, "Live work").context("printing the live-work header")?;
        writeln!(
            out,
            "  Ready to start: {}   Blocked: {}   Unassigned: {}",
            data.ready, data.blocked, data.unassigned
        )
        .context("printing the live-work counts")?;
        if let Some(oldest) = &data.oldest_open {
            writeln!(
                out,
                "  Oldest open: {} - {} (created {}, {} days old)",
                oldest.id, oldest.title, oldest.created_day, oldest.days_old
            )
            .context("printing the oldest-open line")?;
        }

        let priority = data
            .priority
            .iter()
            .map(|(name, count)| format!("{name} {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(out).context("printing the priority line")?;
        writeln!(out, "Priority (live): {priority}").context("printing the priority line")?;
    }

    writeln!(out).context("printing the activity header")?;
    writeln!(out, "Activity").context("printing the activity header")?;
    writeln!(
        out,
        "  Created: {} in the last 7 days, {} in the last 30 days",
        data.created_7d, data.created_30d
    )
    .context("printing the created-activity line")?;
    writeln!(
        out,
        "  Done:    {} in the last 7 days, {} in the last 30 days",
        data.done_7d, data.done_30d
    )
    .context("printing the done-activity line")?;

    if !data.top_labels.is_empty() {
        let labels = data
            .top_labels
            .iter()
            .map(|(label, count)| format!("{label} {count}"))
            .collect::<Vec<_>>()
            .join(", ");
        writeln!(out).context("printing the labels line")?;
        writeln!(out, "Top labels (live): {labels}").context("printing the labels line")?;
    }

    Ok(())
}

/// The status matrix: one row per status, columns Live / Completed /
/// Archived / Total, and a grand Total row. Column widths grow to fit the
/// widest cell so large counts cannot misalign the table.
///
/// # Errors
///
/// Writing `out` failed.
fn render_matrix<W: Write>(data: &BacklogOverview, out: &mut W) -> anyhow::Result<()> {
    let mut table: Vec<[String; 5]> = vec![[
        "Status".to_string(),
        "Live".to_string(),
        "Completed".to_string(),
        "Archived".to_string(),
        "Total".to_string(),
    ]];
    for row in &data.rows {
        table.push([
            row.status.clone(),
            row.live.to_string(),
            row.completed.to_string(),
            row.archived.to_string(),
            row.total().to_string(),
        ]);
    }
    table.push([
        "Total".to_string(),
        data.live.to_string(),
        data.completed.to_string(),
        data.archived.to_string(),
        data.total.to_string(),
    ]);
    let mut widths = [0usize; 5];
    for line in &table {
        for (w, cell) in widths.iter_mut().zip(line.iter()) {
            *w = (*w).max(cell.chars().count());
        }
    }
    let [sw, lw, cw, aw, tw] = widths;
    for line in &table {
        let [status, live, completed, archived, total] = line;
        writeln!(
            out,
            "{status:<sw$}  {live:>lw$}  {completed:>cw$}  {archived:>aw$}  {total:>tw$}",
        )
        .context("printing a status-matrix row")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One seeded task. List-valued fields are `&[]`/`None` for the common
    /// minimal case; the seeder renders them as block lists when non-empty.
    struct Seed<'a> {
        dir: &'a str,
        id: &'a str,
        status: &'a str,
        title: &'a str,
        created: &'a str,
        updated: Option<&'a str>,
        priority: Option<&'a str>,
        labels: &'a [&'a str],
        deps: &'a [&'a str],
        assignees: &'a [&'a str],
    }

    impl<'a> Seed<'a> {
        fn new(dir: &'a str, id: &'a str, status: &'a str) -> Self {
            Self {
                dir,
                id,
                status,
                title: "t",
                created: "2026-01-01 00:00",
                updated: None,
                priority: None,
                labels: &[],
                deps: &[],
                assignees: &[],
            }
        }
    }

    fn scratch(seeds: &[Seed<'_>]) -> (tempfile::TempDir, Store, BacklogConfig) {
        use std::fmt::Write as _;
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().join(".backlog");
        std::fs::create_dir_all(root.join("tasks")).expect("tasks dir");
        for seed in seeds {
            let target = root.join(seed.dir);
            std::fs::create_dir_all(&target).expect("seed dir");
            let digits: String = seed
                .id
                .trim_start_matches("TASK-")
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            let list = |items: &[&str]| {
                if items.is_empty() {
                    "[]".to_string()
                } else {
                    format!(
                        "\n{}",
                        items
                            .iter()
                            .map(|i| format!("  - {i}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    )
                }
            };
            let mut src = format!(
                "---\nid: {}\ntitle: '{}'\nstatus: {}\nassignee: {}\ncreated_date: '{}'\nlabels: {}\ndependencies: {}\n",
                seed.id,
                seed.title,
                seed.status,
                list(seed.assignees),
                seed.created,
                list(seed.labels),
                list(seed.deps),
            );
            if let Some(updated) = seed.updated {
                let _ = writeln!(src, "updated_date: '{updated}'");
            }
            if let Some(priority) = seed.priority {
                let _ = writeln!(src, "priority: {priority}");
            }
            src.push_str("---\n");
            std::fs::write(target.join(format!("task-{digits:0>4} - x.md")), src)
                .expect("seed task");
        }
        let store = Store::open(&root).expect("open");
        (dir, store, BacklogConfig::default())
    }

    /// Fixed clock: 2026-09-06 12:00 UTC — the cleanup tests' clock.
    fn now() -> DateTime<Utc> {
        chrono::TimeZone::from_utc_datetime(
            &Utc,
            &chrono::NaiveDateTime::parse_from_str("2026-09-06 12:00", "%Y-%m-%d %H:%M")
                .expect("now"),
        )
    }

    fn scan(store: &Store) -> Vec<LocatedTask> {
        store.scan_all_tasks().expect("scan all")
    }

    fn row<'a>(data: &'a BacklogOverview, status: &str) -> &'a StatusRow {
        data.rows
            .iter()
            .find(|r| r.status == status)
            .unwrap_or_else(|| panic!("no row for {status}"))
    }

    /// The matrix counts every directory, keeps config status order with
    /// unknown statuses after it, and skips empty groups.
    #[test]
    fn matrix_counts_by_status_across_all_dirs() {
        let (_dir, store, cfg) = scratch(&[
            Seed::new("tasks", "TASK-0001", "Triage"),
            Seed::new("tasks", "TASK-0002", "To Do"),
            Seed::new("tasks", "TASK-0003", "Done"),
            Seed::new("tasks", "TASK-0008", "Blocked"),
            Seed::new("completed", "TASK-0004", "Done"),
            Seed::new("archive/tasks", "TASK-0005", "Done"),
            Seed::new("archive/tasks", "TASK-0006", "To Do"),
            Seed::new("archive/completed", "TASK-0007", "Done"),
        ]);
        let tasks = scan(&store);
        let data = overview(&tasks, &cfg, now());

        assert_eq!(data.total, 8);
        assert_eq!(data.live, 4);
        assert_eq!(data.completed, 1);
        assert_eq!(data.archived, 3);
        assert_eq!(data.complete, 4, "three Done + archive copy");

        let statuses: Vec<&str> = data.rows.iter().map(|r| r.status.as_str()).collect();
        assert_eq!(
            statuses,
            vec!["Triage", "To Do", "Done", "Blocked"],
            "config order first, unknown first-seen after, In Progress skipped"
        );

        let done = row(&data, "Done");
        assert_eq!((done.live, done.completed, done.archived), (1, 1, 2));
        let to_do = row(&data, "To Do");
        assert_eq!((to_do.live, to_do.completed, to_do.archived), (1, 0, 1));
        let blocked = row(&data, "Blocked");
        assert_eq!(
            (blocked.live, blocked.completed, blocked.archived),
            (1, 0, 0)
        );
    }

    /// An empty tree (only the required, empty `tasks/`) says so and nothing
    /// else.
    #[test]
    fn empty_backlog_prints_the_notice() {
        let (_dir, store, cfg) = scratch(&[]);
        let mut out = Vec::new();
        about_with(&scan(&store), &cfg, now(), &mut out).expect("about");
        let text = String::from_utf8(out).expect("utf8");
        assert_eq!(text, "No backlog tasks found.\n");
    }

    /// Completion is matched case-insensitively against the last configured
    /// column — a `done` task counts — and is `n/a` without a terminal
    /// status.
    #[test]
    fn completion_rate_is_case_insensitive_and_na_without_terminal() {
        let (_dir, store, cfg) = scratch(&[
            Seed::new("tasks", "TASK-0001", "done"),
            Seed::new("tasks", "TASK-0002", "To Do"),
        ]);
        let data = overview(&scan(&store), &cfg, now());
        assert_eq!(data.complete, 1);

        let mut out = Vec::new();
        about_with(&scan(&store), &cfg, now(), &mut out).expect("about");
        let text = String::from_utf8(out).expect("utf8");
        assert!(
            text.contains("2 tasks, 1 complete (50.0%)"),
            "headline must carry the rate, got: {text}"
        );

        let cfg_no_terminal = BacklogConfig {
            statuses: Vec::new(),
            ..BacklogConfig::default()
        };
        let data = overview(&scan(&store), &cfg_no_terminal, now());
        assert_eq!(data.terminal, None);
        assert_eq!(data.complete, 0);
        let mut out = Vec::new();
        about_with(&scan(&store), &cfg_no_terminal, now(), &mut out).expect("about");
        let text = String::from_utf8(out).expect("utf8");
        assert!(
            text.contains("completion n/a"),
            "no terminal status must render n/a, got: {text}"
        );
    }

    /// Ready/blocked run over live non-terminal tasks against live ids only:
    /// an archive duplicate of a dependency must not flip readiness, and an
    /// unresolved dependency is missing, not blocking.
    #[test]
    fn ready_and_blocked_read_live_statuses_only() {
        let (_dir, store, cfg) = scratch(&[
            // 0001 is blocked: its dependency 0003 is live and not Done.
            Seed {
                deps: &["TASK-0003"],
                ..Seed::new("tasks", "TASK-0001", "To Do")
            },
            Seed::new("tasks", "TASK-0003", "In Progress"),
            // 0004 is ready: its dependency 0002 is live Done — even though
            // an archive duplicate of 0002 carries a To Do status, which
            // would flip 0004 to blocked if the lookup read archives.
            Seed {
                deps: &["TASK-0002"],
                ..Seed::new("tasks", "TASK-0004", "To Do")
            },
            Seed::new("tasks", "TASK-0002", "Done"),
            Seed::new("archive/tasks", "TASK-0002", "To Do"),
            // 0005 is ready: its unresolved dependency is missing, not
            // blocking (readiness_of's rule, shared with `task list`).
            Seed {
                deps: &["TASK-9999"],
                ..Seed::new("tasks", "TASK-0005", "To Do")
            },
            // Done is finished, never ready or blocked.
            Seed::new("tasks", "TASK-0006", "Done"),
            // 0007 is blocked: its dependency is the live TASK-0003,
            // referenced with different casing — the lookup must resolve
            // it, not treat it as missing.
            Seed {
                deps: &["task-0003"],
                ..Seed::new("tasks", "TASK-0007", "To Do")
            },
        ]);
        let data = overview(&scan(&store), &cfg, now());
        assert_eq!(data.ready, 3, "0003 (no deps), 0004, 0005 (missing dep)");
        assert_eq!(
            data.blocked, 2,
            "0001, and 0007 whose differently-cased dependency must resolve"
        );
    }

    /// Oldest open reads the oldest live non-terminal task; terminal tasks
    /// and unparseable dates are excluded.
    #[test]
    fn oldest_open_excludes_terminal_and_unparseable_dates() {
        let (_dir, store, cfg) = scratch(&[
            Seed {
                created: "2020-01-01 00:00",
                ..Seed::new("tasks", "TASK-0001", "Done")
            },
            Seed {
                created: "back then",
                ..Seed::new("tasks", "TASK-0002", "To Do")
            },
            Seed {
                created: "2026-01-05 08:00",
                ..Seed::new("tasks", "TASK-0003", "To Do")
            },
            Seed {
                created: "2026-02-01 00:00",
                ..Seed::new("tasks", "TASK-0004", "Triage")
            },
        ]);
        let data = overview(&scan(&store), &cfg, now());
        let oldest = data.oldest_open.as_ref().expect("oldest open");
        assert_eq!(oldest.id, "TASK-0003");
        assert_eq!(oldest.created_day, "2026-01-05");
        assert_eq!(oldest.days_old, 244);
    }

    /// Created counts every directory by `created_date`; Done counts
    /// terminal-status tasks by `updated_date` with `created_date` fallback.
    #[test]
    fn activity_windows_count_created_and_done() {
        let (_dir, store, cfg) = scratch(&[
            // Created 3 days ago: both windows.
            Seed {
                created: "2026-09-03 12:00",
                ..Seed::new("tasks", "TASK-0001", "To Do")
            },
            // Created 20 days ago: 30d window only.
            Seed {
                created: "2026-08-17 12:00",
                ..Seed::new("tasks", "TASK-0002", "To Do")
            },
            // Done, updated 2 days ago: done 7d + 30d.
            Seed {
                created: "2026-01-01 00:00",
                updated: Some("2026-09-04 12:00"),
                ..Seed::new("tasks", "TASK-0003", "Done")
            },
            // Done, no updated_date: falls back to a January created_date —
            // outside both windows.
            Seed {
                created: "2026-01-02 00:00",
                ..Seed::new("completed", "TASK-0004", "Done")
            },
            // To Do but bumped yesterday: counts nowhere — Done requires the
            // terminal status.
            Seed {
                created: "2026-01-01 00:00",
                updated: Some("2026-09-05 12:00"),
                ..Seed::new("tasks", "TASK-0005", "To Do")
            },
            // Archived Done updated 10 days ago: done 30d, proving archives
            // count.
            Seed {
                created: "2026-01-01 00:00",
                updated: Some("2026-08-27 12:00"),
                ..Seed::new("archive/tasks", "TASK-0006", "Done")
            },
        ]);
        let data = overview(&scan(&store), &cfg, now());
        assert_eq!((data.created_7d, data.created_30d), (1, 2));
        assert_eq!((data.done_7d, data.done_30d), (1, 2));
    }

    /// Priority buckets rank over live tasks, unassigned counts live tasks
    /// with no assignees, and labels keep the top 5 by count then name.
    #[test]
    fn priority_unassigned_and_top_labels() {
        // One live task carrying six labels of count 1 each: truncation
        // keeps five, the tie breaking by name.
        let labels_many = ["alpha", "beta", "gamma", "delta", "epsilon", "zeta"];
        let label_task = Seed {
            labels: &labels_many,
            ..Seed::new("tasks", "TASK-0001", "To Do")
        };
        let (_dir, store, cfg) = scratch(&[
            label_task,
            Seed {
                priority: Some("critical"),
                ..Seed::new("tasks", "TASK-0002", "To Do")
            },
            Seed {
                priority: Some("high"),
                assignees: &["someone"],
                ..Seed::new("tasks", "TASK-0003", "In Progress")
            },
            Seed {
                priority: Some("low"),
                ..Seed::new("tasks", "TASK-0004", "To Do")
            },
            Seed {
                priority: Some("odd"),
                ..Seed::new("tasks", "TASK-0005", "To Do")
            },
            // Completed-dir tasks are not live: no priority, no labels.
            Seed {
                priority: Some("critical"),
                labels: &["archived-label"],
                ..Seed::new("completed", "TASK-0006", "Done")
            },
        ]);
        let data = overview(&scan(&store), &cfg, now());
        assert_eq!(
            data.priority,
            vec![
                ("critical", 1),
                ("high", 1),
                ("medium", 0),
                ("low", 1),
                ("unset", 2),
            ],
            "odd priority and none both land in unset; completed-dir excluded"
        );
        assert_eq!(data.unassigned, 4, "only TASK-0003 has an assignee");
        assert_eq!(data.top_labels.len(), 5, "six live labels truncate to five");
        assert!(
            !data.top_labels.iter().any(|(l, _)| l == "archived-label"),
            "labels from completed/ must not count"
        );
        // All six labels carry count 1, so the tie breaks by name and the
        // five alphabetically-first survive.
        let names: Vec<&str> = data.top_labels.iter().map(|(l, _)| l.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta", "delta", "epsilon", "gamma"]);
    }

    /// The rendered page carries every section marker; a backlog with no
    /// live tasks omits the live-only sections.
    #[test]
    fn render_carries_all_sections_and_omits_live_sections_when_no_live() {
        let (_dir, store, cfg) = scratch(&[
            Seed {
                labels: &["security"],
                ..Seed::new("tasks", "TASK-0001", "To Do")
            },
            Seed::new("completed", "TASK-0002", "Done"),
        ]);
        let mut out = Vec::new();
        about_with(&scan(&store), &cfg, now(), &mut out).expect("about");
        let text = String::from_utf8(out).expect("utf8");
        for marker in [
            "Backlog overview — 2 tasks, 1 complete (50.0%)",
            "Status",
            "Total",
            "Live work",
            "Ready to start:",
            "Oldest open: TASK-0001",
            "Priority (live):",
            "Activity",
            "Created: 0 in the last 7 days, 0 in the last 30 days",
            "Top labels (live): security 1",
        ] {
            assert!(text.contains(marker), "missing {marker:?} in:\n{text}");
        }

        let (_dir, store, cfg) = scratch(&[Seed::new("completed", "TASK-0001", "Done")]);
        let mut out = Vec::new();
        about_with(&scan(&store), &cfg, now(), &mut out).expect("about");
        let text = String::from_utf8(out).expect("utf8");
        assert!(!text.contains("Live work"), "no live tasks: {text}");
        assert!(!text.contains("Priority (live):"), "no live tasks: {text}");
        assert!(!text.contains("Top labels"), "no live tasks: {text}");
        assert!(
            text.contains("Activity"),
            "activity counts all dirs: {text}"
        );
    }
}
