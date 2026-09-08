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
/// first, and a same-directory `rename` — atomic on POSIX — swaps it in. A
/// crash mid-write leaves the previous document intact and at most one
/// leftover staging file, whose `.tmp` suffix keeps it invisible to task
/// scans; the next attempt truncates and replaces it, so a leftover can
/// never become a second, competing document.
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
    let staging = path.with_file_name(format!(".{}.tmp", name.to_string_lossy()));
    let mut handle =
        std::fs::File::create(&staging).with_context(|| format!("staging {}", path.display()))?;
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
