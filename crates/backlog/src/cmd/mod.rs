//! Command handlers: plain option structs (no clap), writer-injectable
//! cores following the CLI crate's `_to` idom.
//!
//! Errors are `anyhow` with the file path attached (ERR-13); the CLI prints
//! them as `ops: error: …`.

pub mod about;
pub mod cleanup;
pub mod create;
pub mod edit;
pub mod list;
pub mod search;
pub mod view;
pub mod wave;

pub use about::run_about_backlog;
pub use cleanup::{run_cleanup, CleanupOptions};
pub use create::{run_create, CreateOptions};
pub use edit::{run_edit, EditOptions};
pub use list::{run_list, ListOptions};
pub use search::{run_search, SearchOptions};
pub use view::{run_view, ViewOptions};
pub use wave::{
    run_wave_list, run_wave_members, run_wave_migrate, WaveListOptions, WaveMembersOptions,
    WaveMigrateOptions, DEFAULT_WAVE_MARKER,
};

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
    input
        .read_line(&mut answer)
        .context("reading the confirmation answer")?;
    Ok(matches!(
        answer.trim().to_ascii_lowercase().as_str(),
        "y" | "yes"
    ))
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
    use std::io::Write as _;

    let Some(name) = path.file_name() else {
        anyhow::bail!("{} has no file name to stage a write under", path.display());
    };
    let staging = path.with_file_name(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let mut handle = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .with_context(|| format!("staging {}", path.display()))?;
    if let Err(err) = handle
        .write_all(contents.as_bytes())
        .and_then(|()| handle.sync_all())
    {
        std::fs::remove_file(&staging).ok();
        return Err(err).with_context(|| format!("staging {}", path.display()));
    }
    drop(handle);
    if let Err(err) = std::fs::rename(&staging, path) {
        std::fs::remove_file(&staging).ok();
        return Err(err).with_context(|| format!("replacing {}", path.display()));
    }
    Ok(())
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
    use std::io::Write as _;

    let Some(name) = path.file_name() else {
        anyhow::bail!("{} has no file name to stage a write under", path.display());
    };
    let staging = path.with_file_name(format!(
        ".{}.{}.tmp",
        name.to_string_lossy(),
        std::process::id()
    ));
    let mut handle = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&staging)
        .with_context(|| format!("staging {}", path.display()))?;
    if let Err(err) = handle
        .write_all(contents.as_bytes())
        .and_then(|()| handle.sync_all())
    {
        std::fs::remove_file(&staging).ok();
        return Err(err).with_context(|| format!("staging {}", path.display()));
    }
    drop(handle);
    if let Err(err) = std::fs::hard_link(&staging, path) {
        std::fs::remove_file(&staging).ok();
        if err.kind() == std::io::ErrorKind::AlreadyExists {
            anyhow::bail!(
                "{} already exists; refusing to overwrite it",
                path.display()
            );
        }
        return Err(err).with_context(|| format!("claiming {}", path.display()));
    }
    // The link holds the content; the staging name is now redundant. Its
    // removal is best-effort — a leftover keeps the dot-prefixed staging
    // name, invisible to task scans.
    std::fs::remove_file(&staging).ok();
    Ok(())
}
