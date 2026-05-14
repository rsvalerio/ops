//! `AboutCard` construction and rendering.

use std::collections::HashSet;

use super::format::{
    compose_codebase_value, compose_project_value, compose_stack_value, field_emoji,
};
use super::ProjectIdentity;
use crate::output::{display_width, pad_to_display_width};
use crate::style::dim_gated;
use crate::text::format_number;
use crate::ui::sanitise_line;

/// Rendering-ready about card, derived from [`ProjectIdentity`].
///
/// Everything that used to live in a title/badge header (name, version, stack,
/// license) is now rendered as ordinary fields.
///
/// TRAIT-1 / TASK-1435: derives `Debug` and `Clone` (C-DEBUG, C-CLONE) so
/// downstream extension structs that wrap an `AboutCard` get the derives
/// mechanically and `tracing::debug!(card = ?card, ...)` compiles.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct AboutCard {
    pub description: Option<String>,
    /// Key-value fields: [("name", "ops v0.10.0"), ("stack", "Rust · Edition 2021"), ("project", "/path"), ...]
    pub fields: Vec<(String, String)>,
}

/// FN-4 / TASK-1406: named about-card field row so callers read by name
/// (`spec.label`, `spec.value`) instead of remembering positional slot order
/// in a tuple. The field id is the dispatch key in [`shown_field_specs`] and
/// is not carried inside the spec.
struct FieldSpec {
    label: String,
    value: Option<String>,
}

/// PERF-3 / TASK-1391: clone an `Option<String>` only when it has non-empty,
/// non-whitespace content. Centralises the previously-duplicated
/// `as_ref().filter(...).cloned()` idiom so a future tightening lands once.
fn non_empty_clone(opt: &Option<String>) -> Option<String> {
    opt.as_ref().filter(|s| !s.trim().is_empty()).cloned()
}

/// PERF-3 / TASK-1417 + TASK-1420: compute only those field specs the caller
/// will actually show. `show` returns `true` for every field id the caller
/// wants rendered; we skip the (potentially allocation-heavy)
/// `compose_*_value` / `format!` work for every other id.
fn shown_field_specs(id: &ProjectIdentity, show: &dyn Fn(&str) -> bool) -> Vec<FieldSpec> {
    // Field order is the canonical render order — keep this in sync with
    // BASE_ABOUT_FIELDS / extension overrides; the renderer trusts this order.
    let ids: [&'static str; 8] = [
        "project",
        "stack",
        "license",
        "modules",
        "codebase",
        "dependencies",
        "repository",
        "homepage",
    ];
    ids.iter()
        .copied()
        .filter(|fid| show(fid))
        .map(|fid| match fid {
            "project" => FieldSpec {
                label: "project".into(),
                value: compose_project_value(id),
            },
            "stack" => FieldSpec {
                label: "stack".into(),
                value: compose_stack_value(id),
            },
            "license" => FieldSpec {
                label: "license".into(),
                value: non_empty_clone(&id.license),
            },
            "modules" => FieldSpec {
                label: id.module_label.clone(),
                value: id.module_count.map(|c| c.to_string()),
            },
            "codebase" => FieldSpec {
                label: "codebase".into(),
                value: compose_codebase_value(id),
            },
            "dependencies" => FieldSpec {
                label: "dependencies".into(),
                value: id.dependency_count.filter(|&c| c > 0).map(|c| {
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
            },
            "repository" => FieldSpec {
                label: "repository".into(),
                value: non_empty_clone(&id.repository),
            },
            "homepage" => FieldSpec {
                label: "homepage".into(),
                value: non_empty_clone(&id.homepage),
            },
            _ => unreachable!("ids array out of sync with match arms"),
        })
        .collect()
}

fn push_special_fields(
    fields: &mut Vec<(String, String)>,
    id: &ProjectIdentity,
    show: &dyn Fn(&str) -> bool,
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
        // PERF-3 / TASK-1420: hash the filter set once so every per-field
        // `show()` check is O(1) instead of O(N) linear scan. Mirrors the
        // already-closed TASK-1332 pattern in about_cmd.
        let visible: Option<HashSet<&str>> =
            visible_fields.map(|f| f.iter().map(String::as_str).collect());
        let show = |field_id: &str| -> bool {
            match &visible {
                None => true,
                Some(set) => set.contains(field_id),
            }
        };

        let mut fields: Vec<(String, String)> = shown_field_specs(id, &show)
            .into_iter()
            .filter_map(|spec| spec.value.map(|v| (spec.label, v)))
            .collect();

        push_special_fields(&mut fields, id, &show, visible_fields.is_some());

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
            // SEC-21 / TASK-1427: descriptions come from Cargo.toml /
            // package.json metadata, which is attacker-controlled under a
            // hostile workspace. Strip ANSI/control bytes before stdout.
            lines.push(String::new());
            lines.push(format!("  {}", sanitised(desc)));
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
///
/// TRAIT-1 / TASK-1435: derives `Debug` / `Clone` (C-DEBUG) for symmetry
/// with the built [`AboutCard`].
#[derive(Debug, Clone, Default)]
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

// READ-2 / TASK-1407: name the prefix column structure used by `render_field`
// so `continuation_indent` is no longer an opaque arithmetic expression.
// Layout of each rendered row is:
//   `"  " (LEADING) + emoji (EMOJI_COLS) + " " (KEY_SEP) + padded_key
//      (max_key_len + KEY_PAD) + " " (VALUE_SEP) + value`
const LEADING_COLS: usize = 2;
const EMOJI_COLS: usize = 2;
const KEY_SEP_COLS: usize = 1;
const KEY_PAD_COLS: usize = 2;
const VALUE_SEP_COLS: usize = 1;

/// Number of leading spaces to align value continuation lines under the
/// first value line. Derived from the named column constants used by
/// [`render_field`] so a future column-width tweak lands in one place.
fn continuation_indent(max_key_len: usize) -> String {
    " ".repeat(
        LEADING_COLS + EMOJI_COLS + KEY_SEP_COLS + (max_key_len + KEY_PAD_COLS) + VALUE_SEP_COLS,
    )
}

/// SEC-21 / TASK-1427: route one line of attacker-controlled text through
/// the shared `ui::sanitise_line` defence (escapes ESC/control bytes) before
/// it reaches stdout.
fn sanitised(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    sanitise_line(line, &mut out);
    out
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
    let styled = |s: &str| dim_gated(s, is_tty).into_owned();
    let emoji = field_emoji(key, value);
    let mut value_lines = value.split('\n');
    let first = value_lines.next().unwrap_or("");
    // DUP-3 / TASK-1390: route through the shared pad helper so a future
    // tightening of width-aware padding lands once.
    let padded_key = pad_to_display_width(key, max_key_len + KEY_PAD_COLS);
    let mut out = vec![format!(
        "  {} {} {}",
        emoji,
        padded_key,
        styled(&sanitised(first))
    )];
    for cont in value_lines {
        out.push(format!("{}{}", cont_indent, styled(&sanitised(cont))));
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

    /// PERF-3 / TASK-1391: the non-empty-clone helper must treat
    /// whitespace-only `Option<String>` as `None` so a future tightening of
    /// the empty-field semantics lands in one place.
    #[test]
    fn non_empty_clone_treats_whitespace_as_none() {
        assert_eq!(non_empty_clone(&None), None);
        assert_eq!(non_empty_clone(&Some(String::new())), None);
        assert_eq!(non_empty_clone(&Some("   ".to_string())), None);
        assert_eq!(non_empty_clone(&Some("\t\n".to_string())), None);
        assert_eq!(
            non_empty_clone(&Some("foo".to_string())),
            Some("foo".to_string())
        );
    }

    /// PERF-3 / TASK-1417: filtered card rendering must skip computing
    /// field values for ids not in `visible_fields`. The `match`-by-id
    /// dispatch in [`shown_field_specs`] structurally enforces this; this
    /// test pins the observable side: rendering for `["project"]` yields
    /// only the project row, even when other fields would be present.
    #[test]
    fn filtered_card_skips_codebase_when_only_project_requested() {
        let id = ProjectIdentity {
            name: "ops".into(),
            stack_label: "Rust".into(),
            project_path: "/p".into(),
            module_label: "crates".into(),
            loc: Some(1_000),
            file_count: Some(10),
            languages: vec![],
            ..Default::default()
        };
        let card = AboutCard::from_identity_filtered(&id, Some(&["project".to_string()]));
        let ids: Vec<&str> = card.fields.iter().map(|(k, _)| k.as_str()).collect();
        assert_eq!(ids, vec!["project"], "only project should be rendered");
    }

    /// SEC-21 / TASK-1427: attacker-controlled identity fields containing
    /// ESC / control bytes must not reach stdout verbatim. The card render
    /// path routes every value line through `ui::sanitise_line`.
    #[test]
    fn render_strips_ansi_escapes_from_field_values() {
        let id = ProjectIdentity {
            name: "ops\x1b[2Jhostile".into(),
            stack_label: "Rust".into(),
            project_path: "/p".into(),
            module_label: "crates".into(),
            description: Some("desc\x1b[31m".into()),
            ..Default::default()
        };
        let out = AboutCard::from_identity(&id).render(false);
        assert!(
            !out.contains('\x1b'),
            "raw ESC must not reach rendered output: {out:?}"
        );
        assert!(out.contains("\\x1b"), "ESC should be hex-escaped: {out:?}");
    }
}
