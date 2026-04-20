//! Standardized project identity data model for `ops about`.
//!
//! Stack-specific extensions provide a `"project_identity"` data provider
//! returning a [`ProjectIdentity`] as JSON. The generic about command
//! deserializes it and converts to an [`AboutCard`] for themed rendering.
//!
//! Split for cohesion:
//! - `format` — emoji and multi-line value composition helpers
//! - `card`   — `AboutCard` construction and styled rendering

mod card;
mod format;
#[cfg(test)]
mod tests;

pub use card::AboutCard;

use serde::{Deserialize, Serialize};

/// Canonical project identity returned by stack-specific data providers.
///
/// Each stack provides its own `"project_identity"` data provider that maps
/// stack-native manifest fields to this struct. See `docs/components.md` §10
/// for the full field reference per stack.
///
/// Fields like `loc` and `file_count` come from tokei (via DuckDB) and are
/// enriched by the generic about command when the provider doesn't set them.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectIdentity {
    /// Project name. Rust: `[package].name`, Node: `name`, fallback: directory name.
    pub name: String,
    /// Project version. Rust: `[package].version`, Node: `version`.
    pub version: Option<String>,
    /// Short project description.
    pub description: Option<String>,
    /// Human-readable stack name: "Rust", "Node", "Go", "Python", etc.
    pub stack_label: String,
    /// Stack-specific detail: "Edition 2021", "ESM", "Go 1.21", "3.12", etc.
    pub stack_detail: Option<String>,
    /// SPDX license identifier.
    pub license: Option<String>,
    /// Absolute path to the project/workspace root.
    pub project_path: String,
    /// Number of sub-projects. Rust: workspace members, Node: workspaces.
    pub module_count: Option<usize>,
    /// Stack-native label for modules: "crates", "packages", "modules", etc.
    pub module_label: String,
    /// Total lines of code (from tokei via DuckDB).
    pub loc: Option<i64>,
    /// Total source file count (from tokei via DuckDB).
    pub file_count: Option<i64>,
    /// Author list. Rust: `[package].authors`, Node: `author` + `contributors`.
    pub authors: Vec<String>,
    /// Repository URL.
    pub repository: Option<String>,
    /// Homepage URL (distinct from repository).
    #[serde(default)]
    pub homepage: Option<String>,
    /// Minimum supported Rust version / language version.
    #[serde(default)]
    pub msrv: Option<String>,
    /// Total dependency count.
    #[serde(default)]
    pub dependency_count: Option<usize>,
    /// Test coverage percentage (0.0–100.0).
    #[serde(default)]
    pub coverage_percent: Option<f64>,
    /// Languages used in the project, with LOC and file counts plus percentages.
    /// Ordered by LOC descending.
    #[serde(default)]
    pub languages: Vec<LanguageStat>,
}

/// Per-language breakdown entry derived from tokei data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LanguageStat {
    /// Language name (e.g. "Rust", "TOML").
    pub name: String,
    /// Lines of code in this language.
    pub loc: i64,
    /// Number of files in this language.
    pub files: i64,
    /// Percentage of total project LOC (0.0–100.0).
    pub loc_pct: f64,
    /// Percentage of total project files (0.0–100.0).
    pub files_pct: f64,
}

/// A sub-unit of a project (crate, module, package, workspace member).
///
/// Stack-specific extensions provide a `"project_units"` data provider returning
/// `Vec<ProjectUnit>` as JSON. The generic `about units` subpage renders these
/// as a grid of cards.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectUnit {
    /// Display name (typically capitalized or from package metadata).
    pub name: String,
    /// Relative path from the project root.
    pub path: String,
    /// Semver/version string, if applicable.
    #[serde(default)]
    pub version: Option<String>,
    /// Short description.
    #[serde(default)]
    pub description: Option<String>,
    /// Lines of code.
    #[serde(default)]
    pub loc: Option<i64>,
    /// Source file count.
    #[serde(default)]
    pub file_count: Option<i64>,
    /// Dependency count.
    #[serde(default)]
    pub dep_count: Option<i64>,
}

/// Lines-covered / total for a coverage report.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct CoverageStats {
    pub lines_percent: f64,
    pub lines_covered: i64,
    pub lines_count: i64,
}

/// Coverage breakdown for a single unit (crate/module/package).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnitCoverage {
    /// Display name for the unit (typically resolved from stack metadata).
    pub unit_name: String,
    /// Relative path of the unit from the project root.
    pub unit_path: String,
    pub stats: CoverageStats,
}

/// Project-wide coverage, optionally broken down by unit.
///
/// Returned by the `"project_coverage"` data provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectCoverage {
    pub total: CoverageStats,
    #[serde(default)]
    pub units: Vec<UnitCoverage>,
}

/// Direct dependencies of a single unit.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UnitDeps {
    pub unit_name: String,
    /// (dependency name, version requirement) pairs.
    pub deps: Vec<(String, String)>,
}

/// Project-wide dependency tree, keyed by unit.
///
/// Returned by the `"project_dependencies"` data provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectDependencies {
    pub units: Vec<UnitDeps>,
}

/// Metadata for a field that can appear on the about card.
pub struct AboutFieldDef {
    /// Identifier used in config (e.g. "project", "code").
    pub id: &'static str,
    /// Human-readable label for interactive prompts.
    pub label: &'static str,
    /// Short description shown in the MultiSelect prompt.
    pub description: &'static str,
}

/// Common about-field definitions shared by all stack identity providers.
///
/// Each tuple is `(id, label, description)`. Stack-specific providers can call
/// [`base_about_fields`] to get these as `Vec<AboutFieldDef>` and append any
/// extras (e.g. Rust adds `homepage`, `msrv`, `dependencies`).
pub const BASE_ABOUT_FIELDS: &[(&str, &str, &str)] = &[
    (
        "stack",
        "Stack",
        "Language/stack and variant (e.g. Edition 2021)",
    ),
    ("license", "License", "SPDX license identifier"),
    ("project", "Project", "Project name, version, and path"),
    ("modules", "Module count", "Number of project modules"),
    (
        "codebase",
        "Codebase",
        "LOC, file count, and language mix (from tokei)",
    ),
    ("repository", "Repository", "Repository URL"),
    ("authors", "Authors", "Project author(s)"),
    ("coverage", "Coverage", "Test coverage percentage"),
];

/// Convert [`BASE_ABOUT_FIELDS`] into a `Vec<AboutFieldDef>`.
pub fn base_about_fields() -> Vec<AboutFieldDef> {
    BASE_ABOUT_FIELDS
        .iter()
        .map(|(id, label, description)| AboutFieldDef {
            id,
            label,
            description,
        })
        .collect()
}
