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
// Downstream extensions cannot use struct-literal or `..Default::default()`
// syntax once this type is `#[non_exhaustive]`; call [`ProjectIdentity::new`]
// and mutate the returned value via its `pub` fields instead.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
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

impl ProjectIdentity {
    /// Build a [`ProjectIdentity`] with the four required fields set and
    /// every other field left at its default. Use direct field assignment to
    /// populate the rest.
    ///
    /// Required to construct [`ProjectIdentity`] from outside `ops-core`
    /// because the type is `#[non_exhaustive]`.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        stack_label: impl Into<String>,
        project_path: impl Into<String>,
        module_label: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            stack_label: stack_label.into(),
            project_path: project_path.into(),
            module_label: module_label.into(),
            ..Self::default()
        }
    }
}

/// Per-language breakdown entry derived from tokei data.
///
/// API-9 / TASK-0858: `#[non_exhaustive]` mirrors `ProjectIdentity` so a
/// future field (e.g. `comments`) is additive across the extension
/// boundary. Construct via [`LanguageStat::new`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
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

impl LanguageStat {
    /// Construct a [`LanguageStat`] with all current fields set.
    ///
    /// External extensions must use this instead of struct-literal syntax
    /// because the type is `#[non_exhaustive]`.
    #[must_use]
    pub fn new(
        name: impl Into<String>,
        loc: i64,
        files: i64,
        loc_pct: f64,
        files_pct: f64,
    ) -> Self {
        Self {
            name: name.into(),
            loc,
            files,
            loc_pct,
            files_pct,
        }
    }
}

/// A sub-unit of a project (crate, module, package, workspace member).
///
/// Stack-specific extensions provide a `"project_units"` data provider returning
/// `Vec<ProjectUnit>` as JSON. The generic `about units` subpage renders these
/// as a grid of cards.
/// API-9 / TASK-0858: `#[non_exhaustive]` for the same reason as
/// [`ProjectIdentity`]. Construct via [`ProjectUnit::new`] (required
/// `name` + `path`); set optional fields directly on the returned value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
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

impl ProjectUnit {
    /// API-9 / TASK-0858: required-only constructor. Optional fields stay
    /// at their `Option::None` / numeric defaults; assign them on the
    /// returned value (`u.version = Some(...)`) since field access is
    /// not affected by `#[non_exhaustive]`.
    #[must_use]
    pub fn new(name: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            ..Self::default()
        }
    }
}

/// Lines-covered / total for a coverage report.
///
/// API-9 / TASK-0858: `#[non_exhaustive]` to keep future field additions
/// (e.g. branch coverage) source-compatible across the extension boundary.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CoverageStats {
    pub lines_percent: f64,
    pub lines_covered: i64,
    pub lines_count: i64,
}

impl CoverageStats {
    #[must_use]
    pub fn new(lines_percent: f64, lines_covered: i64, lines_count: i64) -> Self {
        Self {
            lines_percent,
            lines_covered,
            lines_count,
        }
    }
}

/// Coverage breakdown for a single unit (crate/module/package).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UnitCoverage {
    /// Display name for the unit (typically resolved from stack metadata).
    pub unit_name: String,
    /// Relative path of the unit from the project root.
    pub unit_path: String,
    pub stats: CoverageStats,
}

impl UnitCoverage {
    #[must_use]
    pub fn new(
        unit_name: impl Into<String>,
        unit_path: impl Into<String>,
        stats: CoverageStats,
    ) -> Self {
        Self {
            unit_name: unit_name.into(),
            unit_path: unit_path.into(),
            stats,
        }
    }
}

/// Project-wide coverage, optionally broken down by unit.
///
/// Returned by the `"project_coverage"` data provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProjectCoverage {
    pub total: CoverageStats,
    #[serde(default)]
    pub units: Vec<UnitCoverage>,
}

impl ProjectCoverage {
    #[must_use]
    pub fn new(total: CoverageStats, units: Vec<UnitCoverage>) -> Self {
        Self { total, units }
    }
}

/// Direct dependencies of a single unit.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct UnitDeps {
    pub unit_name: String,
    /// (dependency name, version requirement) pairs.
    pub deps: Vec<(String, String)>,
}

impl UnitDeps {
    #[must_use]
    pub fn new(unit_name: impl Into<String>, deps: Vec<(String, String)>) -> Self {
        Self {
            unit_name: unit_name.into(),
            deps,
        }
    }
}

/// Project-wide dependency tree, keyed by unit.
///
/// Returned by the `"project_dependencies"` data provider.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct ProjectDependencies {
    pub units: Vec<UnitDeps>,
}

impl ProjectDependencies {
    #[must_use]
    pub fn new(units: Vec<UnitDeps>) -> Self {
        Self { units }
    }
}

/// Metadata for a field that can appear on the about card.
#[derive(Clone)]
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

/// Insert a `homepage` field just before `coverage` (or at the end if `coverage`
/// is absent). Shared helper so every stack renders the homepage field in the
/// same slot of the about card.
pub fn insert_homepage_field(fields: &mut Vec<AboutFieldDef>) {
    let insert_pos = fields
        .iter()
        .position(|f| f.id == "coverage")
        .unwrap_or(fields.len());
    fields.insert(
        insert_pos,
        AboutFieldDef {
            id: "homepage",
            label: "Homepage",
            description: "Project homepage URL",
        },
    );
}
