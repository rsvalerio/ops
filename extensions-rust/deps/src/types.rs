//! Data types for the dependency health report.
//!
//! Re-exported from the crate root, so `ops_deps::UpgradeEntry` and friends
//! keep their published paths.

use serde::{Deserialize, Serialize};

/// A single available upgrade entry from `cargo upgrade --dry-run`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UpgradeEntry {
    pub name: String,
    pub old_req: String,
    pub compatible: String,
    pub latest: String,
    pub new_req: String,
    pub note: Option<String>,
}

/// Parsed result from `cargo upgrade --dry-run`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[must_use = "UpgradeResult carries compatible/incompatible upgrade entries — silently dropping it loses the parsed report"]
#[non_exhaustive]
pub struct UpgradeResult {
    pub compatible: Vec<UpgradeEntry>,
    pub incompatible: Vec<UpgradeEntry>,
}

/// A single advisory finding from `cargo deny check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct AdvisoryEntry {
    pub id: String,
    pub package: String,
    pub severity: String,
    pub title: String,
}

/// A single issue (license, ban, or source) from `cargo deny check`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DenyEntry {
    pub package: String,
    pub message: String,
    pub severity: String,
}

/// Distinct newtypes per diagnostic class — prevents cross-mixing at compile time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct LicenseEntry(pub DenyEntry);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct BanEntry(pub DenyEntry);

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
    pub advisories: Vec<AdvisoryEntry>,
    pub licenses: Vec<LicenseEntry>,
    pub bans: Vec<BanEntry>,
    pub sources: Vec<SourceEntry>,
}

/// Full dependency health report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DepsReport {
    pub upgrades: UpgradeResult,
    pub deny: DenyResult,
}

#[cfg(test)]
mod tests;
