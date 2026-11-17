//! Command handlers: plain option structs (no clap), writer-injectable
//! cores following the CLI crate's `_to` idom.
//!
//! Errors are `anyhow` with the file path attached; the CLI prints
//! them as `ops: error: …`.

pub mod about;
pub mod cleanup;
pub mod commit;
pub mod create;
pub mod edit;
pub mod git;
pub mod list;
pub mod search;
pub mod view;
pub mod wave;
pub mod wave_claim;
pub mod wave_create;

pub use about::run_about_backlog;
pub use cleanup::{run_cleanup, CleanupOptions};
pub use commit::{run_commit, CommitOptions};
pub use create::{create_task, run_create, CreateOptions, Created};
pub use edit::{run_edit, EditOptions};
pub use list::{run_list, ListOptions};
pub use search::{run_search, SearchOptions};
pub use view::{run_view, ViewOptions};
pub use wave::{
    run_wave_list, run_wave_members, run_wave_migrate, run_wave_overlap, WaveListOptions,
    WaveMembersOptions, WaveMigrateOptions, WaveOverlapOptions, DEFAULT_WAVE_MARKER,
};
pub use wave_claim::{run_wave_claim, run_wave_park, WaveClaimOptions, WaveParkOptions};
pub use wave_create::{run_wave_create, WaveCreateOptions, DEFAULT_WAVE_STATUS};

use std::io::Write;

/// Output mode for `task view` / `task list`: exactly one renderer.
///
/// The two-bool (`plain`, `json`) form this replaces could represent
/// "plain and json at once", a state that is not a valid output mode;
/// the enum makes it unrepresentable, and the CLI rejects `--plain --json`
/// at parse time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputFormat {
    /// Human-readable rendering; also the mode when no flag is passed.
    #[default]
    Plain,
    /// The machine-readable JSON envelope.
    Json,
}

impl OutputFormat {
    /// The format the CLI's flag pair selects: `--json` picks
    /// [`OutputFormat::Json`]; no flag — and `--plain`, which renders
    /// identically to the default — picks [`OutputFormat::Plain`]. Owning
    /// the mapping here keeps the `--plain`≡default equivalence contract
    /// out of per-action CLI handlers.
    #[must_use]
    pub const fn from_json_flag(json: bool) -> Self {
        if json {
            Self::Json
        } else {
            Self::Plain
        }
    }
}

/// The host wall clock as a UTC instant, read once at the command edge and
/// threaded down so the handlers stay testable against a fixed clock.
///
/// # Errors
///
/// The host clock reads outside the range `jiff` can represent (before year
/// -9999 or after 9999) — reported instead of panicking.
pub(crate) fn wall_clock() -> anyhow::Result<jiff::Timestamp> {
    use anyhow::Context as _;
    jiff::Timestamp::try_from(std::time::SystemTime::now())
        .context("system clock reads outside the representable date range")
}

/// `days` UTC calendar days as an elapsed duration. In UTC a calendar day is
/// exactly 24 hours (no DST), so this equals calendar-day arithmetic; the
/// helper exists so callers never spell the `* 24` themselves.
pub(crate) const fn utc_days(days: i64) -> jiff::SignedDuration {
    jiff::SignedDuration::from_hours(days.saturating_mul(24))
}

/// Ask `<prompt> [y/N] ` and read one answer line. `y`/`yes`
/// (case-insensitive) proceeds; empty input — including EOF on a closed
/// stdin — and anything else cancels. No is the default, matching the
/// backlog CLI's confirm prompt.
///
/// # Errors
///
/// Writing the prompt or reading the answer failed.
pub(crate) fn confirm<W: Write>(
    prompt: &str,
    input: &mut dyn std::io::BufRead,
    out: &mut W,
) -> anyhow::Result<bool> {
    use anyhow::Context as _;
    write!(out, "{prompt} [y/N] ").context("printing the confirmation prompt")?;
    out.flush().context("flushing the confirmation prompt")?;
    let mut answer = String::new();
    input.read_line(&mut answer).context("reading the confirmation answer")?;
    Ok(matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}

/// Write `contents` to `path` so the destination is never observable
/// half-written: the bytes land in a staging file next to the destination
/// first, and a same-directory `rename` — atomic on POSIX — swaps it in.
///
/// The staging name carries this process's id and is created exclusively
/// (`O_EXCL` via `create_new`), so it can neither follow a symlink
/// pre-planted at a predictable path nor be truncated by a concurrent
/// process's staging attempt: both surface as an error naming the staging
/// path instead of silently writing through the wrong file. A crash
/// mid-write leaves the previous document intact and at most one leftover
/// staging file, whose dot-prefixed name keeps it invisible to task scans.
/// The CLI's write paths are sequential, so two in-process writers staging
/// the same destination do not occur; if one ever does, the exclusive
/// creation fails loudly rather than letting the writers interleave.
///
/// # Errors
///
/// The staging file cannot be created or written, or the rename over the
/// destination fails — each error names the destination path.
pub(crate) fn atomic_write(path: &std::path::Path, contents: &str) -> anyhow::Result<()> {
    use anyhow::Context as _;
    let staging = stage_contents(path, contents)?;
    if let Err(err) = std::fs::rename(&staging, path) {
        std::fs::remove_file(&staging).ok();
        return Err(err).with_context(|| format!("replacing {}", path.display()));
    }
    Ok(())
}

/// Write `contents` to a fully synced staging file next to `path` and
/// return the staging path, ready to be published under the destination
/// name. The staging name is `.{name}.{pid}.tmp` — truncated and hashed
/// when that would overflow `NAME_MAX`, see [`staging_name`] — created
/// exclusively; a failed write removes it again.
///
/// # Errors
///
/// `path` has no file name, or the staging file cannot be created, written
/// or synced — each error names the destination path.
fn stage_contents(path: &std::path::Path, contents: &str) -> anyhow::Result<std::path::PathBuf> {
    use anyhow::Context as _;
    use std::io::Write as _;

    let Some(name) = path.file_name() else {
        anyhow::bail!("{} has no file name to stage a write under", path.display());
    };
    let staging = path.with_file_name(staging_name(&name.to_string_lossy(), std::process::id()));
    let mut handle = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .with_context(|| format!("staging {}", path.display()))?;
    if let Err(err) = handle.write_all(contents.as_bytes()).and_then(|()| handle.sync_all()) {
        std::fs::remove_file(&staging).ok();
        return Err(err).with_context(|| format!("staging {}", path.display()));
    }
    Ok(staging)
}

/// `NAME_MAX` on every filesystem a backlog tree plausibly lives on (ext4,
/// xfs, btrfs, tmpfs, APFS): a longer staging name fails to create with
/// `ENAMETOOLONG`, which is the bug this bound exists to avoid. Names on
/// exotic filesystems with a smaller limit still fail loudly at `create`,
/// naming the destination.
const NAME_MAX: usize = 255;

/// Build the staging file name for a destination `name`:
/// `.{name}.{pid}.tmp`. When that would not fit in [`NAME_MAX`], the name
/// is truncated on a char boundary and a hash of the full name is
/// appended, so two long names that share a prefix still stage under
/// distinct files while the staging name always fits.
fn staging_name(name: &str, pid: u32) -> String {
    let plain = format!(".{name}.{pid}.tmp");
    if plain.len() <= NAME_MAX {
        return plain;
    }
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    name.hash(&mut hasher);
    let digest = hasher.finish();
    let tail = format!(".{digest:016x}.{pid}.tmp");
    // The budget leaves room for the tail and the leading dot; truncation
    // stays on a char boundary so `to_string_lossy` output is never split
    // mid-character.
    let budget = NAME_MAX.saturating_sub(tail.len()).saturating_sub(1);
    let mut end = budget.min(name.len());
    while end > 0 && !name.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }
    let prefix = name.get(..end).unwrap_or_default();
    format!(".{prefix}{tail}")
}

/// [`atomic_write`] for a destination that must not already exist — the
/// config-creation path (`backlog.config.yml`).
///
/// `rename` replaces an existing destination, so a check-then-write
/// sequence races a concurrent creator and silently clobbers it. The
/// committed name is therefore claimed with a hard link instead:
/// `link(2)` fails with `EEXIST` when the name is taken — no window in
/// between — and only then is the staging name dropped. The staging file
/// is fully written and synced before the claim, so the destination is
/// either absent or complete, never half-written; a crash before the
/// staging name is removed leaves one recoverable copy (the same
/// post-state as `move_to_completed` in `cleanup`).
///
/// # Errors
///
/// The destination already exists (the error names it), or staging failed
/// — as [`atomic_write`].
pub(crate) fn atomic_write_noclobber(path: &std::path::Path, contents: &str) -> anyhow::Result<()> {
    use anyhow::Context as _;
    let staging = stage_contents(path, contents)?;
    if let Err(err) = std::fs::hard_link(&staging, path) {
        std::fs::remove_file(&staging).ok();
        if err.kind() == std::io::ErrorKind::AlreadyExists {
            anyhow::bail!("{} already exists; refusing to overwrite it", path.display());
        }
        return Err(err).with_context(|| format!("claiming {}", path.display()));
    }
    // The link holds the content; the staging name is now redundant. Its
    // removal is best-effort — a leftover keeps the dot-prefixed staging
    // name, invisible to task scans.
    std::fs::remove_file(&staging).ok();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_names_keep_the_plain_staging_scheme() {
        assert_eq!(
            staging_name("task-0001 - original.md", 1234),
            ".task-0001 - original.md.1234.tmp"
        );
    }

    /// A destination at the `NAME_MAX` limit stages under a name that still
    /// fits, and two long names sharing a prefix stay distinct (the hash).
    #[test]
    fn long_names_truncate_to_fit_and_stay_distinct() {
        let base = "task-0002 - ";
        let filler = |n: usize| "x".repeat(n);
        let a = format!("{base}{}a.md", filler(239));
        let b = format!("{base}{}b.md", filler(239));
        let staged = staging_name(&a, 1234);
        assert_eq!(a.len(), NAME_MAX, "destination itself sits at NAME_MAX");
        assert!(staged.len() <= NAME_MAX, "staging name must fit: {} bytes", staged.len());
        assert!(staged.starts_with(&format!(".{base}")), "prefix survives for diagnostics");
        assert_ne!(staged, staging_name(&b, 1234), "shared prefix must not collide");
    }

    /// Truncation never splits a multi-byte character.
    #[test]
    fn truncation_respects_char_boundaries() {
        // 'é' is two bytes; 130 of them overflow NAME_MAX with the tail,
        // and the cut lands mid-character for most budgets.
        let name = "é".repeat(130);
        let staged = staging_name(&name, 1234);
        assert!(staged.len() <= NAME_MAX);
        assert!(staged.is_char_boundary(staged.len()));
    }
}
