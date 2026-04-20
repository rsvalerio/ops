//! Formatting and composition helpers for `ProjectIdentity` rendering.
//!
//! These helpers translate identity fields into emoji-prefixed, multi-line
//! text blocks used by [`super::AboutCard`].

use super::{LanguageStat, ProjectIdentity};
use crate::text::format_number;

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
    let name_width = langs
        .iter()
        .take(top_n)
        .map(|l| short_language_name(&l.name).len())
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
            format!(
                "  {} {:<name_w$}  {:>val_w$} ({:.1}%)",
                language_emoji(&l.name),
                short_language_name(&l.name),
                format_number(value_fn(l)),
                pct_fn(l),
                name_w = name_width,
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
pub(super) fn compose_project_value(id: &ProjectIdentity) -> Option<String> {
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
