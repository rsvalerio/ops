//! Backlog tree store: directory scan, scoped task lookup, and id
//! allocation over the `.backlog` markdown tree.
//!
//! Layout (relative to the backlog root, itself relative to the workspace
//! root): `tasks/` holds every live task regardless of status; `completed/`
//! and `archive/tasks/` (+ `archive/completed/`) hold relocated files whose
//! numbers must never be re-allocated. `task list` and `search` scan
//! `tasks/` only — matching the backlog CLI, which never surfaces completed
//! or archived tasks in listings — while `task view` resolves ids across
//! `tasks/` → `completed/` → `archive/tasks/` in that precedence order
//! (the corpus contains four id collisions between `completed/` and
//! `archive/tasks/`).

use std::path::{Path, PathBuf};

use crate::model::TaskDoc;

/// backlog.md directories (relative to the backlog root) that can hold task
/// markdown files.
///
/// `tasks` is the only one required to exist; the others are scanned when
/// present because id allocation must never collide with an archived or
/// completed task.
pub const TASK_DIRS: &[&str] = &["tasks", "completed", "archive/tasks", "archive/completed"];

/// Directories searched — in this order — to resolve one task id for
/// `task view` / `task edit`.
const LOOKUP_DIRS: &[&str] = &["tasks", "completed", "archive/tasks"];

/// One task file found by a scan: its path and parsed document.
pub struct TaskEntry {
    pub path: PathBuf,
    pub doc: TaskDoc,
}

/// The backlog tree rooted at one backlog directory (`.backlog` by default).
#[derive(Debug, Clone)]
pub struct Store {
    backlog_root: PathBuf,
}

impl Store {
    /// Open the store, requiring the `tasks/` directory to exist.
    ///
    /// # Errors
    ///
    /// No `<root>/tasks` directory (ERR-13: the error names the missing
    /// directory so the operator knows what to create).
    pub fn open(backlog_root: &Path) -> anyhow::Result<Self> {
        let tasks_dir = backlog_root.join("tasks");
        if tasks_dir.is_dir() {
            Ok(Self {
                backlog_root: backlog_root.to_path_buf(),
            })
        } else {
            anyhow::bail!(
                "no {} directory found — run `backlog init` in the workspace before \
                 managing tasks",
                tasks_dir.display()
            )
        }
    }

    /// Every parseable task in `tasks/`, sorted by numeric id ascending.
    ///
    /// # Errors
    ///
    /// A task file that exists but does not parse; the error names the file.
    pub fn scan_tasks(&self) -> anyhow::Result<Vec<TaskEntry>> {
        let mut entries = Vec::new();
        let dir = self.backlog_root.join("tasks");
        let read = std::fs::read_dir(&dir)
            .map_err(|e| anyhow::anyhow!("reading {}: {e}", dir.display()))?;
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let src = std::fs::read_to_string(&path)
                .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
            let doc = TaskDoc::parse(&src)
                .map_err(|e| anyhow::anyhow!("parsing {}: {e:#}", path.display()))?;
            entries.push(TaskEntry { path, doc });
        }
        entries.sort_by_key(|e| leading_task_number_of(&e.doc.frontmatter.id).unwrap_or(0));
        Ok(entries)
    }

    /// Resolve one task id across [`LOOKUP_DIRS`] in precedence order.
    /// Matching is case-insensitive on the full `TASK-NNNN` id.
    #[must_use = "looking up without using the result finds the file for nothing"]
    pub fn find(&self, id: &str) -> Option<TaskEntry> {
        let wanted = id.to_ascii_lowercase();
        for dir in LOOKUP_DIRS {
            let dir_path = self.backlog_root.join(dir);
            let Ok(read) = std::fs::read_dir(&dir_path) else {
                continue;
            };
            for entry in read.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let Ok(src) = std::fs::read_to_string(&path) else {
                    continue;
                };
                let Ok(doc) = TaskDoc::parse(&src) else {
                    continue;
                };
                if doc.frontmatter.id.to_ascii_lowercase() == wanted {
                    return Some(TaskEntry { path, doc });
                }
            }
        }
        None
    }

    /// Next free main-task number: one more than the highest `task-<n>` id
    /// across every [`TASK_DIRS`] directory that exists. Dotted subtask ids
    /// share their parent's number, so the integer part alone determines
    /// allocation; a number living only in `completed/` or an archive is
    /// still taken, or the CLI would treat the new task as the resurrected
    /// old one.
    #[must_use = "allocating without using the number burns the id for nothing"]
    pub fn next_task_number(&self) -> u32 {
        let mut max_number = 0u32;
        for_each_task_file(&self.backlog_root, |_dir, file_name| {
            if let Some(number) = TaskFileName::parse(file_name).and_then(|parsed| parsed.number) {
                max_number = max_number.max(number);
            }
        });
        max_number.saturating_add(1)
    }

    /// Absolute path a new `task-<n> - <slug>.md` file would take in
    /// `tasks/`.
    #[must_use = "the path is derived; discarding it re-derives nothing"]
    pub fn task_path(&self, file_name: &str) -> PathBuf {
        self.backlog_root.join("tasks").join(file_name)
    }
}

/// The integer part of a `TASK-<n>` / `TASK-<n>.<mm>` id, when the id
/// carries a numeric part.
fn leading_task_number_of(id: &str) -> Option<u32> {
    let digits = id
        .split_once('-')
        .and_then(|(_, rest)| rest.split('.').next())
        .filter(|rest| !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit()))?;
    digits.parse::<u32>().ok()
}

/// A backlog task filename split into id and slug:
/// `task-<n>[.<mm>] - <slug>.md`.
pub struct TaskFileName<'a> {
    /// Integer part of the `task-<n>` id, when the name carries one.
    pub number: Option<u32>,
    /// Slug between the `" - "` separator and the `.md` extension.
    pub slug: &'a str,
}

impl<'a> TaskFileName<'a> {
    /// Split `file_name`, or `None` when it is not a markdown file at all.
    ///
    /// Every file the backlog CLI writes is `<id> - <slug>.md`; a name with
    /// no `" - "` separator still yields its id, so id allocation keeps
    /// seeing a slugless `task-0500.md` and can never hand its number out
    /// twice. Such a name has an empty slug, which matches no title (slugs
    /// are always non-empty).
    #[must_use = "parsing without using the split discards the scan it was parsed for"]
    pub fn parse(file_name: &'a str) -> Option<Self> {
        let stem = file_name.strip_suffix(".md")?;
        let (id, slug) = stem.split_once(" - ").unwrap_or((stem, ""));
        Some(Self {
            number: leading_task_number(id),
            slug,
        })
    }
}

/// The integer part of a `task-<n>` / `task-<n>.<mm>` id, if it starts with
/// the `task-` prefix followed by at least one digit.
fn leading_task_number(id: &str) -> Option<u32> {
    let digits = id
        .strip_prefix("task-")?
        .split(|c: char| !c.is_ascii_digit())
        .next()
        .filter(|d| !d.is_empty())?;
    digits.parse::<u32>().ok()
}

/// Invoke `f` for every entry name in each existing [`TASK_DIRS`] directory.
///
/// Read errors and non-UTF-8 names are skipped silently here: id allocation
/// treats an unreadable directory like an absent one rather than failing the
/// whole command (the required `tasks` dir existence is checked separately
/// by [`Store::open`]).
pub fn for_each_task_file(backlog_root: &Path, mut f: impl FnMut(&str, &str)) {
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

/// Zero-padded task id string (`TASK-0042`) for a main-task number under the
/// configured prefix and padding.
#[must_use = "formatting without using the string is dead work"]
pub fn format_task_id(prefix: &str, number: u32, pad: usize) -> String {
    format!("{prefix}-{number:0pad$}")
}

/// Filename for a main task: `task-0042 - <slug>.md`, slug via
/// [`crate::model::file_slug`].
#[must_use = "the filename must reach the write that reserves the id"]
pub fn main_task_file_name(number: u32, pad: usize, title: &str) -> String {
    format!(
        "task-{number:0pad$} - {}.md",
        crate::model::file_slug(title)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch_backlog(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for &(sub, name) in files {
            let d = dir.path().join(".backlog").join(sub);
            std::fs::create_dir_all(&d).expect("create dir");
            let digits: String = name
                .trim_start_matches("task-")
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            let content = format!(
                "---\nid: TASK-{digits:0>4}\ntitle: 't'\nstatus: To Do\nassignee: []\ncreated_date: '2026-01-01 00:00'\nlabels: []\ndependencies: []\n---\n"
            );
            std::fs::write(d.join(name), content).expect("write file");
        }
        std::fs::create_dir_all(dir.path().join(".backlog").join("tasks")).expect("tasks dir");
        dir
    }

    #[test]
    fn open_rejects_missing_tasks_dir_naming_it() {
        let dir = tempfile::tempdir().expect("tempdir");
        let err = Store::open(&dir.path().join(".backlog")).expect_err("must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("tasks"),
            "error must name the missing directory, got: {rendered}"
        );
    }

    #[test]
    fn next_number_scans_all_four_dirs() {
        let dir = scratch_backlog(&[
            ("tasks", "task-0010 - open.md"),
            ("completed", "task-0500 - done.md"),
            ("archive/tasks", "task-1667 - archived.md"),
            ("archive/completed", "task-0003 - old.md"),
        ]);
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        assert_eq!(store.next_task_number(), 1668);
    }

    #[test]
    fn next_number_starts_at_one_for_empty_backlog() {
        let dir = scratch_backlog(&[]);
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        assert_eq!(store.next_task_number(), 1);
    }

    #[test]
    fn next_number_ignores_dotted_subtask_fraction() {
        let dir = scratch_backlog(&[("tasks", "task-0007.09 - child.md")]);
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        assert_eq!(store.next_task_number(), 8);
    }

    #[test]
    fn find_prefers_tasks_then_completed_then_archive() {
        // The corpus's four duplicate ids exist because completed/ and
        // archive/tasks/ hold *different* tasks under one id; precedence
        // must pick tasks/ first, completed/ second.
        let dir = scratch_backlog(&[
            ("tasks", "task-0059 - live.md"),
            ("completed", "task-0059 - done.md"),
            ("archive/tasks", "task-0059 - archived.md"),
        ]);
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        let found = store.find("TASK-0059").expect("must find");
        assert!(
            found.path.ends_with("task-0059 - live.md"),
            "tasks/ must win, got {}",
            found.path.display()
        );

        let dir = scratch_backlog(&[
            ("completed", "task-0060 - done.md"),
            ("archive/tasks", "task-0060 - archived.md"),
        ]);
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        let found = store.find("task-0060").expect("must find");
        assert!(
            found.path.ends_with("task-0060 - done.md"),
            "completed/ must beat archive/, got {}",
            found.path.display()
        );
    }

    #[test]
    fn find_is_case_insensitive_and_misses_cleanly() {
        let dir = scratch_backlog(&[("tasks", "task-0012 - x.md")]);
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        assert!(store.find("task-0012").is_some());
        assert!(store.find("TASK-9999").is_none());
    }

    #[test]
    fn scan_tasks_sorts_by_numeric_id_and_skips_non_md() {
        let dir = scratch_backlog(&[
            ("tasks", "task-0020 - b.md"),
            ("tasks", "task-0003 - a.md"),
            ("tasks", "notes.txt"),
        ]);
        std::fs::write(dir.path().join(".backlog/tasks/notes.txt"), "x").expect("write");
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        let scanned = store.scan_tasks().expect("scan");
        let ids: Vec<&str> = scanned
            .iter()
            .map(|e| e.doc.frontmatter.id.as_str())
            .collect();
        assert_eq!(ids, vec!["TASK-0003", "TASK-0020"]);
    }

    #[test]
    fn format_helpers() {
        assert_eq!(format_task_id("TASK", 42, 4), "TASK-0042");
        assert_eq!(
            main_task_file_name(42, 4, "REVIEW: Run against x"),
            "task-0042 - REVIEW-Run-against-x.md"
        );
    }

    /// A slugless name still reserves its number: allocation must never hand
    /// out an id some file already carries.
    #[test]
    fn next_number_reserves_slugless_names() {
        let dir = scratch_backlog(&[("tasks", "task-0030.md"), ("tasks", "task-0020 - real.md")]);
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        assert_eq!(store.next_task_number(), 31);
    }
}
