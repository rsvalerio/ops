//! Standardized project identity data model for `ops about`.
//!
//! Stack-specific extensions provide a `"project_identity"` data provider
//! returning a [`ProjectIdentity`] as JSON. The generic about command
//! deserializes it and converts to an [`AboutCard`] for themed rendering.

use serde::{Deserialize, Serialize};

use crate::style::dim;

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

/// Map a field (key, value) pair to its emoji prefix. The `stack` field is
/// value-aware so each language gets its own glyph.
fn field_emoji(key: &str, value: &str) -> &'static str {
    match key {
        "stack" => stack_emoji(value),
        "license" => "\u{1f4dc}",                         // 📜
        "project" => "\u{1f3f7}\u{fe0f}",                 // 🏷️
        "crates" | "packages" | "modules" => "\u{1f4e6}", // 📦
        "codebase" => "\u{1f4dd}",                        // 📝
        "author" | "authors" => "\u{1f464}",              // 👤
        "repository" => "\u{1f517}",                      // 🔗
        "homepage" => "\u{1f310}",                        // 🌐
        "dependencies" => "\u{1f9e9}",                    // 🧩
        "coverage" => "\u{1f9ea}",                        // 🧪
        _ => "\u{25b8}",                                  // ▸ fallback
    }
}

/// Language-specific emoji derived from the stack label (first token of the
/// `stack` field value, e.g. `"Rust · Edition 2021"` → `"Rust"` → 🦀).
fn stack_emoji(value: &str) -> &'static str {
    let label = value.split_whitespace().next().unwrap_or("");
    match label {
        "Rust" => "\u{1f980}",               // 🦀
        "Go" => "\u{1f439}",                 // 🐹
        "Node" | "JavaScript" => "\u{2b22}", // ⬢
        "Python" => "\u{1f40d}",             // 🐍
        "Java" => "\u{2615}",                // ☕
        "Terraform" => "\u{1f4a0}",          // 💠
        "Ansible" => "\u{1f170}\u{fe0f}",    // 🅰️
        _ => "\u{1f4da}",                    // 📚 generic
    }
}

/// Shorten long language names for the compact codebase field.
fn short_language_name(name: &str) -> &str {
    match name {
        "JavaScript" => "JS",
        "TypeScript" => "TS",
        "Protocol Buffers" => "Protobuf",
        "Handlebars" => "HBS",
        other => other,
    }
}

/// Compose the `languages` tail of the codebase field: keep the top 3 entries
/// and roll the rest into `+N more`. Each input is already formatted as
/// `"Language X.Y%"` by the data provider.
fn format_language_mix(langs: &[String], top_n: usize) -> String {
    if langs.is_empty() {
        return String::new();
    }
    let shortened: Vec<String> = langs
        .iter()
        .map(|entry| match entry.rsplit_once(' ') {
            Some((name, pct)) => format!("{} {}", short_language_name(name), pct),
            None => entry.clone(),
        })
        .collect();
    if shortened.len() <= top_n {
        return shortened.join(", ");
    }
    let head = shortened[..top_n].join(", ");
    let rest = shortened.len() - top_n;
    format!("{}, +{} more", head, rest)
}

/// Compose the multi-line `stack` field value: language label, optional
/// stack detail (e.g. "Edition 2021"), and optional MSRV.
fn compose_stack_value(id: &ProjectIdentity) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if !id.stack_label.is_empty() {
        parts.push(id.stack_label.clone());
    }
    if let Some(d) = &id.stack_detail {
        if !d.is_empty() {
            parts.push(d.clone());
        }
    }
    if let Some(msrv) = &id.msrv {
        if !msrv.is_empty() {
            parts.push(format!("{} (msrv)", msrv));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

/// Compose the multi-line `project` field value: name, optional version
/// line, and project path.
fn compose_project_value(id: &ProjectIdentity) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if !id.name.is_empty() {
        parts.push(id.name.clone());
    }
    if let Some(v) = &id.version {
        if !v.is_empty() {
            parts.push(format!("v{}", v));
        }
    }
    if !id.project_path.is_empty() {
        parts.push(id.project_path.clone());
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

/// Compose the single `codebase` field value from LOC, file count, and
/// language mix. Returns `None` when none of the inputs are present.
fn compose_codebase_value(id: &ProjectIdentity) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(loc) = id.loc {
        parts.push(format!("{} loc", format_number(loc)));
    }
    if let Some(f) = id.file_count {
        if f > 0 {
            parts.push(format!(
                "{} file{}",
                format_number(f),
                if f != 1 { "s" } else { "" }
            ));
        }
    }
    let langs = format_language_mix(&id.languages, 3);
    if !langs.is_empty() {
        parts.push(langs);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

/// Rendering-ready about card, derived from [`ProjectIdentity`].
///
/// Everything that used to live in a title/badge header (name, version, stack,
/// license) is now rendered as ordinary fields.
pub struct AboutCard {
    pub description: Option<String>,
    /// Key-value fields: [("name", "ops v0.10.0"), ("stack", "Rust · Edition 2021"), ("project", "/path"), ...]
    pub fields: Vec<(String, String)>,
}

fn std_field_specs(id: &ProjectIdentity) -> Vec<(&'static str, String, Option<String>)> {
    let stack_value = compose_stack_value(id);
    let project_value = compose_project_value(id);
    vec![
        ("project", "project".into(), project_value),
        ("stack", "stack".into(), stack_value),
        (
            "license",
            "license".into(),
            id.license.clone().filter(|s| !s.is_empty()),
        ),
        (
            "modules",
            id.module_label.clone(),
            id.module_count.map(|c| c.to_string()),
        ),
        ("codebase", "codebase".into(), compose_codebase_value(id)),
        (
            "dependencies",
            "dependencies".into(),
            id.dependency_count.filter(|&c| c > 0).map(|c| {
                format!(
                    "{} dependenc{}",
                    format_number(c as i64),
                    if c != 1 { "ies" } else { "y" }
                )
            }),
        ),
        (
            "repository",
            "repository".into(),
            id.repository.clone().filter(|s| !s.is_empty()),
        ),
        (
            "homepage",
            "homepage".into(),
            id.homepage.clone().filter(|s| !s.is_empty()),
        ),
    ]
}

fn push_special_fields(
    fields: &mut Vec<(String, String)>,
    id: &ProjectIdentity,
    show: impl Fn(&str) -> bool,
    explicit_filter: bool,
) {
    if show("authors") && !id.authors.is_empty() {
        let label = if id.authors.len() == 1 {
            "author"
        } else {
            "authors"
        };
        fields.push((label.to_string(), id.authors.join(", ")));
    }
    // Coverage: show the percentage when known. When unknown, only render the
    // "not collected" placeholder if the user has explicitly asked for the
    // coverage field via config (visible_fields); otherwise hide it so the
    // card stays compact for stacks that haven't wired up coverage yet.
    if show("coverage") {
        match id.coverage_percent {
            Some(pct) => fields.push(("coverage".to_string(), format!("{:.1}%", pct))),
            None if explicit_filter => {
                fields.push(("coverage".to_string(), "not collected".to_string()));
            }
            None => {}
        }
    }
}

impl AboutCard {
    pub fn from_identity(id: &ProjectIdentity) -> Self {
        Self::from_identity_filtered(id, None)
    }

    pub fn from_identity_filtered(id: &ProjectIdentity, visible_fields: Option<&[String]>) -> Self {
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

        push_special_fields(&mut fields, id, show, visible_fields.is_some());

        Self {
            description: id.description.clone(),
            fields,
        }
    }

    /// Render the about card as styled text lines.
    ///
    /// Pass `is_tty = true` to enable ANSI colors in the output.
    pub fn render(&self, _columns: u16, is_tty: bool) -> String {
        let mut lines = vec![String::new()];

        if let Some(desc) = &self.description {
            lines.push(format!("  {}", desc));
        }

        if !self.fields.is_empty() {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            let max_key_len = self.fields.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
            // Continuation indent: 2 leading + 2 emoji cols + 1 space + key width + 1 space.
            let cont_indent = " ".repeat(2 + 2 + 1 + (max_key_len + 2) + 1);
            for (key, value) in &self.fields {
                let emoji = field_emoji(key, value);
                let mut value_lines = value.split('\n');
                let first = value_lines.next().unwrap_or("");
                let styled_first = if is_tty {
                    dim(first)
                } else {
                    first.to_string()
                };
                lines.push(format!(
                    "  {} {:<width$} {}",
                    emoji,
                    key,
                    styled_first,
                    width = max_key_len + 2
                ));
                for cont in value_lines {
                    let styled = if is_tty { dim(cont) } else { cont.to_string() };
                    lines.push(format!("{}{}", cont_indent, styled));
                }
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
        assert_eq!(card.description, Some("Task runner".to_string()));
        // project, stack, license, crates, codebase, repository, author
        // (no coverage — hidden when empty + all-fields mode)
        assert_eq!(card.fields.len(), 7);
        assert_eq!(
            card.fields[0],
            (
                "project".to_string(),
                "ops\nv0.10.0\n/home/user/ops".to_string()
            )
        );
        assert_eq!(
            card.fields[1],
            ("stack".to_string(), "Rust\nEdition 2021".to_string())
        );
        assert_eq!(
            card.fields[2],
            ("license".to_string(), "Apache-2.0".to_string())
        );
        assert_eq!(card.fields[3], ("crates".to_string(), "15".to_string()));
        assert_eq!(
            card.fields[4],
            ("codebase".to_string(), "21,324 loc\n96 files".to_string())
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
        assert!(card.description.is_none());
        // project, stack — no license, no coverage (empty).
        assert_eq!(card.fields.len(), 2);
        assert_eq!(
            card.fields[0],
            (
                "project".to_string(),
                "myproject\n/tmp/myproject".to_string()
            )
        );
        assert_eq!(card.fields[1], ("stack".to_string(), "Generic".to_string()));
    }

    #[test]
    fn about_card_codebase_with_languages() {
        let id = ProjectIdentity {
            name: "openbao".to_string(),
            version: None,
            description: None,
            stack_label: "Go".to_string(),
            stack_detail: None,
            license: None,
            project_path: "/p".to_string(),
            module_count: Some(7),
            module_label: "modules".to_string(),
            loc: Some(555_910),
            file_count: Some(4_929),
            authors: vec![],
            repository: None,
            homepage: None,
            msrv: None,
            dependency_count: None,
            coverage_percent: None,
            languages: vec![
                "Go 77.7%".into(),
                "JavaScript 11.5%".into(),
                "Handlebars 3.9%".into(),
                "YAML 2.6%".into(),
                "SVG 1.2%".into(),
            ],
        };
        let card = AboutCard::from_identity(&id);
        let codebase = card
            .fields
            .iter()
            .find(|(k, _)| k == "codebase")
            .expect("codebase field")
            .1
            .clone();
        assert_eq!(
            codebase,
            "555,910 loc\n4,929 files\nGo 77.7%, JS 11.5%, HBS 3.9%, +2 more"
        );
    }

    #[test]
    fn about_card_coverage_hidden_when_empty() {
        let id = ProjectIdentity {
            name: "x".into(),
            stack_label: "Rust".into(),
            project_path: "/p".into(),
            module_label: "crates".into(),
            coverage_percent: None,
            ..Default::default()
        };
        let card = AboutCard::from_identity(&id);
        assert!(card.fields.iter().all(|(k, _)| k != "coverage"));
    }

    #[test]
    fn about_card_coverage_shown_when_explicitly_selected() {
        let id = ProjectIdentity {
            name: "x".into(),
            stack_label: "Rust".into(),
            project_path: "/p".into(),
            module_label: "crates".into(),
            coverage_percent: None,
            ..Default::default()
        };
        let card = AboutCard::from_identity_filtered(
            &id,
            Some(&["project".to_string(), "coverage".to_string()]),
        );
        let cov = card
            .fields
            .iter()
            .find(|(k, _)| k == "coverage")
            .expect("coverage");
        assert_eq!(cov.1, "not collected");
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
    fn render_non_tty_contains_identity_fields() {
        let card = AboutCard::from_identity(&sample_identity());
        let output = card.render(80, false);
        assert!(output.contains("ops"), "got: {output}");
        assert!(output.contains("v0.10.0"), "got: {output}");
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
        // stack, project — project spans 2 lines (name + path). 3 output lines.
        assert_eq!(output.matches('\n').count(), 2);
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
