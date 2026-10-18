//! Formatting and composition helpers for `ProjectIdentity` rendering.
//!
//! These helpers translate identity fields into emoji-prefixed, multi-line
//! text blocks used by [`super::AboutCard`].

use super::{region_display, ProjectIdentity, RegionStat};
use crate::output::{display_width, pad_to_display_width};
use crate::text::format_number;

/// Push `s` onto `parts` iff non-empty, so the
/// non-empty-then-push idiom lives in one place.
fn push_non_empty(parts: &mut Vec<String>, s: &str) {
    if !s.is_empty() {
        parts.push(s.to_string());
    }
}

/// `Option`-aware sibling of [`push_non_empty`].
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
/// Delegates to [`language_emoji`] for the shared
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

/// Canonical language-to-glyph mapping, consumed by [`stack_emoji`] for the
/// stack field. Falls back to a generic document icon when no specific
/// mapping is known.
// "Markdown" resolves to the same glyph as the fallback, but the explicit arm
// records that it is a deliberate choice rather than a missing mapping.
#[allow(clippy::match_same_arms)]
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

/// Per-region glyph for the codebase breakdown, keyed by the display name
/// [`region_display`] returns. Unrecognised regions fall back to the ▸
/// generic, matching [`field_emoji`]'s fallback.
fn region_emoji(name: &str) -> &'static str {
    match name {
        "production" => "\u{1f3ed}", // 🏭
        "test" => "\u{1f9ea}",       // 🧪
        "example" => "\u{1f4d6}",    // 📖
        _ => "\u{25b8}",             // ▸ fallback
    }
}

/// Render the production / test / example split: a header with the summed
/// code lines, then one line per region with its code lines, share of the
/// total, and file count. Regions render in [`REGION_ORDER`](super::REGION_ORDER)
/// order regardless of the input order.
///
/// The header total is computed from the regions themselves so it always
/// agrees with the lines under it. The share clause is omitted when the
/// total is zero — every percentage would be a division by zero dressed up
/// as `0.0%` (the same guard `about loc`'s totals line applies).
fn format_region_breakdown(regions: &[RegionStat]) -> Vec<String> {
    let mut sorted: Vec<&RegionStat> = regions.iter().collect();
    sorted.sort_by(|a, b| {
        region_display(&a.region)
            .0
            .cmp(&region_display(&b.region).0)
            .then_with(|| a.region.cmp(&b.region))
    });

    let total_code: i64 = sorted.iter().map(|r| r.code).sum();
    let mut lines = vec![format!("{} loc", format_number(total_code))];

    // Align by display width, not byte length — an unknown region keeps its
    // raw key verbatim, so a future non-ASCII key would otherwise misalign
    // the column. Pattern mirrors theme_cmd / help.rs.
    let name_width = sorted
        .iter()
        .map(|r| display_width(region_display(&r.region).1))
        .max()
        .unwrap_or(0);
    let code_width = sorted
        .iter()
        .map(|r| format_number(r.code).len())
        .max()
        .unwrap_or(0);

    for region in sorted {
        let (_, name) = region_display(&region.region);
        let share = if total_code > 0 {
            // `i64 -> f64` has no `From` impl, so these two casts stay.
            // Line counts never approach f64's 2^53 exact-integer range
            // (~9e15 lines), so both conversions are exact, and the result
            // is rendered to one decimal place anyway.
            #[allow(clippy::cast_precision_loss, clippy::as_conversions)]
            let pct = region.code as f64 * 100.0 / total_code as f64;
            format!(" ({pct:.1}%)")
        } else {
            String::new()
        };
        // Pad the glyph to a fixed two-cell slot (the same `EMOJI_COLS`
        // contract as `card::render_field`) so the width-1 ▸ fallback for
        // an unknown region does not shift that row's columns; pad the
        // name through the shared helper so a future tightening
        // (tab/ZWJ handling) lands once.
        let padded_emoji = pad_to_display_width(region_emoji(name), 2);
        let padded_name = pad_to_display_width(name, name_width);
        lines.push(format!(
            "  {} {}  {:>code_w$}{share} \u{b7} {} file{}",
            padded_emoji,
            padded_name,
            format_number(region.code),
            format_number(region.files),
            if region.files == 1 { "" } else { "s" },
            code_w = code_width,
        ));
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
        parts.push(format!("{msrv} (msrv)"));
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
        parts.push(format!("v{v}"));
    }
    push_non_empty(&mut parts, &id.project_path);
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

/// Compose the `codebase` field value. With a region split available the
/// value is that split — total code lines, one line per region, then the
/// distinct file count — matching the `about loc` page's data. Without one
/// (every non-Rust stack: `rust-loc` only registers on Rust) the value is
/// bare totals. Returns `None` when none of the inputs are present.
pub(super) fn compose_codebase_value(id: &ProjectIdentity) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    if id.regions.is_empty() {
        if let Some(loc) = id.loc {
            parts.push(format!("{} loc", format_number(loc)));
        }
    } else {
        parts.extend(format_region_breakdown(&id.regions));
    }
    if let Some(f) = id.file_count {
        if f > 0 {
            parts.push(format!(
                "{} file{}",
                format_number(f),
                if f == 1 { "" } else { "s" }
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

    /// READ-5 / TASK-1187 (ported from the language breakdown): the region
    /// name column must align by display width, so a non-ASCII unknown
    /// region key lines up under an ASCII sibling at the same terminal
    /// column.
    #[test]
    fn region_breakdown_aligns_non_ascii_names_by_display_width() {
        let regions = vec![
            RegionStat::new("日本語", 1, 100),
            RegionStat::new("main", 1, 100),
        ];
        let lines = format_region_breakdown(&regions);
        assert_eq!(lines.len(), 3);

        // The numeric value column starts after "  <emoji> <name><pad>  "; using
        // display_width on the prefix preceding the value gives the column
        // position. Both rows must put "100" at the same column.
        let col = |line: &str| -> usize {
            let idx = line
                .find(|c: char| c.is_ascii_digit())
                .expect("value digit present");
            display_width(line.get(..idx).expect("find returns a char boundary"))
        };
        assert_eq!(col(&lines[1]), col(&lines[2]));
    }

    /// The split renders in canonical production / test / example order
    /// whatever order the input arrives in, with each region's code lines,
    /// share of the summed total, and file count.
    #[test]
    fn region_breakdown_sorts_canonically_and_renders_share() {
        let regions = vec![
            RegionStat::new("test", 5, 250),
            RegionStat::new("example", 2, 50),
            RegionStat::new("main", 10, 700),
        ];
        let lines = format_region_breakdown(&regions);
        // Canonical order is positional: the input arrived test, example,
        // main; the output must be production, test, example. The header
        // total is the sum of the regions, not any caller-supplied `loc`.
        assert_eq!(lines[0], "1,000 loc");
        assert!(lines[1].contains("production"), "got: {}", lines[1]);
        assert!(lines[1].contains("700 (70.0%)"), "got: {}", lines[1]);
        assert!(lines[1].contains("10 files"), "got: {}", lines[1]);
        assert!(lines[2].contains("test"), "got: {}", lines[2]);
        assert!(lines[2].contains("250 (25.0%)"), "got: {}", lines[2]);
        assert!(lines[3].contains("example"), "got: {}", lines[3]);
        assert!(lines[3].contains("50 (5.0%)"), "got: {}", lines[3]);
    }

    /// A zero-code split drops the share clause instead of printing `NaN%`
    /// or a row of meaningless zeroes — the same guard `about loc`'s totals
    /// line applies.
    #[test]
    fn region_breakdown_omits_shares_when_no_code() {
        let regions = vec![RegionStat::new("main", 2, 0)];
        let lines = format_region_breakdown(&regions);
        assert_eq!(lines[0], "0 loc");
        assert_eq!(lines.len(), 2);
        assert!(!lines[1].contains('%'), "no share clause: {}", lines[1]);
        assert!(!lines[1].contains("NaN"), "no NaN: {}", lines[1]);
    }
}
