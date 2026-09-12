//! `check-json` and `check-yaml` — generic-stack file validators modelled on
//! the same-named hooks from `pre-commit/pre-commit-hooks` (and mirrored by
//! `j178/prek`).
//!
//! Each checker walks the candidate file set (reusing the text-fixers'
//! discovery walk + git ls-files fast path), filters by extension, parses
//! each file, and reports a [`CheckerReport`] so the CLI can exit non-zero
//! when at least one file failed to parse. Files are never modified.
//!
//! # Dependency note (TASK-2162)
//!
//! The bounded-read pipeline and the failure vocabulary this crate shares
//! with the fixers live in [`ops_core::bounded_read`] — not here, not in the
//! fixers — so the generic file-walking concerns create no
//! extension-to-extension edge. The one edge that remains is deliberate:
//! `ops_text_fixers::discovery`. Discovery encodes the *rewriting* tools'
//! policy (the symlink-never policy, the gitignore rationale tied to the
//! fixers' exit-code contract), this crate is modelled on the same
//! pre-commit hooks and wants byte-identical candidate semantics, and moving
//! it into `ops-core` would drag the `ignore` crate into the core for one
//! consumer. If a third extension ever needs it, extraction to its own crate
//! is the move; two consumers do not justify it.

// `src/tests.rs` relies on `unwrap()`; the crate performs no numeric casts,
// in tests or otherwise, so no cast lint is suppressed here.
#![cfg_attr(test, allow(clippy::unwrap_used))]
// API-14 / TASK-2138: undocumented public items are a warning, not silence.
#![warn(missing_docs)]

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
pub use report::{write_summary, CheckerReport, FailedFile, FailureKind};
pub use runner::{run_check_json, run_check_yaml};

use ops_extension::ExtensionType;

/// Extension identifier used to register this crate in the engine's
/// extension registry.
pub const NAME: &str = "config-checkers";
/// One-line description shown by `ops about` for this extension.
pub const DESCRIPTION: &str = "JSON and YAML parse-validators";
/// CLI-facing short name — the subcommand the user types
/// (`ops check-json`, `ops check-yaml`).
pub const SHORTNAME: &str = "config-checkers";

/// Default per-file size cap (16 MiB).
///
/// Files exceeding this are skipped and recorded in
/// [`CheckerReport::files_skipped`] rather than read and parsed. The cap is
/// enforced on the read itself (`Read::take`), not by a preceding
/// `metadata()` call, so it holds even if the file changes underneath.
///
/// DUP-2 / TASK-2162: defined once in [`ops_core::bounded_read`] and
/// re-exported, so this crate and the text fixers share one value and it
/// cannot drift between them.
///
/// It bounds *input* size only, which is the wrong unit for the two `DoS`
/// classes that do not need a large file. Those are bounded where they
/// happen, not here: [`json::MAX_NESTING_DEPTH`] caps nesting for both JSON
/// modes (40 KB of `[[[…]]]` otherwise overflows the stack under
/// `--allow-json5`), and [`yaml::MAX_EXPANDED_NODES`] caps alias expansion
/// (a 324-byte anchor bomb otherwise exhausts memory).
pub use ops_core::bounded_read::DEFAULT_MAX_BYTES;

/// Command extension registering the `check-json` and `check-yaml`
/// subcommands.
///
/// Both are registered as `Exec` specs resolving the current `ops` binary
/// (SEC-13 / TASK-2122), so they run through the runner rather than as
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
        // SEC-13 / TASK-2122: a bare "ops" resolves through the invoking
        // environment's PATH, so a shim earlier on PATH silently becomes the
        // validator. Spawn the absolute current_exe()-resolved binary via the
        // same shared helper the runner's builtin store uses for these very
        // command ids, and render as `ops check-json` / `ops check-yaml`.
        let ops_bin = ops_core::config::current_ops_program();
        for subcommand in ["check-json", "check-yaml"] {
            let mut spec = ops_core::config::ExecCommandSpec::new(
                ops_bin.clone(),
                [subcommand.to_string()],
            );
            spec.display_program = Some("ops".to_string());
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
