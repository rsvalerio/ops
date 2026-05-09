//! `AboutCard` construction and rendering.

use super::format::{
    compose_codebase_value, compose_project_value, compose_stack_value, field_emoji,
};
use super::ProjectIdentity;
use crate::output::display_width;
use crate::style::dim_gated;
use crate::text::format_number;

/// Rendering-ready about card, derived from [`ProjectIdentity`].
///
/// Everything that used to live in a title/badge header (name, version, stack,
/// license) is now rendered as ordinary fields.
#[non_exhaustive]
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
            id.license.as_ref().filter(|s| !s.is_empty()).cloned(),
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
                // Avoid `as i64` narrowing (SEC-15 / TASK-0339): saturate so an
                // unrealistically large usize still renders a sensible string
                // instead of wrapping into a negative i64.
                let n = i64::try_from(c).unwrap_or(i64::MAX);
                format!(
                    "{} dependenc{}",
                    format_number(n),
                    if c != 1 { "ies" } else { "y" }
                )
            }),
        ),
        (
            "repository",
            "repository".into(),
            id.repository.as_ref().filter(|s| !s.is_empty()).cloned(),
        ),
        (
            "homepage",
            "homepage".into(),
            id.homepage.as_ref().filter(|s| !s.is_empty()).cloned(),
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
    /// API-9 / TASK-0892: builder so a future field addition stays
    /// non-breaking. The previous `AboutCard::new(description, fields)`
    /// positional constructor exposed every current field — adding a
    /// third would have been a breaking signature change, defeating
    /// `#[non_exhaustive]`.
    #[must_use]
    pub fn builder() -> AboutCardBuilder {
        AboutCardBuilder::default()
    }

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

        Self::builder()
            .description(id.description.clone())
            .fields(fields)
            .build()
    }

    /// Render the about card as styled text lines.
    ///
    /// Pass `is_tty = true` to enable ANSI colors in the output.
    pub fn render(&self, is_tty: bool) -> String {
        let mut lines: Vec<String> = Vec::new();

        if let Some(desc) = &self.description {
            lines.push(String::new());
            lines.push(format!("  {}", desc));
        }

        if !self.fields.is_empty() {
            if !lines.is_empty() {
                lines.push(String::new());
            }
            // PERF-3 / TASK-1220: align by display width, not byte length, so
            // multi-byte keys do not shift the value column by one cell per
            // non-ASCII char. Mirrors the format_language_breakdown / theme_cmd
            // alignment pattern.
            let max_key_len = self
                .fields
                .iter()
                .map(|(k, _)| display_width(k))
                .max()
                .unwrap_or(0);
            let cont_indent = continuation_indent(max_key_len);
            for (key, value) in &self.fields {
                lines.extend(render_field(key, value, max_key_len, &cont_indent, is_tty));
            }
        }

        lines.join("\n")
    }
}

/// API-9 / TASK-0892: builder for [`AboutCard`]. New fields land as
/// additional setter methods rather than positional constructor args, so
/// downstream code that built via `AboutCard::builder().description(...)
/// .fields(...).build()` keeps compiling unchanged.
#[derive(Default)]
pub struct AboutCardBuilder {
    description: Option<String>,
    fields: Vec<(String, String)>,
}

impl AboutCardBuilder {
    #[must_use]
    pub fn description(mut self, description: Option<String>) -> Self {
        self.description = description;
        self
    }

    #[must_use]
    pub fn fields(mut self, fields: Vec<(String, String)>) -> Self {
        self.fields = fields;
        self
    }

    #[must_use]
    pub fn build(self) -> AboutCard {
        AboutCard {
            description: self.description,
            fields: self.fields,
        }
    }
}

/// Number of leading spaces that align value continuation lines under the
/// first value line:
///   2 (leading) + 2 (emoji cols) + 1 (space) + (max_key_len + 2) + 1.
fn continuation_indent(max_key_len: usize) -> String {
    " ".repeat(2 + 2 + 1 + (max_key_len + 2) + 1)
}

/// Render a single `(key, value)` row into its first line plus any
/// continuation lines, applying `dim` styling to values when `is_tty`.
fn render_field(
    key: &str,
    value: &str,
    max_key_len: usize,
    cont_indent: &str,
    is_tty: bool,
) -> Vec<String> {
    let styled = |s: &str| dim_gated(s, is_tty);
    let emoji = field_emoji(key, value);
    let mut value_lines = value.split('\n');
    let first = value_lines.next().unwrap_or("");
    let pad = (max_key_len + 2).saturating_sub(display_width(key));
    let mut padded_key = String::from(key);
    for _ in 0..pad {
        padded_key.push(' ');
    }
    let mut out = vec![format!("  {} {} {}", emoji, padded_key, styled(first))];
    for cont in value_lines {
        out.push(format!("{}{}", cont_indent, styled(cont)));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PERF-3 / TASK-1220: a multi-byte key must not shift the value column
    /// by one cell per non-ASCII char. The value should start at the same
    /// display column as it would for an ASCII-only key of equal width.
    #[test]
    fn render_field_aligns_multi_byte_key_by_display_width() {
        let card = AboutCardBuilder::default()
            .fields(vec![
                ("name".to_string(), "alpha".to_string()),
                ("名前".to_string(), "beta".to_string()),
            ])
            .build();
        let rendered = card.render(false);
        let lines: Vec<&str> = rendered.lines().collect();
        let name_line = lines.iter().find(|l| l.contains("alpha")).unwrap();
        let cjk_line = lines.iter().find(|l| l.contains("beta")).unwrap();
        let value_col = |line: &str, value: &str| -> usize {
            let idx = line.find(value).unwrap();
            display_width(&line[..idx])
        };
        assert_eq!(
            value_col(name_line, "alpha"),
            value_col(cjk_line, "beta"),
            "value column must align by display width"
        );
    }
}
