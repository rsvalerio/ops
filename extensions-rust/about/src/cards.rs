//! Card rendering and grid layout for crate info display.

use ops_core::output::display_width;
use ops_core::style::{cyan, dim, grey, white};

use super::text_util::{
    format_number, get_terminal_width, pad_to_width_plain, truncate_to_width, tty_style, wrap_text,
};

/// Layout constants for the about dashboard.
pub(crate) struct CardLayoutConfig;

impl CardLayoutConfig {
    /// Width of the header box in characters.
    pub(crate) const BOX_WIDTH: usize = 100;
    /// Width of each crate card in characters.
    const CARD_WIDTH: usize = 32;
    /// Maximum lines for description in a card.
    const CARD_DESC_LINES: usize = 3;
    /// Spacing between cards horizontally.
    const CARD_SPACING: usize = 2;
    /// Minimum terminal width to show 3 cards per row.
    const MIN_WIDTH_3_CARDS: usize = 105;
    /// Minimum terminal width to show 2 cards per row.
    const MIN_WIDTH_2_CARDS: usize = 70;
}

pub(crate) struct CrateInfo {
    pub(crate) name: String,
    pub(crate) package_name: String,
    pub(crate) path: String,
    pub(crate) version: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) loc: Option<i64>,
    pub(crate) file_count: Option<i64>,
    pub(crate) dep_count: Option<i64>,
}

pub(crate) fn format_crate_name(member: &str) -> String {
    let name = member
        .strip_prefix("**/")
        .unwrap_or(member)
        .split('/')
        .next_back()
        .unwrap_or(member);

    let mut chars = name.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
    }
}

/// Resolve the display name for a workspace member.
///
/// Reads the package name from the member's Cargo.toml if `workspace_root` is
/// provided, falling back to [`format_crate_name`] when the file is missing or
/// has no `[package]` section.
pub(crate) fn resolve_crate_display_name(member: &str, workspace_root: &std::path::Path) -> String {
    let toml_path = workspace_root.join(member).join("Cargo.toml");
    let (pkg_name, _, _) = read_crate_metadata(&toml_path);
    pkg_name.unwrap_or_else(|| format_crate_name(member))
}

pub(crate) fn load_crate_infos(
    members: &[&str],
    workspace_root: &std::path::Path,
) -> Vec<CrateInfo> {
    members
        .iter()
        .map(|member| {
            let crate_path = workspace_root.join(member).join("Cargo.toml");
            let (pkg_name, version, description) = read_crate_metadata(&crate_path);

            CrateInfo {
                name: format_crate_name(member),
                package_name: pkg_name.unwrap_or_default(),
                path: member.to_string(),
                version,
                description,
                loc: None,
                file_count: None,
                dep_count: None,
            }
        })
        .collect()
}

/// Read package name, version, and description from a crate's Cargo.toml.
fn read_crate_metadata(
    crate_toml_path: &std::path::Path,
) -> (Option<String>, Option<String>, Option<String>) {
    use std::fs;

    let content = match fs::read_to_string(crate_toml_path) {
        Ok(c) => c,
        Err(_) => return (None, None, None),
    };

    let parsed: Result<toml::Value, _> = toml::from_str(&content);
    let parsed = match parsed {
        Ok(p) => p,
        Err(_) => return (None, None, None),
    };

    let package = parsed.get("package");

    let name = package
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(|s| s.to_string());

    let version = package
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let description = package
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    (name, version, description)
}

pub(crate) fn build_card_stats_line(info: &CrateInfo) -> Option<String> {
    let parts: Vec<String> = [
        info.loc.map(|loc| format!("{} loc", format_number(loc))),
        info.file_count
            .map(|f| format!("{} file{}", format_number(f), if f != 1 { "s" } else { "" })),
        info.dep_count
            .map(|deps| format!("{} deps", format_number(deps))),
    ]
    .into_iter()
    .flatten()
    .collect();

    if parts.is_empty() {
        None
    } else {
        Some(parts.join(" \u{00b7} "))
    }
}

pub(crate) fn render_card(info: &CrateInfo, is_tty: bool) -> Vec<String> {
    let inner_width = CardLayoutConfig::CARD_WIDTH - 2;

    let title = if let Some(ref v) = info.version {
        format!("{} v{}", info.name, v)
    } else {
        info.name.clone()
    };

    let title_truncated = if display_width(&title) > inner_width {
        truncate_to_width(&title, inner_width)
    } else {
        title.clone()
    };

    let path_truncated = if display_width(&info.path) > inner_width {
        truncate_to_width(&info.path, inner_width)
    } else {
        info.path.clone()
    };

    let desc_lines = wrap_text(
        info.description.as_deref().unwrap_or(""),
        inner_width,
        CardLayoutConfig::CARD_DESC_LINES,
    );

    let top_border = format!("\u{256d}{}\u{256e}", "\u{2500}".repeat(inner_width));
    let bottom_border = format!("\u{2570}{}\u{256f}", "\u{2500}".repeat(inner_width));

    let mut lines = vec![top_border];
    lines.push(format!(
        "\u{2502}{}\u{2502}",
        tty_style(
            &pad_to_width_plain(&title_truncated, inner_width),
            cyan,
            is_tty
        )
    ));
    lines.push(format!(
        "\u{2502}{}\u{2502}",
        tty_style(
            &pad_to_width_plain(&path_truncated, inner_width),
            grey,
            is_tty
        )
    ));

    let empty_line = " ".repeat(inner_width);
    if let Some(stats_text) = build_card_stats_line(info) {
        lines.push(format!(
            "\u{2502}{}\u{2502}",
            tty_style(&pad_to_width_plain(&stats_text, inner_width), dim, is_tty)
        ));
    } else {
        lines.push(format!("\u{2502}{}\u{2502}", empty_line));
    }

    for i in 0..CardLayoutConfig::CARD_DESC_LINES {
        let desc_line = desc_lines.get(i).map(|s| s.as_str()).unwrap_or("");
        let content = if desc_line.is_empty() {
            empty_line.clone()
        } else {
            tty_style(&pad_to_width_plain(desc_line, inner_width), white, is_tty)
        };
        lines.push(format!("\u{2502}{}\u{2502}", content));
    }

    lines.push(bottom_border);
    lines
}

pub(crate) fn layout_cards_in_grid(cards: &[Vec<String>]) -> Vec<String> {
    if cards.is_empty() {
        return vec![];
    }

    let term_width = get_terminal_width();
    let cards_per_row = if term_width >= CardLayoutConfig::MIN_WIDTH_3_CARDS {
        3
    } else if term_width >= CardLayoutConfig::MIN_WIDTH_2_CARDS {
        2
    } else {
        1
    };

    let mut result = Vec::new();
    let spacing = " ".repeat(CardLayoutConfig::CARD_SPACING);

    for chunk in cards.chunks(cards_per_row) {
        let max_lines = chunk.iter().map(|c| c.len()).max().unwrap_or(0);

        for line_idx in 0..max_lines {
            let mut row_parts = Vec::new();
            for card in chunk {
                let line = card.get(line_idx).map(|s| s.as_str()).unwrap_or("");
                row_parts.push(line.to_string());
            }
            result.push(format!("  {}{}", row_parts.join(&spacing), spacing));
        }

        result.push(String::new());
    }

    if result.last().map(|s| s.is_empty()).unwrap_or(false) {
        result.pop();
    }

    result
}
