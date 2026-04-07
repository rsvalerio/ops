//! Standardized project identity data model for `ops about`.
//!
//! Stack-specific extensions provide a `"project_identity"` data provider
//! returning a [`ProjectIdentity`] as JSON. The generic about command
//! deserializes it and converts to an [`AboutCard`] for themed rendering.

use std::io::{self, IsTerminal};

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
#[derive(Debug, Clone, Serialize, Deserialize)]
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

impl AboutCard {
    pub fn from_identity(id: &ProjectIdentity) -> Self {
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

        let mut fields = vec![("project".to_string(), id.project_path.clone())];

        if let Some(count) = id.module_count {
            fields.push((id.module_label.clone(), count.to_string()));
        }
        if let Some(loc) = id.loc {
            fields.push(("code".to_string(), format!("{} loc", format_number(loc))));
        }
        if let Some(files) = id.file_count {
            if files > 0 {
                let suffix = if files != 1 { "s" } else { "" };
                fields.push((
                    "files".to_string(),
                    format!("{} file{}", format_number(files), suffix),
                ));
            }
        }
        if !id.authors.is_empty() {
            let label = if id.authors.len() == 1 {
                "author"
            } else {
                "authors"
            };
            fields.push((label.to_string(), id.authors.join(", ")));
        }
        if let Some(url) = &id.repository {
            fields.push(("repository".to_string(), url.clone()));
        }

        Self {
            title,
            badge,
            description: id.description.clone(),
            fields,
        }
    }

    /// Render the about card as styled text lines.
    ///
    /// Uses ANSI colors when stdout is a TTY.
    pub fn render(&self, _columns: u16) -> String {
        let is_tty = io::stdout().is_terminal();
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
                lines.push(format!(
                    "  \u{25b8} {:<width$} {}",
                    key,
                    styled_value,
                    width = max_key_len + 2
                ));
            }
        }

        lines.join("\n")
    }
}

/// Format a number with comma separators (e.g. 1234 → "1,234").
fn format_number(n: i64) -> String {
    let s = n.to_string();
    let mut result = String::with_capacity(s.len() + s.len() / 3);
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_number_small() {
        assert_eq!(format_number(42), "42");
    }

    #[test]
    fn format_number_thousands() {
        assert_eq!(format_number(1234), "1,234");
    }

    #[test]
    fn format_number_millions() {
        assert_eq!(format_number(1_234_567), "1,234,567");
    }

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
        };
        let card = AboutCard::from_identity(&id);
        assert_eq!(card.title, "ops v0.10.0");
        assert_eq!(card.badge, "Rust \u{00b7} Edition 2021 \u{00b7} Apache-2.0");
        assert_eq!(card.description, Some("Task runner".to_string()));
        assert_eq!(card.fields.len(), 6); // project, crates, code, files, author, repository
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
        };
        let card = AboutCard::from_identity(&id);
        assert_eq!(card.title, "myproject");
        assert_eq!(card.badge, "Generic");
        assert!(card.description.is_none());
        assert_eq!(card.fields.len(), 1); // just project path
    }
}
