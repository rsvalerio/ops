//! Standardized project identity data model for `ops about`.
//!
//! Stack-specific extensions provide a `"project_identity"` data provider
//! returning a [`ProjectIdentity`] as JSON. The generic about command
//! deserializes it and converts to an [`AboutCard`] for themed rendering.

use serde::{Deserialize, Serialize};

use crate::style::{cyan, dim};

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
    /// Languages used in the project (e.g. ["Rust", "TOML"]).
    #[serde(default)]
    pub languages: Vec<String>,
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
    ("project", "Project path", "Absolute path to project root"),
    ("modules", "Module count", "Number of project modules"),
    ("code", "Lines of code", "Total lines of code (from tokei)"),
    ("files", "File count", "Total source file count"),
    ("authors", "Authors", "Project author(s)"),
    ("repository", "Repository", "Repository URL"),
    ("coverage", "Coverage", "Test coverage percentage"),
    ("languages", "Languages", "Languages used in the project"),
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

/// Map a field key to its emoji prefix.
fn field_emoji(key: &str) -> &'static str {
    match key {
        "project" => "\u{1f4c1}",                         // 📁
        "crates" | "packages" | "modules" => "\u{1f4e6}", // 📦
        "code" => "\u{1f4dd}",                            // 📝
        "files" => "\u{1f4c4}",                           // 📄
        "author" | "authors" => "\u{1f464}",              // 👤
        "repository" => "\u{1f517}",                      // 🔗
        "homepage" => "\u{1f310}",                        // 🌐
        "msrv" => "\u{2699}\u{fe0f}",                     // ⚙️
        "dependencies" => "\u{1f9e9}",                    // 🧩
        "coverage" => "\u{1f9ea}",                        // 🧪
        "languages" => "\u{1f4ac}",                       // 💬
        _ => "\u{25b8}",                                  // ▸ fallback
    }
}

/// Rendering-ready about card, derived from [`ProjectIdentity`].
pub struct AboutCard {
    /// e.g. "ops v0.10.0"
    pub title: String,
    /// e.g. "Rust · Apache-2.0"
    pub badge: String,
    pub description: Option<String>,
    /// Key-value fields: [("project", "/path/..."), ("crates", "15"), ...]
    pub fields: Vec<(String, String)>,
}

fn std_field_specs(id: &ProjectIdentity) -> Vec<(&'static str, String, Option<String>)> {
    vec![
        ("project", "project".into(), Some(id.project_path.clone())),
        (
            "modules",
            id.module_label.clone(),
            id.module_count.map(|c| c.to_string()),
        ),
        (
            "code",
            "code".into(),
            id.loc.map(|l| format!("{} loc", format_number(l))),
        ),
        (
            "files",
            "files".into(),
            id.file_count.and_then(|f| {
                (f > 0)
                    .then(|| format!("{} file{}", format_number(f), if f != 1 { "s" } else { "" }))
            }),
        ),
        ("repository", "repository".into(), id.repository.clone()),
        ("homepage", "homepage".into(), id.homepage.clone()),
        ("msrv", "msrv".into(), id.msrv.clone()),
        (
            "dependencies",
            "dependencies".into(),
            id.dependency_count.map(|c| {
                format!(
                    "{} dependenc{}",
                    format_number(c as i64),
                    if c != 1 { "ies" } else { "y" }
                )
            }),
        ),
    ]
}

fn push_special_fields(
    fields: &mut Vec<(String, String)>,
    id: &ProjectIdentity,
    show: impl Fn(&str) -> bool,
) {
    if show("authors") && !id.authors.is_empty() {
        let label = if id.authors.len() == 1 {
            "author"
        } else {
            "authors"
        };
        fields.push((label.to_string(), id.authors.join(", ")));
    }
    if show("coverage") {
        let value = match id.coverage_percent {
            Some(pct) => format!("{:.1}%", pct),
            None => "\u{26a0}\u{fe0f} run 'ops about --refresh' to collect".to_string(),
        };
        fields.push(("coverage".to_string(), value));
    }
    if show("languages") && !id.languages.is_empty() {
        fields.push(("languages".to_string(), id.languages.join(", ")));
    }
}

impl AboutCard {
    pub fn from_identity(id: &ProjectIdentity) -> Self {
        Self::from_identity_filtered(id, None)
    }

    pub fn from_identity_filtered(id: &ProjectIdentity, visible_fields: Option<&[String]>) -> Self {
        let title = match &id.version {
            Some(v) => format!("{} v{}", id.name, v),
            None => id.name.clone(),
        };

        let mut badge_parts = vec![id.stack_label.clone()];
        if let Some(detail) = &id.stack_detail {
            badge_parts.push(detail.clone());
        }
        if let Some(license) = &id.license {
            badge_parts.push(license.clone());
        }
        let badge = badge_parts.join(" \u{00b7} ");

        let show = |field_id: &str| -> bool {
            match visible_fields {
                None => true,
                Some(fields) => fields.iter().any(|f| f == field_id),
            }
        };

        let mut fields: Vec<(String, String)> = std_field_specs(id)
            .into_iter()
            .filter(|(fid, _, _)| show(fid))
            .filter_map(|(_, label, val)| val.map(|v| (label, v)))
            .collect();

        push_special_fields(&mut fields, id, show);

        Self {
            title,
            badge,
            description: id.description.clone(),
            fields,
        }
    }

    /// Render the about card as styled text lines.
    ///
    /// Pass `is_tty = true` to enable ANSI colors in the output.
    pub fn render(&self, _columns: u16, is_tty: bool) -> String {
        let mut lines = Vec::new();

        // Inline: title · badge
        let header = if is_tty {
            format!("{} \u{00b7} {}", cyan(&self.title), dim(&self.badge))
        } else {
            format!("{} \u{00b7} {}", self.title, self.badge)
        };
        lines.push(format!("  {}", header));

        // Description
        if let Some(desc) = &self.description {
            lines.push(String::new());
            lines.push(format!("  {}", desc));
        }

        // Fields
        if !self.fields.is_empty() {
            lines.push(String::new());
            let max_key_len = self.fields.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
            for (key, value) in &self.fields {
                let styled_value = if is_tty { dim(value) } else { value.clone() };
                let emoji = field_emoji(key);
                lines.push(format!(
                    "  {} {:<width$} {}",
                    emoji,
                    key,
                    styled_value,
                    width = max_key_len + 2
                ));
            }
        }

        lines.join("\n")
    }
}

use crate::text::format_number;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn about_card_from_identity_full() {
        let id = ProjectIdentity {
            name: "ops".to_string(),
            version: Some("0.10.0".to_string()),
            description: Some("Task runner".to_string()),
            stack_label: "Rust".to_string(),
            stack_detail: Some("Edition 2021".to_string()),
            license: Some("Apache-2.0".to_string()),
            project_path: "/home/user/ops".to_string(),
            module_count: Some(15),
            module_label: "crates".to_string(),
            loc: Some(21324),
            file_count: Some(96),
            authors: vec!["Alice".to_string()],
            repository: Some("https://github.com/user/ops".to_string()),
            homepage: None,
            msrv: None,
            dependency_count: None,
            coverage_percent: None,
            languages: vec![],
        };
        let card = AboutCard::from_identity(&id);
        assert_eq!(card.title, "ops v0.10.0");
        assert_eq!(card.badge, "Rust \u{00b7} Edition 2021 \u{00b7} Apache-2.0");
        assert_eq!(card.description, Some("Task runner".to_string()));
        assert_eq!(card.fields.len(), 7); // project, crates, code, files, author, repository, coverage
        assert_eq!(
            card.fields[0],
            ("project".to_string(), "/home/user/ops".to_string())
        );
        assert_eq!(card.fields[1], ("crates".to_string(), "15".to_string()));
        assert_eq!(
            card.fields[2],
            ("code".to_string(), "21,324 loc".to_string())
        );
    }

    #[test]
    fn about_card_from_identity_minimal() {
        let id = ProjectIdentity {
            name: "myproject".to_string(),
            version: None,
            description: None,
            stack_label: "Generic".to_string(),
            stack_detail: None,
            license: None,
            project_path: "/tmp/myproject".to_string(),
            module_count: None,
            module_label: "modules".to_string(),
            loc: None,
            file_count: None,
            authors: vec![],
            repository: None,
            homepage: None,
            msrv: None,
            dependency_count: None,
            coverage_percent: None,
            languages: vec![],
        };
        let card = AboutCard::from_identity(&id);
        assert_eq!(card.title, "myproject");
        assert_eq!(card.badge, "Generic");
        assert!(card.description.is_none());
        assert_eq!(card.fields.len(), 2); // project path + coverage hint
    }

    fn sample_identity() -> ProjectIdentity {
        ProjectIdentity {
            name: "ops".to_string(),
            version: Some("0.10.0".to_string()),
            description: Some("Task runner".to_string()),
            stack_label: "Rust".to_string(),
            stack_detail: Some("Edition 2021".to_string()),
            license: Some("Apache-2.0".to_string()),
            project_path: "/home/user/ops".to_string(),
            module_count: Some(15),
            module_label: "crates".to_string(),
            loc: Some(21324),
            file_count: Some(96),
            authors: vec!["Alice".to_string()],
            repository: Some("https://github.com/user/ops".to_string()),
            homepage: None,
            msrv: None,
            dependency_count: None,
            coverage_percent: None,
            languages: vec![],
        }
    }

    #[test]
    fn render_non_tty_contains_title_and_badge() {
        let card = AboutCard::from_identity(&sample_identity());
        let output = card.render(80, false);
        assert!(output.contains("ops v0.10.0"), "got: {output}");
        assert!(output.contains("Rust"), "got: {output}");
        assert!(output.contains("Apache-2.0"), "got: {output}");
    }

    #[test]
    fn render_non_tty_contains_fields() {
        let card = AboutCard::from_identity(&sample_identity());
        let output = card.render(80, false);
        assert!(output.contains("/home/user/ops"), "got: {output}");
        assert!(output.contains("21,324 loc"), "got: {output}");
        assert!(output.contains("96 files"), "got: {output}");
        assert!(output.contains("Alice"), "got: {output}");
    }

    #[test]
    fn render_non_tty_contains_description() {
        let card = AboutCard::from_identity(&sample_identity());
        let output = card.render(80, false);
        assert!(output.contains("Task runner"), "got: {output}");
    }

    #[test]
    fn render_tty_contains_ansi_escapes() {
        let card = AboutCard::from_identity(&sample_identity());
        let output = card.render(80, true);
        // ANSI escape codes start with \x1b[
        assert!(
            output.contains("\x1b["),
            "TTY output should contain ANSI escapes: {output}"
        );
    }

    #[test]
    fn render_non_tty_no_ansi_escapes() {
        let card = AboutCard::from_identity(&sample_identity());
        let output = card.render(80, false);
        assert!(
            !output.contains("\x1b["),
            "non-TTY output should not contain ANSI escapes: {output}"
        );
    }

    #[test]
    fn render_minimal_card_no_description() {
        let id = ProjectIdentity {
            name: "bare".to_string(),
            version: None,
            description: None,
            stack_label: "Generic".to_string(),
            stack_detail: None,
            license: None,
            project_path: "/tmp".to_string(),
            module_count: None,
            module_label: "modules".to_string(),
            loc: None,
            file_count: None,
            authors: vec![],
            repository: None,
            homepage: None,
            msrv: None,
            dependency_count: None,
            coverage_percent: None,
            languages: vec![],
        };
        let card = AboutCard::from_identity(&id);
        let output = card.render(80, false);
        assert!(output.contains("bare"), "got: {output}");
        assert!(output.contains("/tmp"), "got: {output}");
        // 2 fields (project + coverage hint), no description line
        assert_eq!(output.matches('\n').count(), 3); // header, blank, field, field
    }

    #[test]
    fn file_count_singular() {
        let mut id = sample_identity();
        id.file_count = Some(1);
        let card = AboutCard::from_identity(&id);
        let output = card.render(80, false);
        assert!(output.contains("1 file"), "got: {output}");
        assert!(!output.contains("1 files"), "should be singular: {output}");
    }
}
