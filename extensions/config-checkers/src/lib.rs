//! `check-json` and `check-yaml` — generic-stack file validators modelled on
//! the same-named hooks from `pre-commit/pre-commit-hooks` (and mirrored by
//! `j178/prek`).
//!
//! Each checker walks the candidate file set (reusing the text-fixers'
//! discovery walk + git ls-files fast path), filters by extension, parses
//! each file, and reports a [`CheckerReport`] so the CLI can exit non-zero
//! when at least one file failed to parse. Files are never modified.
//!
//! # Shared code
//!
//! Candidate discovery is [`ops_text_fixers::discovery`], shared with the
//! text fixers so both see the same candidate set; the bounded read and the
//! failure vocabulary are [`ops_core::bounded_read`].

// `src/tests.rs` relies on `unwrap()`; the crate performs no numeric casts,
// in tests or otherwise, so no cast lint is suppressed here.
#![cfg_attr(test, allow(clippy::unwrap_used))]

mod error;
pub mod json;
mod options;
mod report;
mod runner;
#[cfg(test)]
mod tests;
pub mod yaml;

pub use error::{CheckError, LimitExceeded};
pub use options::CheckerOptions;
pub use report::{CheckerReport, FailedFile, FailureKind, write_summary};
pub use runner::{run_check_json, run_check_yaml};

use ops_extension::ExtensionType;

/// Extension identifier used to register this crate in the engine's
/// extension registry.
pub const NAME: &str = "config-checkers";
/// One-line description shown by `ops about` for this extension.
pub const DESCRIPTION: &str = "JSON and YAML parse-validators";
/// Short identifier shown in the `Shortname` column of the extension
/// listing; the same as [`NAME`].
///
/// It is not a subcommand: the commands this extension registers are
/// `check-json` and `check-yaml`.
pub const SHORTNAME: &str = "config-checkers";

/// Default per-file size cap (16 MiB).
///
/// Files exceeding this are skipped and recorded in
/// [`CheckerReport::files_skipped`] rather than read and parsed. The cap is
/// enforced on the read itself (`Read::take`), not by a preceding
/// `metadata()` call, so it holds even if the file changes underneath.
///
/// Defined once in [`ops_core::bounded_read`] and re-exported, so this crate
/// and the text fixers share one value and it cannot drift between them.
///
/// It bounds *input* size only, which is the wrong unit for the two `DoS`
/// classes that do not need a large file. Those are bounded where they
/// happen, not here: [`MAX_NESTING_DEPTH`] caps nesting for both checkers
/// (40 KB of `[[[…]]]` otherwise overflows the stack under `--allow-json5`),
/// and [`yaml::MAX_EXPANDED_NODES`] caps alias expansion (a 324-byte anchor
/// bomb otherwise exhausts memory).
pub use ops_core::bounded_read::DEFAULT_MAX_BYTES;

/// Maximum bracket, brace or collection nesting accepted by both checkers —
/// either JSON mode and YAML alike.
///
/// Matches `serde_json`'s own `RECURSION_LIMIT`, so the strict and lenient
/// JSON branches agree on the same input. One definition serves both
/// checkers, so raising it for one raises it for both.
pub const MAX_NESTING_DEPTH: u64 = 128;

/// Command extension registering the `check-json` and `check-yaml`
/// subcommands.
///
/// Both are registered as `Exec` specs resolving the current `ops` binary,
/// so they run through the runner rather than as
/// extension-local handlers; no data providers are registered.
pub struct ConfigCheckersExtension;

ops_extension::impl_extension! {
    ConfigCheckersExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::COMMAND,
    command_names: &["check-json", "check-yaml"],
    data_provider_name: None,
    register_commands: |_self, registry| {
        // A bare "ops" resolves through the invoking
        // environment's PATH, so a shim earlier on PATH silently becomes the
        // validator. `ops_subcommand` spawns the absolute current_exe()-resolved
        // binary, the same constructor the runner's builtin store uses for these
        // very command ids, and renders as `ops check-json` / `ops check-yaml`.
        // The checkers only read files, so they opt out of exclusivity.
        for subcommand in ["check-json", "check-yaml"] {
            let mut spec = ops_core::config::ExecCommandSpec::ops_subcommand(subcommand);
            spec.exclusive = false;
            registry.insert(
                subcommand.into(),
                ops_core::config::CommandSpec::Exec(spec),
            );
        }
    },
    register_data_providers: |_self, _registry| {},
    factory: CONFIG_CHECKERS_FACTORY = |_, _| {
        Some((NAME, Box::new(ConfigCheckersExtension)))
    },
}
