//! `AboutCard` construction and rendering.

use super::format::{
    compose_codebase_value, compose_project_value, compose_stack_value, field_emoji,
};
use super::ProjectIdentity;
use crate::style::dim;
use crate::text::format_number;

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
        let mut lines: Vec<String> = Vec::new();

        if let Some(desc) = &self.description {
            lines.push(String::new());
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
