//! Formatting and composition helpers for `ProjectIdentity` rendering.
//!
//! These helpers translate identity fields into emoji-prefixed, multi-line
//! text blocks used by [`super::AboutCard`].

use super::{LanguageStat, ProjectIdentity};
use crate::output::{display_width, pad_to_display_width};
use crate::text::format_number;

/// READ-1 / TASK-1392: push `s` onto `parts` iff non-empty, so the
/// non-empty-then-push idiom lives in one place.
fn push_non_empty(parts: &mut Vec<String>, s: &str) {
    if !s.is_empty() {
        parts.push(s.to_string());
    }
}

/// READ-1 / TASK-1392: `Option`-aware sibling of [`push_non_empty`].
fn push_non_empty_opt(parts: &mut Vec<String>, opt: Option<&str>) {
    if let Some(s) = opt {
        push_non_empty(parts, s);
    }
}

/// Map a field (key, value) pair to its emoji prefix. The `stack` field is
/// value-aware so each language gets its own glyph.
pub(super) fn field_emoji(key: &str, value: &str) -> &'static str {
    match key {
        "stack" => stack_emoji(value),
        "license" => "\u{1f4dc}",         // 📜
        "project" => "\u{1f3f7}\u{fe0f}", // 🏷️
        "crates" | "packages" | "modules" | "subprojects" => "\u{1f4e6}", // 📦
        "codebase" => "\u{1f4dd}",        // 📝
        "author" | "authors" => "\u{1f464}", // 👤
        "repository" => "\u{1f517}",      // 🔗
        "homepage" => "\u{1f310}",        // 🌐
        "dependencies" => "\u{1f9e9}",    // 🧩
        "coverage" => "\u{1f9ea}",        // 🧪
        _ => "\u{25b8}",                  // ▸ fallback
    }
}

/// Language-specific emoji derived from the stack label (first token of the
/// `stack` field value, e.g. `"Rust · Edition 2021"` → `"Rust"` → 🦀).
///
/// DUP-3 / TASK-0983: delegates to [`language_emoji`] for the shared
/// language-to-glyph mapping. Two cases are deliberately divergent:
///
/// 1. **Node / JavaScript**: the stack field renders the Node ecosystem
///    glyph (⬢) — a runtime / package-manager identity, since the stack
///    line describes *which platform built this*. The codebase breakdown
///    renders the language glyph (🟨) for JavaScript files. These are two
///    different rendering intents on purpose.
/// 2. **Generic fallback**: 📚 ("multi-language project") on the stack
///    line vs 📄 ("plain document") in the codebase breakdown.
fn stack_emoji(value: &str) -> &'static str {
    let label = value.split_whitespace().next().unwrap_or("");
    match label {
        "Node" | "JavaScript" => "\u{2b22}", // ⬢ runtime/platform glyph
        // Defer to the canonical language table so adding a new language
        // is a one-line edit on language_emoji rather than a two-table
        // change. The fallback below handles the stack-specific generic.
        other => match language_emoji(other) {
            // language_emoji's generic fallback is 📄; the stack line
            // wants 📚 for "multi-language project" — translate it here.
            "\u{1f4c4}" => "\u{1f4da}",
            glyph => glyph,
        },
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

/// Per-language glyph for the codebase breakdown. Falls back to a generic
/// document icon when no specific mapping is known.
fn language_emoji(name: &str) -> &'static str {
    match name {
        "Rust" => "\u{1f980}",                                   // 🦀
        "Go" => "\u{1f439}",                                     // 🐹
        "Python" => "\u{1f40d}",                                 // 🐍
        "Java" => "\u{2615}",                                    // ☕
        "JavaScript" | "Node" => "\u{1f7e8}",                    // 🟨
        "TypeScript" => "\u{1f537}",                             // 🔷
        "Ruby" => "\u{1f48e}",                                   // 💎
        "Shell" | "Bash" | "Zsh" | "Fish" | "Sh" => "\u{1f41a}", // 🐚
        "HTML" => "\u{1f310}",                                   // 🌐
        "CSS" | "SCSS" | "Sass" | "Less" => "\u{1f3a8}",         // 🎨
        "SVG" => "\u{1f5bc}\u{fe0f}",                            // 🖼️
        "Markdown" => "\u{1f4c4}",                               // 📄
        "YAML" => "\u{1f9fe}",                                   // 🧾
        "TOML" => "\u{1f527}",                                   // 🔧
        "JSON" => "\u{1f4cb}",                                   // 📋
        "XML" => "\u{1f9be}",                                    // 🦾 (loose)
        "SQL" => "\u{1f5c4}\u{fe0f}",                            // 🗄️
        "Dockerfile" | "Docker" => "\u{1f433}",                  // 🐳
        "Makefile" | "CMake" => "\u{1f6e0}\u{fe0f}",             // 🛠️
        "Handlebars" => "\u{1fa84}",                             // 🪄
        "Protocol Buffers" => "\u{1f4e6}",                       // 📦
        "Terraform" | "HCL" => "\u{1f4a0}",                      // 💠
        "Ansible" => "\u{1f170}\u{fe0f}",                        // 🅰️
        "Kotlin" => "\u{1f7e3}",                                 // 🟣
        "Swift" => "\u{1f426}",                                  // 🐦
        "C" | "C++" | "C Header" => "\u{1f52c}",                 // 🔬 (loose)
        "C#" => "\u{1f3b5}",                                     // 🎵 (loose play on sharp)
        _ => "\u{1f4c4}",                                        // 📄 generic
    }
}

/// Render per-language breakdown lines for a single metric. Keeps top-N and
/// rolls the rest into a "+N more" line. `value_fn` extracts either loc or
/// file count, `pct_fn` extracts the matching percentage.
fn format_language_breakdown(
    langs: &[LanguageStat],
    top_n: usize,
    value_fn: impl Fn(&LanguageStat) -> i64,
    pct_fn: impl Fn(&LanguageStat) -> f64,
) -> Vec<String> {
    if langs.is_empty() {
        return Vec::new();
    }
    // READ-5 (TASK-1187): align by display width, not byte length —
    // `short_language_name` falls back to the original name verbatim, so a
    // future non-ASCII LanguageStat would otherwise misalign the column.
    // Pattern mirrors theme_cmd / tools_cmd / help.rs.
    let name_width = langs
        .iter()
        .take(top_n)
        .map(|l| display_width(short_language_name(&l.name)))
        .max()
        .unwrap_or(0);
    let value_width = langs
        .iter()
        .take(top_n)
        .map(|l| format_number(value_fn(l)).len())
        .max()
        .unwrap_or(0);
    let mut lines: Vec<String> = langs
        .iter()
        .take(top_n)
        .map(|l| {
            // DUP-3 / TASK-1390: route through the shared pad helper so a
            // future tightening (tab/ZWJ handling) lands once.
            let padded_name = pad_to_display_width(short_language_name(&l.name), name_width);
            format!(
                "  {} {}  {:>val_w$} ({:.1}%)",
                language_emoji(&l.name),
                padded_name,
                format_number(value_fn(l)),
                pct_fn(l),
                val_w = value_width,
            )
        })
        .collect();
    if langs.len() > top_n {
        lines.push(format!("  (+{} more)", langs.len() - top_n));
    }
    lines
}

/// Compose the multi-line `stack` field value: language label, optional
/// stack detail (e.g. "Edition 2021"), and optional MSRV.
pub(super) fn compose_stack_value(id: &ProjectIdentity) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    push_non_empty(&mut parts, &id.stack_label);
    push_non_empty_opt(&mut parts, id.stack_detail.as_deref());
    if let Some(msrv) = id.msrv.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("{} (msrv)", msrv));
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

/// Compose the multi-line `project` field value: name, optional version
/// line, and project path.
pub(super) fn compose_project_value(id: &ProjectIdentity) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    push_non_empty(&mut parts, &id.name);
    if let Some(v) = id.version.as_deref().filter(|s| !s.is_empty()) {
        parts.push(format!("v{}", v));
    }
    push_non_empty(&mut parts, &id.project_path);
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

/// Compose the `codebase` field value as two blocks: total LOC with a
/// per-language breakdown, then total file count with a per-language
/// breakdown. Returns `None` when none of the inputs are present.
pub(super) fn compose_codebase_value(id: &ProjectIdentity) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if let Some(loc) = id.loc {
        parts.push(format!("{} loc", format_number(loc)));
        parts.extend(format_language_breakdown(
            &id.languages,
            3,
            |l| l.loc,
            |l| l.loc_pct,
        ));
    }
    if let Some(f) = id.file_count {
        if f > 0 {
            parts.push(format!(
                "{} file{}",
                format_number(f),
                if f != 1 { "s" } else { "" }
            ));
            parts.extend(format_language_breakdown(
                &id.languages,
                3,
                |l| l.files,
                |l| l.files_pct,
            ));
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DUP-3 / TASK-0983: shared language entries must agree across
    /// `stack_emoji` and `language_emoji`. The Node/JavaScript case and
    /// the generic fallback are deliberately divergent and asserted
    /// explicitly below.
    #[test]
    fn shared_languages_agree_across_stack_and_codebase() {
        for lang in ["Rust", "Go", "Python", "Java", "Terraform", "Ansible"] {
            assert_eq!(
                stack_emoji(lang),
                language_emoji(lang),
                "stack_emoji and language_emoji must agree on `{lang}`"
            );
        }
    }

    #[test]
    fn node_renders_runtime_glyph_in_stack_and_language_glyph_in_codebase() {
        assert_eq!(stack_emoji("Node"), "\u{2b22}"); // ⬢ runtime
        assert_eq!(stack_emoji("JavaScript"), "\u{2b22}");
        assert_eq!(language_emoji("JavaScript"), "\u{1f7e8}"); // 🟨 language
    }

    #[test]
    fn fallback_glyphs_diverge_intentionally() {
        // Generic stack: 📚 (multi-language project)
        assert_eq!(stack_emoji("Brainfuck"), "\u{1f4da}");
        // Generic codebase: 📄 (plain document)
        assert_eq!(language_emoji("Brainfuck"), "\u{1f4c4}");
    }

    /// READ-5 / TASK-1187: language column must align by display width, so a
    /// non-ASCII fallback name lines up under an ASCII sibling at the same
    /// terminal column.
    #[test]
    fn language_breakdown_aligns_non_ascii_names_by_display_width() {
        let langs = vec![
            LanguageStat::new("日本語", 100, 1, 50.0, 50.0),
            LanguageStat::new("Rust", 100, 1, 50.0, 50.0),
        ];
        let lines = format_language_breakdown(&langs, 5, |l| l.loc, |l| l.loc_pct);
        assert_eq!(lines.len(), 2);

        // The numeric value column starts after "  <emoji> <name><pad>  "; using
        // display_width on the prefix preceding the value gives the column
        // position. Both rows must put "100" at the same column.
        let col = |line: &str| -> usize {
            let idx = line
                .find(|c: char| c.is_ascii_digit())
                .expect("value digit present");
            display_width(&line[..idx])
        };
        assert_eq!(col(&lines[0]), col(&lines[1]));
    }
}
