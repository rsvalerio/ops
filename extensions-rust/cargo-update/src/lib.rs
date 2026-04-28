//! Cargo update extension: runs `cargo update --dry-run` and parses available dependency updates.
//!
//! This is a data-source-only extension (no commands). It provides parsed update
//! information that the about page consumes via the `--update` flag.

#[cfg(test)]
mod tests;

use ops_core::subprocess::{run_cargo, RunError};
use ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Output;
use std::time::Duration;

pub const NAME: &str = "cargo-update";
pub const DESCRIPTION: &str = "Cargo update dry-run: available dependency updates";
pub const SHORTNAME: &str = "update";
pub const DATA_PROVIDER_NAME: &str = "cargo_update";

/// The action type for a dependency update entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UpdateAction {
    Update,
    Add,
    Remove,
}

/// A single dependency update entry parsed from `cargo update --dry-run` output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateEntry {
    pub action: UpdateAction,
    pub name: String,
    /// Version being updated from (None for Add actions).
    pub from: Option<String>,
    /// Version being updated to (None for Remove actions).
    pub to: Option<String>,
}

/// Result of parsing `cargo update --dry-run` output.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[must_use = "CargoUpdateResult carries the parsed update entries and counts — silently dropping it makes the cargo update --dry-run invocation observe nothing"]
pub struct CargoUpdateResult {
    pub entries: Vec<UpdateEntry>,
    pub update_count: usize,
    pub add_count: usize,
    pub remove_count: usize,
}

/// Default timeout for `cargo update --dry-run`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`.
pub const CARGO_UPDATE_TIMEOUT: Duration = Duration::from_secs(120);

/// Run `cargo update --dry-run` in the given working directory.
///
/// # Errors
///
/// Returns [`RunError::Io`] if the subprocess fails to spawn and
/// [`RunError::Timeout`] if it runs longer than [`CARGO_UPDATE_TIMEOUT`] (or
/// the `OPS_SUBPROCESS_TIMEOUT_SECS` override).
pub fn run_cargo_update_dry_run(working_dir: &Path) -> Result<Output, RunError> {
    run_cargo(
        &["update", "--dry-run"],
        working_dir,
        CARGO_UPDATE_TIMEOUT,
        "cargo update --dry-run",
    )
}

/// Strip leading `v` prefix from a version string.
fn strip_v_prefix(version: &str) -> &str {
    version.strip_prefix('v').unwrap_or(version)
}

/// Parse the stderr output of `cargo update --dry-run` into structured data.
///
/// Handles lines like:
/// - `Updating serde v1.0.0 -> v1.0.1`
/// - `Adding new-crate v0.1.0`
/// - `Removing old-crate v0.2.0`
///
/// Skips noise lines: `Updating crates.io index`, `Locking ...`, `warning:`, `note:`.
pub fn parse_update_output(stderr: &[u8]) -> CargoUpdateResult {
    let text = String::from_utf8_lossy(stderr);
    let mut entries = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();

        // Strip ANSI escape codes for robust parsing
        let clean = strip_ansi(trimmed);
        let clean = clean.trim();

        // Skip noise lines
        if clean.is_empty()
            || clean.starts_with("Locking")
            || clean.starts_with("warning:")
            || clean.starts_with("note:")
        {
            continue;
        }

        // "Updating crates.io index" — skip index updates
        if clean.starts_with("Updating") && clean.contains("index") {
            continue;
        }

        if let Some(entry) = parse_action_line(clean) {
            entries.push(entry);
        } else if starts_with_known_verb(clean) {
            // TASK-0472: a line that begins with a known verb but did not
            // parse is highly likely to indicate cargo-update format drift.
            // Promote to warn so the count regression is observable at the
            // default log level — debug would silently disappear.
            tracing::warn!(
                line = %clean,
                "skipping cargo-update line that begins with a known verb but did not parse — possible format drift"
            );
        }
    }

    let update_count = entries
        .iter()
        .filter(|e| e.action == UpdateAction::Update)
        .count();
    let add_count = entries
        .iter()
        .filter(|e| e.action == UpdateAction::Add)
        .count();
    let remove_count = entries
        .iter()
        .filter(|e| e.action == UpdateAction::Remove)
        .count();

    CargoUpdateResult {
        entries,
        update_count,
        add_count,
        remove_count,
    }
}

/// Strip ANSI escape sequences from a string.
fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            // Skip until 'm' (SGR terminator)
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(ch);
        }
    }
    result
}

/// Whether the version after `name` represents the source (from) or target (to) version.
#[derive(Clone, Copy)]
enum VersionRole {
    From,
    To,
}

/// Table-driven dispatch for `Updating` / `Adding` / `Removing` lines.
///
/// Each entry maps a leading verb to its `UpdateAction` and the role of the
/// single version that follows the name (for `Updating`, both versions are
/// captured separately via the `->` arrow form).
const ACTION_PREFIXES: &[(&str, UpdateAction, VersionRole)] = &[
    ("Updating", UpdateAction::Update, VersionRole::From),
    ("Adding", UpdateAction::Add, VersionRole::To),
    ("Removing", UpdateAction::Remove, VersionRole::From),
];

/// True when `line` starts with one of our recognised verb prefixes — used
/// solely to keep the tracing diagnostic narrow: lines that don't begin with
/// any known verb are noise (warnings, blank, etc.) and don't deserve a
/// "skipping cargo-update line" log.
fn starts_with_known_verb(line: &str) -> bool {
    ACTION_PREFIXES
        .iter()
        .any(|(prefix, _, _)| line.starts_with(prefix))
}

/// Parse one of:
/// - `Updating serde v1.0.0 -> v1.0.1`
/// - `Adding new-crate v0.1.0`
/// - `Removing old-crate v0.2.0`
fn parse_action_line(line: &str) -> Option<UpdateEntry> {
    for (prefix, action, role) in ACTION_PREFIXES {
        let Some(rest) = line.strip_prefix(prefix) else {
            continue;
        };
        let rest = rest.trim();

        // TASK-0476: iterator-based destructuring avoids the per-line
        // `Vec<&str>` allocation that `splitn(...).collect()` introduces on
        // a hot path (must_use provider runs in CI metadata pipelines).
        if matches!(action, UpdateAction::Update) {
            let mut it = rest.splitn(4, ' ');
            let name = it.next()?;
            let from = it.next()?;
            let arrow = it.next()?;
            let to = it.next()?;
            if arrow != "->" {
                return None;
            }
            return Some(UpdateEntry {
                action: action.clone(),
                name: name.to_string(),
                from: Some(strip_v_prefix(from).to_string()),
                to: Some(strip_v_prefix(to).to_string()),
            });
        }

        let (name, version_raw) = rest.split_once(' ')?;
        let version = Some(strip_v_prefix(version_raw).to_string());
        let (from, to) = match role {
            VersionRole::From => (version, None),
            VersionRole::To => (None, version),
        };
        return Some(UpdateEntry {
            action: action.clone(),
            name: name.to_string(),
            from,
            to,
        });
    }
    None
}

pub struct CargoUpdateExtension;

ops_extension::impl_extension! {
    CargoUpdateExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Rust),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(CargoUpdateProvider));
    },
    factory: CARGO_UPDATE_FACTORY = |_, _| {
        Some((NAME, Box::new(CargoUpdateExtension)))
    },
}

/// Data provider that runs `cargo update --dry-run` and returns parsed results.
pub struct CargoUpdateProvider;

impl DataProvider for CargoUpdateProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let output = run_cargo_update_dry_run(&ctx.working_directory).map_err(|e| {
            DataProviderError::from(anyhow::anyhow!("cargo update --dry-run failed: {}", e))
        })?;

        // TASK-0502: a successful spawn with a non-zero exit (e.g. lockfile
        // contention, network error, malformed Cargo.toml) leaves stderr
        // *not* shaped like the dry-run report. Parsing it would silently
        // produce an empty `CargoUpdateResult` — i.e. "no updates available"
        // for a failed invocation. Surface the error like sibling providers
        // (test-coverage, metadata, deps) instead.
        if !output.status.success() {
            let stderr_tail: String = String::from_utf8_lossy(&output.stderr)
                .lines()
                .rev()
                .take(10)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            return Err(DataProviderError::from(anyhow::anyhow!(
                "cargo update --dry-run exited with status {}: {}",
                output.status,
                stderr_tail
            )));
        }

        let result = parse_update_output(&output.stderr);
        serde_json::to_value(&result).map_err(DataProviderError::from)
    }

    fn schema(&self) -> DataProviderSchema {
        DataProviderSchema::new(
            "Available dependency updates from cargo update --dry-run",
            vec![
                DataField::new(
                    "entries",
                    "Vec<UpdateEntry>",
                    "List of dependency update/add/remove entries",
                ),
                DataField::new("update_count", "usize", "Number of updates available"),
                DataField::new("add_count", "usize", "Number of new dependencies to add"),
                DataField::new("remove_count", "usize", "Number of dependencies to remove"),
            ],
        )
    }
}
