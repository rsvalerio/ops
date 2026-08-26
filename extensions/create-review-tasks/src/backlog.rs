//! backlog.md task-file layout: id allocation, filename slugs, and
//! frontmatter rendering.
//!
//! The shapes mirror what `backlog task create` (backlog.md CLI v1.50.1)
//! writes, so files produced here are indistinguishable from CLI-created
//! ones when re-read by the CLI: `task-NNNN - <slug>.md` for main tasks,
//! `task-NNNN.MM - <slug>.md` dotted ids with a `parent_task_id` field for
//! subtasks, zero-padded 4-digit ids, and UTC `created_date`.

use std::io::Write;
use std::path::Path;

use crate::clock::UtcStamp;

/// backlog.md directories (relative to the `.backlog` root, itself relative
/// to the workspace root) that can hold task markdown files. `tasks` is the
/// only one required to exist; the others are scanned when present because
/// id allocation must never collide with an archived or completed task.
const TASK_DIRS: &[&str] = &["tasks", "completed", "archive/tasks", "archive/completed"];

/// Main-task frontmatter labels, in order.
const MAIN_LABELS: &[&str] = &["code-review-request", "code-review", "qa"];
/// Subtask frontmatter labels, in order.
const SUBTASK_LABELS: &[&str] = &["code-review", "qa"];

/// Ordinal the backlog CLI assigns to a freshly created parent task.
const MAIN_ORDINAL: u32 = 1_000;
/// Ordinal base the backlog CLI assigns to subtasks (child i → 2000 + i).
const SUBTASK_ORDINAL_BASE: u32 = 2_000;

/// Ensure the backlog tree this writer targets actually exists. ERR-13: the
/// error names the missing directory so the operator knows what to create.
pub(crate) fn require_backlog_tasks_dir(workspace_root: &Path) -> anyhow::Result<()> {
    let tasks_dir = workspace_root.join(".backlog").join("tasks");
    if tasks_dir.is_dir() {
        Ok(())
    } else {
        anyhow::bail!(
            "no {} directory found — run `backlog init` in {} before creating review tasks",
            tasks_dir.display(),
            workspace_root.display()
        )
    }
}

/// Next free main-task number: one more than the highest `task-<n>` id found
/// across every [`TASK_DIRS`] directory that exists (dotted subtask ids share
/// their parent's number, so the integer part alone determines allocation).
/// Returns 1 for an empty backlog.
pub(crate) fn next_main_task_number(workspace_root: &Path) -> u32 {
    let backlog_root = workspace_root.join(".backlog");
    let mut max = 0u32;
    for_each_task_file(&backlog_root, |_dir, file_name| {
        if let Some(n) = leading_task_number(file_name) {
            max = max.max(n);
        }
    });
    max.saturating_add(1)
}

/// Sequence number for a `review-request-<date>-<n>` main task created on
/// `date`: one more than the highest `<n>` already present in any task
/// filename whose slug starts with `review-request-<date>-`. Returns 1 when
/// this is the first request of the day.
pub(crate) fn next_daily_sequence(workspace_root: &Path, date: &str) -> u32 {
    let prefix = format!("review-request-{date}-");
    let backlog_root = workspace_root.join(".backlog");
    let mut max = 0u32;
    for_each_task_file(&backlog_root, |_dir, file_name| {
        let Some((_, rest)) = file_name.split_once(prefix.as_str()) else {
            return;
        };
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(n) = digits.parse::<u32>() {
            max = max.max(n);
        }
    });
    max.saturating_add(1)
}

/// The integer part of a `task-<n>` / `task-<n>.<mm>` filename, if the name
/// starts with the `task-` prefix followed by at least one digit.
fn leading_task_number(file_name: &str) -> Option<u32> {
    let rest = file_name.strip_prefix("task-")?;
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    digits.parse::<u32>().ok()
}

/// Invoke `f` for every entry name in each existing [`TASK_DIRS`] directory.
/// Read errors and non-UTF-8 names are skipped silently here: id allocation
/// treats an unreadable directory like an absent one rather than failing the
/// whole command (the required `tasks` dir existence is checked separately by
/// [`require_backlog_tasks_dir`]).
fn for_each_task_file(backlog_root: &Path, mut f: impl FnMut(&str, &str)) {
    for dir in TASK_DIRS {
        let Ok(entries) = std::fs::read_dir(backlog_root.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                f(dir, &name);
            }
        }
    }
}

/// Filename slug matching the backlog CLI's observed behaviour: runs of
/// characters outside `[A-Za-z0-9._-]` collapse to a single `-`, and
/// leading/trailing `-` are trimmed. Case is preserved.
///
/// Observed CLI samples this pins:
/// - `"Main task"` → `Main-task`
/// - `"REVIEW: Run skill code-review-rust against ops-core"`
///   → `REVIEW-Run-skill-code-review-rust-against-ops-core`
#[must_use = "slugify is pure; discarding it means the title was formatted for nothing"]
pub(crate) fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut in_run = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == '-' {
            out.push(ch);
            in_run = false;
        } else if !in_run {
            out.push('-');
            in_run = true;
        }
    }
    out.trim_matches('-').to_string()
}

/// Render one task markdown file (frontmatter only, no body sections) into
/// `w`. `subtask_of` is the parent's zero-padded id (`"TASK-1671"`) plus the
/// 1-based subtask position for a subtask, or `None` for the main task.
///
/// PERF-13: writes go straight into `w`; no intermediate `String` per line.
pub(crate) fn render_task_file<W: Write>(
    w: &mut W,
    id: &str,
    title: &str,
    stamp: &UtcStamp,
    subtask_of: Option<(&str, usize)>,
) -> std::io::Result<()> {
    let (labels, ordinal) = match subtask_of {
        None => (MAIN_LABELS, MAIN_ORDINAL),
        Some((_parent_id, index)) => (
            SUBTASK_LABELS,
            SUBTASK_ORDINAL_BASE + u32::try_from(index).unwrap_or(u32::MAX),
        ),
    };
    writeln!(w, "---")?;
    writeln!(w, "id: {id}")?;
    writeln!(w, "title: {}", yaml_single_quoted(title))?;
    writeln!(w, "status: To Do")?;
    writeln!(w, "assignee: []")?;
    writeln!(w, "created_date: '{} {}'", stamp.date, stamp.minutes)?;
    writeln!(w, "labels:")?;
    for label in labels {
        writeln!(w, "  - {label}")?;
    }
    writeln!(w, "dependencies: []")?;
    if let Some((parent_id, _)) = subtask_of {
        writeln!(w, "parent_task_id: {parent_id}")?;
    }
    writeln!(w, "priority: low")?;
    writeln!(w, "ordinal: {ordinal}")?;
    writeln!(w, "---")?;
    Ok(())
}

/// YAML single-quoted scalar: wrap in `'` and double any embedded `'`.
/// The backlog CLI quotes titles containing `: `; quoting unconditionally is
/// byte-compatible for our titles and safe for any future label/title shape.
fn yaml_single_quoted(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('\'');
    out.push_str(&value.replace('\'', "''"));
    out.push('\'');
    out
}

/// Zero-padded task id string (`TASK-0042`) for a main-task number.
pub(crate) fn main_task_id(number: u32) -> String {
    format!("TASK-{number:04}")
}

/// Filename for a main task: `task-0042 - <slug>.md`.
pub(crate) fn main_task_file_name(number: u32, title: &str) -> String {
    format!("task-{number:04} - {}.md", slugify(title))
}

/// Id for the subtask at 1-based `index` under main-task `number`:
/// `TASK-0042.03`.
pub(crate) fn subtask_id(number: u32, index: usize) -> String {
    format!("TASK-{number:04}.{index:02}")
}

/// Filename for a subtask: `task-0042.03 - <slug>.md`.
pub(crate) fn subtask_file_name(number: u32, index: usize, title: &str) -> String {
    format!("task-{number:04}.{index:02} - {}.md", slugify(title))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_backlog(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for &(sub, name) in files {
            let d = dir.path().join(".backlog").join(sub);
            std::fs::create_dir_all(&d).expect("create dir");
            std::fs::write(d.join(name), "").expect("write file");
        }
        std::fs::create_dir_all(dir.path().join(".backlog").join("tasks")).expect("tasks dir");
        dir
    }

    #[test]
    fn slugify_matches_backlog_cli_observed_shapes() {
        assert_eq!(slugify("Main task"), "Main-task");
        assert_eq!(
            slugify("REVIEW: Run skill code-review-rust against ops-core"),
            "REVIEW-Run-skill-code-review-rust-against-ops-core"
        );
    }

    #[test]
    fn slugify_collapses_runs_and_trims_edges() {
        assert_eq!(slugify("a  b"), "a-b");
        assert_eq!(slugify("?!leading"), "leading");
        assert_eq!(slugify("trailing??"), "trailing");
        assert_eq!(slugify("keep.dots_and-dashes"), "keep.dots_and-dashes");
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn next_number_starts_at_one_for_empty_backlog() {
        let dir = scratch_backlog(&[]);
        assert_eq!(next_main_task_number(dir.path()), 1);
    }

    #[test]
    fn next_number_ignores_dotted_subtask_fraction() {
        let dir = scratch_backlog(&[("tasks", "task-0007.09 - child.md")]);
        assert_eq!(next_main_task_number(dir.path()), 8);
    }

    /// Id allocation must never reuse a number that lives in `completed`
    /// or either archive directory, or the CLI would treat the new task as
    /// the resurrected old one.
    #[test]
    fn next_number_scans_completed_and_archive_dirs() {
        let dir = scratch_backlog(&[
            ("tasks", "task-0010 - open.md"),
            ("completed", "task-0500 - done.md"),
            ("archive/tasks", "task-1670 - archived.md"),
            ("archive/completed", "task-0003 - old.md"),
        ]);
        assert_eq!(next_main_task_number(dir.path()), 1671);
    }

    #[test]
    fn next_number_skips_non_task_files_and_bare_prefix() {
        let dir = scratch_backlog(&[
            ("tasks", "task-not-a-number.md"),
            ("tasks", "notes.md"),
            ("tasks", "task-0020 - real.md"),
        ]);
        assert_eq!(next_main_task_number(dir.path()), 21);
    }

    #[test]
    fn daily_sequence_starts_at_one() {
        let dir = scratch_backlog(&[]);
        assert_eq!(next_daily_sequence(dir.path(), "2026-08-20"), 1);
    }

    #[test]
    fn daily_sequence_increments_past_same_day_requests() {
        let dir = scratch_backlog(&[
            ("tasks", "task-1671 - review-request-2026-08-20-1.md"),
            ("tasks", "task-1671.01 - REVIEW-Run-skill.md"),
            ("completed", "task-1600 - review-request-2026-08-20-2.md"),
        ]);
        assert_eq!(next_daily_sequence(dir.path(), "2026-08-20"), 3);
    }

    /// A request from another day must not inflate today's sequence, and a
    /// same-prefix different-day date (2026-08-2 vs 2026-08-20) must not be
    /// confused with today either — the date prefix includes the trailing
    /// `-` precisely so `…-08-2-…` cannot match `…-08-20-…`.
    #[test]
    fn daily_sequence_ignores_other_days() {
        let dir = scratch_backlog(&[
            ("tasks", "task-1600 - review-request-2026-08-19-4.md"),
            ("tasks", "task-1601 - review-request-2026-08-2-9.md"),
        ]);
        assert_eq!(next_daily_sequence(dir.path(), "2026-08-20"), 1);
    }

    #[test]
    fn daily_sequence_skips_non_numeric_suffix() {
        let dir = scratch_backlog(&[("tasks", "task-1600 - review-request-2026-08-20-notes.md")]);
        assert_eq!(next_daily_sequence(dir.path(), "2026-08-20"), 1);
    }

    #[test]
    fn require_backlog_tasks_dir_rejects_missing_tree() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = require_backlog_tasks_dir(dir.path()).expect_err("must fail");
        assert!(
            err.to_string().contains("backlog init"),
            "error must hint at backlog init, got: {err:#}"
        );
    }

    #[test]
    fn require_backlog_tasks_dir_accepts_existing_tree() {
        let dir = scratch_backlog(&[]);
        require_backlog_tasks_dir(dir.path()).expect("must pass");
    }

    /// Byte-shape of a main-task file as the backlog CLI writes it
    /// (golden test against the shapes captured from `backlog task create`).
    #[test]
    fn render_main_task_matches_cli_shape() {
        let stamp = UtcStamp {
            date: "2026-08-20".to_string(),
            minutes: "19:02".to_string(),
        };
        let mut buf = Vec::new();
        render_task_file(
            &mut buf,
            "TASK-1671",
            "review-request-2026-08-20-1",
            &stamp,
            None,
        )
        .expect("render");
        let expected = concat!(
            "---\n",
            "id: TASK-1671\n",
            "title: 'review-request-2026-08-20-1'\n",
            "status: To Do\n",
            "assignee: []\n",
            "created_date: '2026-08-20 19:02'\n",
            "labels:\n",
            "  - code-review-request\n",
            "  - code-review\n",
            "  - qa\n",
            "dependencies: []\n",
            "priority: low\n",
            "ordinal: 1000\n",
            "---\n",
        );
        assert_eq!(String::from_utf8(buf).expect("utf8"), expected);
    }

    #[test]
    fn render_subtask_matches_cli_shape() {
        let stamp = UtcStamp {
            date: "2026-08-20".to_string(),
            minutes: "19:02".to_string(),
        };
        let mut buf = Vec::new();
        render_task_file(
            &mut buf,
            "TASK-1671.01",
            "REVIEW: Run skill code-review-rust against ops-core",
            &stamp,
            Some(("TASK-1671", 1)),
        )
        .expect("render");
        let expected = concat!(
            "---\n",
            "id: TASK-1671.01\n",
            "title: 'REVIEW: Run skill code-review-rust against ops-core'\n",
            "status: To Do\n",
            "assignee: []\n",
            "created_date: '2026-08-20 19:02'\n",
            "labels:\n",
            "  - code-review\n",
            "  - qa\n",
            "dependencies: []\n",
            "parent_task_id: TASK-1671\n",
            "priority: low\n",
            "ordinal: 2001\n",
            "---\n",
        );
        assert_eq!(String::from_utf8(buf).expect("utf8"), expected);
    }

    #[test]
    fn yaml_single_quoted_doubles_embedded_quotes() {
        assert_eq!(yaml_single_quoted("it's"), "'it''s'");
    }

    #[test]
    fn file_name_helpers_format_ids_and_slugs() {
        assert_eq!(
            main_task_file_name(42, "review-request-2026-08-20-1"),
            "task-0042 - review-request-2026-08-20-1.md"
        );
        assert_eq!(subtask_id(42, 3), "TASK-0042.03");
        assert_eq!(
            subtask_file_name(42, 3, "REVIEW: Run against x"),
            "task-0042.03 - REVIEW-Run-against-x.md"
        );
    }
}
