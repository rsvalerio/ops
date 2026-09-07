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
