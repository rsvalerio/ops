//! backlog.md task-file layout for review requests: daily-sequence id
//! allocation, claim re-checks, and frontmatter rendering.
//!
//! The generic shapes — filename slugs, the YAML scalar codec, id-number
//! scanning across the backlog tree, zero-padded ids, dotted subtask ids —
//! live in `ops-backlog` (shared with `ops backlog task ...`); what stays
//! here is everything specific to the `review-request-<date>-<n>` scheme.

use std::io::Write;
use std::path::Path;

use ops_backlog::clock::UtcStamp;
use ops_backlog::model::{Body, FmValue, Frontmatter, TaskDoc};
use ops_backlog::store::{find_task_file, for_each_task_file, TaskFileName};

/// Main-task frontmatter labels, in order.
const MAIN_LABELS: &[&str] = &["code-review-request", "code-review", "qa"];
/// Subtask frontmatter labels, in order.
const SUBTASK_LABELS: &[&str] = &["code-review", "qa"];

/// Ordinal the backlog CLI assigns to a freshly created parent task.
const MAIN_ORDINAL: u32 = 1_000;
/// Ordinal base the backlog CLI assigns to subtasks (child i → 2000 + i).
const SUBTASK_ORDINAL_BASE: u32 = 2_000;

/// Id prefix and zero-padding width for every id and filename this module
/// writes (READ-11: one named pair, not four repeated literals).
///
/// These deliberately do not follow `BacklogConfig::task_prefix` /
/// `zero_padded_ids` per run: the `review-request-<date>-<n>` scheme
/// allocates from the same `task-<n>` numbering the backlog CLI already
/// owns in the target tree, so the ids and filenames written here must be
/// byte-identical to the ones `ops backlog task create` writes beside them.
/// Every existing task file was allocated under the values below — the
/// backlog.md defaults this repo's `backlog.config.yml` pins explicitly —
/// and a tree that reconfigured them would need this extension taught to
/// discover the width from the files on disk, not just from the config's
/// claim. Until then the coupling is pinned here, in one place.
const TASK_PREFIX: &str = "TASK";
const ID_WIDTH: usize = 4;

/// Ensure the backlog tree this writer targets actually exists. ERR-13: the
/// error names the missing directory so the operator knows what to create.
pub fn require_backlog_tasks_dir(workspace_root: &Path) -> anyhow::Result<()> {
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

/// The identifiers one allocation attempt claims, both read from a single
/// scan of the backlog tree.
pub struct NextIds {
    /// Next free main-task number; 1 for an empty backlog.
    pub number: u32,
    /// Next `review-request-<date>-<n>` sequence for the requested date; 1
    /// when this is the first request of the day.
    pub sequence: u32,
}

/// Allocate both ids from **one** walk of every task directory that exists
/// (see [`ops_backlog::store::TASK_DIRS`]): one more than the highest `task-<n>` id (dotted
/// subtask ids share their parent's number, so the integer part alone
/// determines allocation), and one more than the highest `<n>` in a
/// `review-request-<date>-<n>` slug for `date`.
///
/// DUP-1: the two allocators used to be the same scan-extract-max loop run
/// twice. Deriving both maxima from one listing makes them consistent with
/// each other by construction — no concurrent writer can land between them —
/// and cuts the per-attempt directory I/O in half.
pub fn next_ids(workspace_root: &Path, date: &str) -> NextIds {
    let prefix = format!("review-request-{date}-");
    let backlog_root = workspace_root.join(".backlog");
    let mut max_number = 0u32;
    let mut max_sequence = 0u32;
    for_each_task_file(&backlog_root, |_dir, file_name| {
        let Some(parsed) = TaskFileName::parse(file_name) else {
            return;
        };
        if let Some(number) = parsed.number {
            max_number = max_number.max(number);
        }
        if let Some(sequence) = review_request_sequence(parsed.slug, &prefix) {
            max_sequence = max_sequence.max(sequence);
        }
    });
    NextIds {
        number: max_number.saturating_add(1),
        sequence: max_sequence.saturating_add(1),
    }
}

/// The `<n>` of a `review-request-<date>-<n>` slug, where `prefix` is
/// `review-request-<date>-`. Anchored at both ends: the prefix must *start*
/// the slug and the digits must be all that follows it, so a task merely
/// mentioning a review request in its title (`task-1900 -
/// Fix-review-request-2026-08-27-3-flakiness.md`) contributes nothing to
/// that day's sequence.
fn review_request_sequence(slug: &str, prefix: &str) -> Option<u32> {
    let digits = slug.strip_prefix(prefix)?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<u32>().ok()
}

/// Identifiers one run has reserved by creating its main task file: the file
/// itself, its main-task number, and its `review-request-<date>-<n>` title.
/// FN-3: grouped rather than passed as four positional parameters.
pub struct MainTaskClaim<'a> {
    /// Name of the main task file this run created, inside `tasks`.
    pub(crate) file_name: &'a str,
    /// Main-task number the run allocated.
    pub number: u32,
    /// Main-task title the run allocated.
    pub title: &'a str,
}

/// A task file, other than the claimant's own, that already claims the
/// main-task number or the review-request title in `claim`. Returns the
/// offending filename, or `None` when the claim is uncontested.
///
/// Allocation reads the tree once, but that read is not atomic with the write
/// that follows it. A run that allocates between this run's scan and its
/// create can reach a plan sharing this run's number or its sequence while
/// landing on a *different* filename, which `create_new` cannot detect.
/// Re-reading the tree once the main task file exists closes that gap: the
/// file is a reservation every other run can see, so whichever run observes a
/// conflict stands down. At most one run can miss the conflict, because
/// whoever checks last necessarily sees both files.
pub fn conflicting_claim(workspace_root: &Path, claim: &MainTaskClaim<'_>) -> Option<String> {
    let backlog_root = workspace_root.join(".backlog");
    let own_slug = ops_backlog::model::slugify(claim.title);
    // PERF-3 / TASK-2131: early-exit traversal — the walk stops at the first
    // conflicting file instead of flag-checking every remaining entry.
    find_task_file(&backlog_root, |dir, file_name| {
        // The claimant's own reservation lives in `tasks`; an identically
        // named file in `completed` or an archive is somebody else's.
        if dir == "tasks" && file_name == claim.file_name {
            return None;
        }
        let parsed = TaskFileName::parse(file_name)?;
        // A dotted subtask counts too: it means another run owns the number.
        let claims_number = parsed.number == Some(claim.number);
        let claims_title = parsed.slug == own_slug;
        (claims_number || claims_title).then(|| file_name.to_string())
    })
}

/// Render one task markdown file (frontmatter only, no body sections) into
/// `w`. `subtask_of` is the parent's zero-padded id (`"TASK-1671"`) plus the
/// 1-based subtask position for a subtask, or `None` for the main task.
///
/// DUP-2: the bytes come from the shared `ops_backlog` task-document
/// renderer — the same encoder `ops backlog task create` uses — so the two
/// writers cannot drift. Everything specific to review requests (label
/// sets, ordinals, `parent_task_id`) enters as a [`Frontmatter`] field
/// here, not as a second frontmatter implementation.
///
/// PERF-13 / TASK-2117: the rendered document reaches `w` as one
/// `write_all` of the shared renderer's `String` — the one-write-per-
/// document property — and `w` must still be buffered (e.g. a `BufWriter`
/// around the `File`) at the call site for the surrounding task-writing
/// loop to keep that property across a whole request's files.
pub fn render_task_file<W: Write>(
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
            // Ordinals only order sibling rows, so pinning an absurd `index` at
            // `u32::MAX` (the sort-last slot) is the correct degraded value —
            // the same one the `try_from` fallback already picks.
            SUBTASK_ORDINAL_BASE.saturating_add(u32::try_from(index).unwrap_or(u32::MAX)),
        ),
    };
    let doc = TaskDoc {
        frontmatter: Frontmatter {
            id: id.to_string(),
            title: title.to_string(),
            status: "To Do".to_string(),
            assignees: Vec::new(),
            created_date: format!("{} {}", stamp.date, stamp.minutes),
            updated_date: None,
            labels: labels.iter().map(|label| (*label).to_string()).collect(),
            dependencies: Vec::new(),
            priority: Some("low".to_string()),
            modified_files: Vec::new(),
            ordinal: Some(ordinal.to_string()),
            extras: match subtask_of {
                None => Vec::new(),
                Some((parent_id, _)) => {
                    vec![(
                        "parent_task_id".to_string(),
                        FmValue::Scalar(parent_id.to_string()),
                    )]
                }
            },
        },
        // Plan tasks carry their content as subtasks; the file ends at the
        // closing `---` with no body sections.
        body: Body::default(),
    };
    w.write_all(doc.render().as_bytes())
}

/// Zero-padded task id string (`TASK-0042`) for a main-task number.
pub fn main_task_id(number: u32) -> String {
    ops_backlog::store::format_task_id(TASK_PREFIX, number, ID_WIDTH)
}

/// Filename for a main task: `task-0042 - <slug>.md`.
pub fn main_task_file_name(number: u32, title: &str) -> String {
    ops_backlog::store::main_task_file_name(number, ID_WIDTH, title)
}

/// Id for the subtask at 1-based `index` under main-task `number`:
/// `TASK-0042.03` — the shared main-task id plus the dotted position, so
/// the prefix and padding come from `format_task_id` with the pinned
/// `TASK_PREFIX`/`ID_WIDTH` above, not from an inline format spec.
pub fn subtask_id(number: u32, index: usize) -> String {
    format!(
        "{}.{index:02}",
        ops_backlog::store::format_task_id(TASK_PREFIX, number, ID_WIDTH)
    )
}

/// Filename for a subtask: `task-0042.03 - <slug>.md` — the subtask id in
/// the filename's lowercase form, so the number's padding is derived, not
/// re-implemented.
pub fn subtask_file_name(number: u32, index: usize, title: &str) -> String {
    format!(
        "{} - {}.md",
        subtask_id(number, index).to_ascii_lowercase(),
        ops_backlog::model::file_slug(title)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Date for the cases that only assert on the main-task number; no
    /// fixture below carries a review request for it.
    const ANY_DATE: &str = "2026-01-01";

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
    fn next_number_starts_at_one_for_empty_backlog() {
        let dir = scratch_backlog(&[]);
        assert_eq!(next_ids(dir.path(), ANY_DATE).number, 1);
    }

    #[test]
    fn next_number_ignores_dotted_subtask_fraction() {
        let dir = scratch_backlog(&[("tasks", "task-0007.09 - child.md")]);
        assert_eq!(next_ids(dir.path(), ANY_DATE).number, 8);
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
        assert_eq!(next_ids(dir.path(), ANY_DATE).number, 1671);
    }

    #[test]
    fn next_number_skips_non_task_files_and_bare_prefix() {
        let dir = scratch_backlog(&[
            ("tasks", "task-not-a-number.md"),
            ("tasks", "notes.md"),
            ("tasks", "task-0020 - real.md"),
        ]);
        // A slugless name still reserves its number: allocation must never
        // hand out an id some file already carries.
        assert_eq!(
            next_ids(dir.path(), ANY_DATE).number,
            21,
            "sanity: the fixture's highest slugged id is 20"
        );
        let dir = scratch_backlog(&[("tasks", "task-0030.md"), ("tasks", "task-0020 - real.md")]);
        assert_eq!(next_ids(dir.path(), ANY_DATE).number, 31);
    }

    #[test]
    fn daily_sequence_starts_at_one() {
        let dir = scratch_backlog(&[]);
        assert_eq!(next_ids(dir.path(), "2026-08-20").sequence, 1);
    }

    #[test]
    fn daily_sequence_increments_past_same_day_requests() {
        let dir = scratch_backlog(&[
            ("tasks", "task-1671 - review-request-2026-08-20-1.md"),
            ("tasks", "task-1671.01 - REVIEW-Run-skill.md"),
            ("completed", "task-1600 - review-request-2026-08-20-2.md"),
        ]);
        assert_eq!(next_ids(dir.path(), "2026-08-20").sequence, 3);
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
        assert_eq!(next_ids(dir.path(), "2026-08-20").sequence, 1);
    }

    #[test]
    fn daily_sequence_skips_non_numeric_suffix() {
        let dir = scratch_backlog(&[("tasks", "task-1600 - review-request-2026-08-20-notes.md")]);
        assert_eq!(next_ids(dir.path(), "2026-08-20").sequence, 1);
    }

    /// The claim helper the concurrent commit path relies on. The
    /// interleaving it guards against cannot be forced single-threaded, so
    /// the predicate itself is pinned here.
    #[test]
    fn conflicting_claim_detects_number_and_title_squatters() {
        const OWN: &str = "task-0001 - review-request-2026-08-20-1.md";

        let claim = |dir: &tempfile::TempDir| {
            conflicting_claim(
                dir.path(),
                &MainTaskClaim {
                    file_name: OWN,
                    number: 1,
                    title: "review-request-2026-08-20-1",
                },
            )
        };

        // Uncontested: only the claimant's own reservation is present.
        let dir = scratch_backlog(&[("tasks", OWN)]);
        assert_eq!(claim(&dir), None, "own file must not count against itself");

        // Unrelated tasks leave the claim alone.
        let dir = scratch_backlog(&[
            ("tasks", OWN),
            ("tasks", "task-0002 - review-request-2026-08-20-2.md"),
            ("tasks", "task-0003 - something-else.md"),
        ]);
        assert_eq!(
            claim(&dir),
            None,
            "distinct number and title must not clash"
        );

        // Same main number under a different title — the case `create_new`
        // cannot see, because the filenames differ.
        let dir = scratch_backlog(&[
            ("tasks", OWN),
            ("tasks", "task-0001 - review-request-2026-08-20-4.md"),
        ]);
        assert_eq!(
            claim(&dir).as_deref(),
            Some("task-0001 - review-request-2026-08-20-4.md"),
            "a second file claiming number 1 must be reported"
        );

        // A dotted subtask also claims its parent's number.
        let dir = scratch_backlog(&[("tasks", OWN), ("tasks", "task-0001.01 - REVIEW-other.md")]);
        assert_eq!(
            claim(&dir).as_deref(),
            Some("task-0001.01 - REVIEW-other.md"),
            "another run's subtask claims the number too"
        );

        // Same title under a different main number — the duplicate-sequence
        // case, likewise invisible to `create_new`.
        let dir = scratch_backlog(&[
            ("tasks", OWN),
            ("tasks", "task-0009 - review-request-2026-08-20-1.md"),
        ]);
        assert_eq!(
            claim(&dir).as_deref(),
            Some("task-0009 - review-request-2026-08-20-1.md"),
            "a duplicate review-request title must be reported"
        );

        // An identically named file outside `tasks` is somebody else's, not
        // the claimant's own reservation.
        let dir = scratch_backlog(&[("tasks", OWN), ("completed", OWN)]);
        assert_eq!(
            claim(&dir).as_deref(),
            Some(OWN),
            "the own-file exemption must not extend to completed/archive"
        );
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

    /// Byte-shape of a main-task file as the backlog CLI writes it (golden
    /// test against the shared renderer's output — the same encoder
    /// `backlog task create` uses, hence the `modified_files: []` key every
    /// CLI-created task carries).
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
            "modified_files: []\n",
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
            "parent_task_id: 'TASK-1671'\n",
            "modified_files: []\n",
            "priority: low\n",
            "ordinal: 2001\n",
            "---\n",
        );
        assert_eq!(String::from_utf8(buf).expect("utf8"), expected);
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

    /// READ-6: a task whose *title* embeds a review-request id must not
    /// inflate that day's sequence — the prefix has to start the slug and the
    /// digits have to be all that follows it.
    #[test]
    fn daily_sequence_ignores_a_review_request_id_embedded_in_a_title() {
        let dir = scratch_backlog(&[
            (
                "tasks",
                "task-1900 - Fix-review-request-2026-08-27-3-flakiness.md",
            ),
            ("tasks", "task-1901 - review-request-2026-08-27-1.md"),
        ]);
        assert_eq!(next_ids(dir.path(), "2026-08-27").sequence, 2);
    }

    /// Both ids come from the same listing, so one call answers what two
    /// separate walks used to.
    #[test]
    fn next_ids_reads_number_and_sequence_from_one_scan() {
        let dir = scratch_backlog(&[
            ("tasks", "task-0010 - review-request-2026-08-20-1.md"),
            ("completed", "task-0020 - review-request-2026-08-20-4.md"),
        ]);
        let ids = next_ids(dir.path(), "2026-08-20");
        assert_eq!(ids.number, 21);
        assert_eq!(ids.sequence, 5);
    }

    /// TEST-8 boundary: a backlog whose highest id is `u32::MAX` cannot
    /// advance, so allocation returns the same number rather than wrapping.
    #[test]
    fn next_number_saturates_at_u32_max() {
        let dir = scratch_backlog(&[("tasks", "task-4294967295 - highest.md")]);
        assert_eq!(next_ids(dir.path(), ANY_DATE).number, u32::MAX);
    }

    /// SEC-11 boundary: non-ASCII letters are outside the slug alphabet, and
    /// a wholly non-ASCII title must not produce `task-NNNN.MM - .md`.
    #[test]
    fn slugify_and_file_names_pin_non_ascii_titles() {
        assert_eq!(ops_backlog::model::slugify("naïve-crate"), "na-ve-crate");
        assert_eq!(ops_backlog::model::slugify("日本語"), "");
        assert_eq!(
            main_task_file_name(42, "日本語"),
            "task-0042 - untitled.md",
            "an empty slug must not yield `task-0042 - .md`"
        );
        assert_eq!(
            subtask_file_name(42, 1, "日本語"),
            "task-0042.01 - untitled.md"
        );
    }

    /// SEC-11: a title carrying a newline must still render as a single YAML
    /// document — a single-quoted scalar cannot encode one.
    #[test]
    fn control_characters_render_as_a_double_quoted_scalar() {
        let stamp = UtcStamp {
            date: "2026-08-20".to_string(),
            minutes: "19:02".to_string(),
        };
        let mut buf = Vec::new();
        render_task_file(&mut buf, "TASK-0001", "ops\ncore", &stamp, None).expect("render");
        let rendered = String::from_utf8(buf).expect("utf8");
        assert!(
            rendered.contains("title: \"ops\\ncore\"\n"),
            "the newline must be escaped inside the scalar, got: {rendered}"
        );
        assert_eq!(
            rendered.lines().filter(|line| *line == "---").count(),
            2,
            "the frontmatter must stay one document, got: {rendered}"
        );
    }
}
