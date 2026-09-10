//! Data types for the dependency health report.
//!
//! Re-exported from the crate root, so `ops_deps::UpgradeEntry` and friends
//! keep their published paths.

use serde::{Deserialize, Serialize};

/// A single available upgrade entry from `cargo upgrade --dry-run`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UpgradeEntry {
    /// Crate name, verbatim from cargo's output.
    pub name: String,
    /// Version requirement currently in the manifest, verbatim.
    pub old_req: String,
    /// Highest requirement satisfying the current spec, verbatim.
    pub compatible: String,
    /// Latest release overall, verbatim.
    pub latest: String,
    /// Requirement `cargo upgrade` would write, verbatim.
    pub new_req: String,
    /// Cargo's remark line for the entry. Load-bearing: an entry whose note
    /// contains `incompatible` (case-insensitive) is classified as breaking
    /// by `categorize_upgrades` — every other wording counts as compatible.
    pub note: Option<String>,
}

/// Parsed result from `cargo upgrade --dry-run`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[must_use = "UpgradeResult carries compatible/incompatible upgrade entries — silently dropping it loses the parsed report"]
#[non_exhaustive]
pub struct UpgradeResult {
    /// Entries classified as compatible upgrades.
    pub compatible: Vec<UpgradeEntry>,
    /// Entries whose note marks them as breaking upgrades.
    pub incompatible: Vec<UpgradeEntry>,
}

/// A single advisory finding from `cargo deny check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AdvisoryEntry {
    /// Advisory identifier (e.g. `RUSTSEC-2024-0001`).
    pub id: String,
    /// Affected crate name.
    pub package: String,
    /// Severity label as reported by cargo-deny.
    pub severity: String,
    /// Advisory title.
    pub title: String,
}

/// A single issue (license, ban, or source) from `cargo deny check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DenyEntry {
    /// Crate name the finding concerns.
    pub package: String,
    /// Finding text as reported by cargo-deny.
    pub message: String,
    /// Severity label as reported by cargo-deny.
    pub severity: String,
}

/// Distinct newtypes per diagnostic class — prevents cross-mixing at compile
/// time. A cargo-deny license finding; see [`DenyEntry`] for the fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LicenseEntry(pub DenyEntry);

/// A cargo-deny dependency-ban finding; see [`DenyEntry`] for the fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BanEntry(pub DenyEntry);

/// A cargo-deny source-replacement finding; see [`DenyEntry`] for the fields.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SourceEntry(pub DenyEntry);

impl std::ops::Deref for LicenseEntry {
    type Target = DenyEntry;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Deref for BanEntry {
    type Target = DenyEntry;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::Deref for SourceEntry {
    type Target = DenyEntry;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Combined result from `cargo deny check`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[must_use = "DenyResult carries advisory/license/ban/source findings — silently dropping it hides cargo-deny output"]
#[non_exhaustive]
pub struct DenyResult {
    /// Published security advisories affecting workspace crates.
    pub advisories: Vec<AdvisoryEntry>,
    /// License-policy findings.
    pub licenses: Vec<LicenseEntry>,
    /// Dependency-ban findings.
    pub bans: Vec<BanEntry>,
    /// Source-replacement findings.
    pub sources: Vec<SourceEntry>,
}

/// Full dependency health report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DepsReport {
    /// Available upgrades, split into compatible and breaking.
    pub upgrades: UpgradeResult,
    /// cargo-deny findings across advisories, licenses, bans and sources.
    pub deny: DenyResult,
}

#[cfg(test)]
mod tests;
