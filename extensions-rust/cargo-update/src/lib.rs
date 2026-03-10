//! Cargo update extension: runs `cargo update --dry-run` and parses available dependency updates.
//!
//! This is a data-source-only extension (no commands). It provides parsed update
//! information that the about page consumes via the `--update` flag.

#[cfg(test)]
mod tests;

use cargo_ops_extension::{
    Context, DataField, DataProvider, DataProviderError, DataProviderSchema, ExtensionType,
};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::{Command, Output};

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
pub struct CargoUpdateResult {
    pub entries: Vec<UpdateEntry>,
    pub update_count: usize,
    pub add_count: usize,
    pub remove_count: usize,
}

/// Run `cargo update --dry-run` in the given working directory.
pub fn run_cargo_update_dry_run(working_dir: &Path) -> std::io::Result<Output> {
    Command::new("cargo")
        .args(["update", "--dry-run"])
        .current_dir(working_dir)
        .output()
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

        if let Some(entry) = parse_updating_line(clean) {
            entries.push(entry);
        } else if let Some(entry) = parse_adding_line(clean) {
            entries.push(entry);
        } else if let Some(entry) = parse_removing_line(clean) {
            entries.push(entry);
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

/// Parse: `Updating serde v1.0.0 -> v1.0.1`
fn parse_updating_line(line: &str) -> Option<UpdateEntry> {
    let rest = line.strip_prefix("Updating")?.trim();
    let parts: Vec<&str> = rest.splitn(4, ' ').collect();
    if parts.len() >= 4 && parts[2] == "->" {
        Some(UpdateEntry {
            action: UpdateAction::Update,
            name: parts[0].to_string(),
            from: Some(strip_v_prefix(parts[1]).to_string()),
            to: Some(strip_v_prefix(parts[3]).to_string()),
        })
    } else {
        None
    }
}

/// Parse: `Adding new-crate v0.1.0`
fn parse_adding_line(line: &str) -> Option<UpdateEntry> {
    let rest = line.strip_prefix("Adding")?.trim();
    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
    if parts.len() >= 2 {
        Some(UpdateEntry {
            action: UpdateAction::Add,
            name: parts[0].to_string(),
            from: None,
            to: Some(strip_v_prefix(parts[1]).to_string()),
        })
    } else {
        None
    }
}

/// Parse: `Removing old-crate v0.2.0`
fn parse_removing_line(line: &str) -> Option<UpdateEntry> {
    let rest = line.strip_prefix("Removing")?.trim();
    let parts: Vec<&str> = rest.splitn(2, ' ').collect();
    if parts.len() >= 2 {
        Some(UpdateEntry {
            action: UpdateAction::Remove,
            name: parts[0].to_string(),
            from: Some(strip_v_prefix(parts[1]).to_string()),
            to: None,
        })
    } else {
        None
    }
}

pub struct CargoUpdateExtension;

cargo_ops_extension::impl_extension! {
    CargoUpdateExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(CargoUpdateProvider));
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

        let result = parse_update_output(&output.stderr);
        serde_json::to_value(&result).map_err(DataProviderError::from)
    }

    fn schema(&self) -> DataProviderSchema {
        DataProviderSchema {
            description: "Available dependency updates from cargo update --dry-run",
            fields: vec![
                DataField {
                    name: "entries",
                    type_name: "Vec<UpdateEntry>",
                    description: "List of dependency update/add/remove entries",
                },
                DataField {
                    name: "update_count",
                    type_name: "usize",
                    description: "Number of updates available",
                },
                DataField {
                    name: "add_count",
                    type_name: "usize",
                    description: "Number of new dependencies to add",
                },
                DataField {
                    name: "remove_count",
                    type_name: "usize",
                    description: "Number of dependencies to remove",
                },
            ],
        }
    }
}
