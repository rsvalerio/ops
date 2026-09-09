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
#[derive(Debug)]
pub struct TaskEntry {
    pub path: PathBuf,
    pub doc: TaskDoc,
}

/// Which [`TASK_DIRS`] directory a task file lives in.
///
/// Archiving is a file-location concept only — the bytes (and the status
/// inside them) are unchanged by a move — so "archived" is answered by
/// location, not frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskLocation {
    /// `tasks/` — every live task regardless of status.
    Tasks,
    /// `completed/` — terminal-status tasks moved by cleanup.
    Completed,
    /// `archive/tasks/` — archived by the external backlog CLI.
    ArchiveTasks,
    /// `archive/completed/` — archived by the external backlog CLI.
    ArchiveCompleted,
}

impl TaskLocation {
    /// Map a [`TASK_DIRS`] relative directory name to its location.
    #[must_use = "mapping without using the result discards the parse"]
    pub fn from_dir(dir: &str) -> Option<Self> {
        match dir {
            "tasks" => Some(Self::Tasks),
            "completed" => Some(Self::Completed),
            "archive/tasks" => Some(Self::ArchiveTasks),
            "archive/completed" => Some(Self::ArchiveCompleted),
            _ => None,
        }
    }

    /// True for `tasks/` — the only directory `scan_tasks` reads and the only
    /// one whose ids are unique (the corpus holds id collisions between
    /// `completed/` and `archive/tasks/`).
    #[must_use = "the answer exists to be branched on"]
    pub const fn is_live(self) -> bool {
        matches!(self, Self::Tasks)
    }
}

/// One task file found by an all-dirs scan: its path, its directory, and its
/// parsed document.
#[derive(Debug)]
pub struct LocatedTask {
    pub path: PathBuf,
    pub location: TaskLocation,
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

    /// Every parseable task in every [`TASK_DIRS`] directory that exists,
    /// each tagged with its location. Absent directories are skipped (only
    /// `tasks/` is required, by [`Store::open`]); within each directory the
    /// same rules as [`Store::scan_tasks`] apply — non-`.md` files skipped,
    /// a read or parse failure errors naming the file. Order is
    /// [`TASK_DIRS`] declaration order, then numeric id ascending within
    /// each directory.
    ///
    /// Duplicate ids across directories are returned as-is (the corpus holds
    /// real collisions between `completed/` and `archive/tasks/`); callers
    /// keying by id must use the live entries only, whose ids are unique.
    ///
    /// # Errors
    ///
    /// A directory exists but cannot be read (the error names the
    /// directory), or a task file that exists does not parse (the error
    /// names the file).
    pub fn scan_all_tasks(&self) -> anyhow::Result<Vec<LocatedTask>> {
        let mut located = Vec::new();
        for dir in TASK_DIRS {
            let Some(location) = TaskLocation::from_dir(dir) else {
                continue;
            };
            let dir_path = self.backlog_root.join(dir);
            // Only a missing directory is skippable (only `tasks/` is
            // required); any other read failure — permissions, a file where
            // a directory belongs — must surface rather than silently
            // under-count the tree.
            let read = match std::fs::read_dir(&dir_path) {
                Ok(read) => read,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => {
                    return Err(anyhow::anyhow!("reading {}: {err}", dir_path.display()));
                }
            };
            let mut entries = Vec::new();
            for entry in read.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                let src = std::fs::read_to_string(&path)
                    .map_err(|e| anyhow::anyhow!("reading {}: {e}", path.display()))?;
                let doc = TaskDoc::parse(&src)
                    .map_err(|e| anyhow::anyhow!("parsing {}: {e:#}", path.display()))?;
                entries.push(LocatedTask {
                    path,
                    location,
                    doc,
                });
            }
            entries.sort_by_key(|l| leading_task_number_of(&l.doc.frontmatter.id).unwrap_or(0));
            located.extend(entries);
        }
        Ok(located)
    }

    /// Resolve one task id across [`LOOKUP_DIRS`] in precedence order.
    /// Matching is case-insensitive on the full `TASK-NNNN` id.
    ///
    /// Tolerance rule: a lookup directory that is merely absent is skipped
    /// (only `tasks/` is required, by [`Store::open`]), and a task file that
    /// exists but fails to read or parse is skipped — a damaged neighbour
    /// file must not hide an intact task elsewhere in the tree.
    ///
    /// # Errors
    ///
    /// A lookup directory exists but cannot be read (the error names the
    /// directory), mirroring [`Store::scan_all_tasks`].
    pub fn find(&self, id: &str) -> anyhow::Result<Option<TaskEntry>> {
        let wanted = id.to_ascii_lowercase();
        for dir in LOOKUP_DIRS {
            let dir_path = self.backlog_root.join(dir);
            // Only a missing directory is skippable; any other read failure —
            // permissions, a file where a directory belongs — must surface
            // rather than read as "not found".
            let read = match std::fs::read_dir(&dir_path) {
                Ok(read) => read,
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
                Err(err) => {
                    return Err(anyhow::anyhow!("reading {}: {err}", dir_path.display()));
                }
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
                    return Ok(Some(TaskEntry { path, doc }));
                }
            }
        }
        Ok(None)
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

    /// Absolute path of the `completed/` directory, where `cleanup` moves
    /// terminal-status task files. Created on demand by the caller; a scan
    /// never requires it to exist.
    #[must_use = "the path is derived; discarding it re-derives nothing"]
    pub fn completed_dir(&self) -> PathBuf {
        self.backlog_root.join("completed")
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

/// Find the first task file for which `f` returns `Some`, stopping the walk
/// at that entry.
///
/// PERF-3 / TASK-2131: early-exit twin of [`for_each_task_file`] — a
/// conflict re-check only needs the first match, and a real backlog tree
/// holds thousands of files, so enumerating the rest of the tree after the
/// answer is decided is pure I/O (retried up to 32 times per contended
/// allocation). Directory order, silent error skipping and non-UTF-8
/// handling match [`for_each_task_file`].
pub fn find_task_file<B>(
    backlog_root: &Path,
    mut f: impl FnMut(&str, &str) -> Option<B>,
) -> Option<B> {
    for dir in TASK_DIRS {
        let Ok(entries) = std::fs::read_dir(backlog_root.join(dir)) else {
            continue;
        };
        for entry in entries.flatten() {
            if let Ok(name) = entry.file_name().into_string() {
                if let Some(found) = f(dir, &name) {
                    return Some(found);
                }
            }
        }
    }
    None
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

    /// PERF-3 / TASK-2131: `find_task_file` must stop the walk at the first
    /// match. The target sits in `tasks` and the non-matches in `completed`
    /// (a later [`TASK_DIRS`] directory), so if the walk continued past the
    /// match the closure would be invoked for the `completed` entries too —
    /// pinned by the visited count, which is immune to `read_dir`'s
    /// intra-directory ordering.
    #[test]
    fn find_task_file_stops_at_first_match() {
        let dir = scratch_backlog(&[
            ("tasks", "task-0002 - beta.md"),
            ("completed", "task-0001 - alpha.md"),
            ("completed", "task-0003 - gamma.md"),
        ]);
        let mut visited = 0usize;
        let found = find_task_file(&dir.path().join(".backlog"), |_dir, name| {
            visited += 1;
            (name == "task-0002 - beta.md").then(|| name.to_string())
        });
        assert_eq!(found.as_deref(), Some("task-0002 - beta.md"));
        assert_eq!(
            visited, 1,
            "walk must stop at the match, not enumerate the rest of the tree"
        );
    }

    /// PERF-3 / TASK-2131: no match means `None`, with every entry in every
    /// existing directory visited — the full-walk contract when nothing
    /// matches.
    #[test]
    fn find_task_file_returns_none_when_nothing_matches() {
        let dir = scratch_backlog(&[
            ("tasks", "task-0001 - alpha.md"),
            ("completed", "task-0003 - gamma.md"),
        ]);
        let mut visited = 0usize;
        let found = find_task_file(&dir.path().join(".backlog"), |_dir, _name| {
            visited += 1;
            None::<String>
        });
        assert!(found.is_none());
        assert_eq!(visited, 2, "both entries must have been examined");
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
        let found = store.find("TASK-0059").expect("lookup").expect("must find");
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
        let found = store.find("task-0060").expect("lookup").expect("must find");
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
        assert!(store.find("task-0012").expect("lookup").is_some());
        assert!(store.find("TASK-9999").expect("lookup").is_none());
    }

    /// A `read_dir` failure other than `NotFound` must surface from `find`
    /// naming the directory — here, a file squatting where `completed/`
    /// belongs — instead of degrading to `None` and reading as "not found".
    #[test]
    fn find_read_failure_names_the_directory() {
        let dir = scratch_backlog(&[("tasks", "task-0001 - live.md")]);
        std::fs::write(dir.path().join(".backlog/completed"), "not a dir").expect("write file");
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        let err = store.find("TASK-9999").expect_err("must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("completed"),
            "error must name the unreadable directory, got: {rendered}"
        );
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

    #[test]
    fn scan_all_buckets_each_dir_and_orders_within_it() {
        let dir = scratch_backlog(&[
            ("tasks", "task-0020 - b.md"),
            ("tasks", "task-0003 - a.md"),
            ("completed", "task-0500 - done.md"),
            ("archive/tasks", "task-1667 - archived.md"),
            ("archive/completed", "task-0009 - old.md"),
            ("tasks", "notes.txt"),
        ]);
        std::fs::write(dir.path().join(".backlog/tasks/notes.txt"), "x").expect("write");
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        let scanned = store.scan_all_tasks().expect("scan");
        let seen: Vec<(TaskLocation, &str)> = scanned
            .iter()
            .map(|l| (l.location, l.doc.frontmatter.id.as_str()))
            .collect();
        assert_eq!(
            seen,
            vec![
                (TaskLocation::Tasks, "TASK-0003"),
                (TaskLocation::Tasks, "TASK-0020"),
                (TaskLocation::Completed, "TASK-0500"),
                (TaskLocation::ArchiveTasks, "TASK-1667"),
                (TaskLocation::ArchiveCompleted, "TASK-0009"),
            ],
            "TASK_DIRS declaration order, numeric id within each dir, non-md skipped"
        );
    }

    /// Only `tasks/` is required; the other three directories are skipped
    /// when absent, not errors.
    #[test]
    fn scan_all_skips_absent_dirs() {
        let dir = scratch_backlog(&[("tasks", "task-0001 - only.md")]);
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        let scanned = store.scan_all_tasks().expect("scan");
        assert_eq!(scanned.len(), 1);
        assert!(scanned[0].location.is_live());
    }

    /// An unparseable file in a non-live directory must fail the scan naming
    /// the file — same strictness as `scan_tasks`, so an externally corrupted
    /// archive is surfaced rather than silently under-counted.
    #[test]
    fn scan_all_parse_failure_names_the_file() {
        let dir = scratch_backlog(&[]);
        std::fs::create_dir_all(dir.path().join(".backlog/archive/tasks")).expect("archive dir");
        std::fs::write(
            dir.path()
                .join(".backlog/archive/tasks/task-0042 - broken.md"),
            "not frontmatter",
        )
        .expect("write broken file");
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        let err = store.scan_all_tasks().expect_err("must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("task-0042 - broken.md"),
            "error must name the file, got: {rendered}"
        );
    }

    /// A `read_dir` failure other than `NotFound` must fail the scan naming
    /// the directory — here, a file squatting where `completed/` belongs —
    /// instead of silently skipping it and under-counting the tree.
    #[test]
    fn scan_all_read_failure_names_the_directory() {
        let dir = scratch_backlog(&[]);
        std::fs::write(dir.path().join(".backlog/completed"), "not a dir").expect("write file");
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        let err = store.scan_all_tasks().expect_err("must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("completed"),
            "error must name the unreadable directory, got: {rendered}"
        );
    }

    /// A duplicate id across directories is returned twice — the corpus
    /// really holds such pairs, and callers decide how to aggregate.
    #[test]
    fn scan_all_returns_duplicate_ids_across_dirs() {
        let dir = scratch_backlog(&[
            ("completed", "task-0059 - done.md"),
            ("archive/tasks", "task-0059 - archived.md"),
        ]);
        let store = Store::open(&dir.path().join(".backlog")).expect("open");
        let scanned = store.scan_all_tasks().expect("scan");
        assert_eq!(scanned.len(), 2);
        assert!(scanned.iter().all(|l| l.doc.frontmatter.id == "TASK-0059"));
    }
}
