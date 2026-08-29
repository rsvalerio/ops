//! `check-json` and `check-yaml` — generic-stack file validators modelled on
//! the same-named hooks from `pre-commit/pre-commit-hooks` (and mirrored by
//! `j178/prek`).
//!
//! Each checker walks the candidate file set (reusing the text-fixers'
//! discovery walk + git ls-files fast path), filters by extension, parses
//! each file, and reports a [`CheckerReport`] so the CLI can exit non-zero
//! when at least one file failed to parse. Files are never modified.

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
pub use report::{write_summary, CheckerReport, FailedFile, FailureKind};
pub use runner::{run_check_json, run_check_yaml};

use ops_extension::ExtensionType;

pub const NAME: &str = "config-checkers";
pub const DESCRIPTION: &str = "JSON and YAML parse-validators";
pub const SHORTNAME: &str = "config-checkers";

/// Default per-file size cap (16 MiB).
///
/// Files exceeding this are skipped and recorded in
/// [`CheckerReport::files_skipped`] rather than read and parsed. The cap is
/// enforced on the read itself (`Read::take`), not by a preceding
/// `metadata()` call, so it holds even if the file changes underneath.
///
/// It bounds *input* size only, which is the wrong unit for the two `DoS`
/// classes that do not need a large file. Those are bounded where they
/// happen, not here: [`json::MAX_NESTING_DEPTH`] caps nesting for both JSON
/// modes (40 KB of `[[[…]]]` otherwise overflows the stack under
/// `--allow-json5`), and [`yaml::MAX_EXPANDED_NODES`] caps alias expansion
/// (a 324-byte anchor bomb otherwise exhausts memory).
pub const DEFAULT_MAX_BYTES: u64 = 16 * 1024 * 1024;

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
        registry.insert(
            "check-json".into(),
            ops_core::config::CommandSpec::Exec(
                ops_core::config::ExecCommandSpec::new("ops", ["check-json"]),
            ),
        );
        registry.insert(
            "check-yaml".into(),
            ops_core::config::CommandSpec::Exec(
                ops_core::config::ExecCommandSpec::new("ops", ["check-yaml"]),
            ),
        );
    },
    register_data_providers: |_self, _registry| {},
    factory: CONFIG_CHECKERS_FACTORY = |_, _| {
        Some((NAME, Box::new(ConfigCheckersExtension)))
    },
}
