//! `trailing-whitespace` and `end-of-file-fixer` — generic-stack text fixers
//! modelled on the same-named hooks from `pre-commit/pre-commit-hooks`.
//!
//! Each fixer walks the candidate file set, skips anything that is not text,
//! rewrites files in place when changes are needed, and reports a
//! [`FixerReport`].
//!
//! # Exit-code contract
//!
//! The CLI exits non-zero when [`FixerReport::changed`] **or**
//! [`FixerReport::failed`] is true.
//!
//! The first half is the pre-commit contract: a fix means the commit must be
//! re-staged. The second half is what a gate requires of its own blind spots.
//! A hook driver reads exit zero as "the tree is clean", so a file the fixer
//! could not read or could not write back must not be allowed to pass as
//! clean — "could not check" is much closer to "failed" than to "passed".
//! Previously an unreadable file was skipped with no message, no counter and
//! no effect on the exit code, so a mode-600 file (routine in a container or
//! on a shared build agent) made the run report a clean tree it had never
//! looked at.
//!
//! Deliberate skips are the other half of that accounting and are *not*
//! failures: a file over [`DEFAULT_MAX_BYTES`], a non-regular file, a file
//! that vanished between discovery and read, and a file that is not text are
//! counted in [`FixerReport::files_skipped`]. `files_scanned + files_skipped +
//! files_failed` accounts for every path discovery returned. All four are in
//! the summary line; the skips are named file by file too, except the routine
//! "not text" one — see `runner::write_skip`.
//!
//! # Safety properties
//!
//! - Rewrites go through [`atomic::replace`]: temp file in the same directory,
//!   `fsync`, then `rename(2)`. There is no window in which the target is
//!   short. See that module for the hard-link trade this makes.
//! - Symlinks are never followed; see [`discovery`]'s symlink policy.
//! - Reads are bounded by [`FixerOptions::max_bytes`], enforced on the read
//!   itself rather than by a preceding `metadata()` call.
//! - A per-file failure does not abort the run, so the record of what was
//!   already rewritten survives; see `runner`.

// READ-10 (TASK-1966): this crate root carries no `cfg_attr(test, allow(..))`
// block. All four lints it used to relax suppress nothing here. The three cast
// lints have no callsite -- the crate contains no `as` cast, and the workspace
// denies `clippy::as_conversions` anyway -- while leaving them in place would
// have silently absorbed the first buffer-offset truncation anyone introduced
// into arithmetic that is all buffer offsets. `unwrap_used` is already relaxed
// for test code workspace-wide by `allow-unwrap-in-tests` in `clippy.toml`, so
// writing it as `expect` reports it as unfulfilled.

pub mod atomic;
pub mod binary;
pub mod discovery;
pub mod eof;
mod options;
mod report;
mod runner;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
pub mod trailing;

use ops_extension::ExtensionType;

pub use options::{FixerOptions, DEFAULT_MAX_BYTES};
pub use report::{write_summary, FailedFile, FailureKind, FixerReport, SkipReason};
pub use runner::{run_end_of_file_fixer, run_trailing_whitespace};

pub const NAME: &str = "text-fixers";
pub const DESCRIPTION: &str = "Trailing whitespace and end-of-file fixers";
pub const SHORTNAME: &str = "text-fixers";

pub struct TextFixersExtension;

ops_extension::impl_extension! {
    TextFixersExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    command_names: &["trailing-whitespace", "end-of-file-fixer"],
    data_provider_name: None,
    register_commands: |_self, registry| {
        registry.insert(
            "trailing-whitespace".into(),
            ops_core::config::CommandSpec::Exec(
                ops_core::config::ExecCommandSpec::new("ops", ["trailing-whitespace"]),
            ),
        );
        registry.insert(
            "end-of-file-fixer".into(),
            ops_core::config::CommandSpec::Exec(
                ops_core::config::ExecCommandSpec::new("ops", ["end-of-file-fixer"]),
            ),
        );
    },
    register_data_providers: |_self, _registry| {},
    factory: TEXT_FIXERS_FACTORY = |_, _| {
        Some((NAME, Box::new(TextFixersExtension)))
    },
}
