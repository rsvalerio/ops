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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_crate_name_simple() {
        assert_eq!(format_crate_name("crate1"), "Crate1");
    }

    #[test]
    fn format_crate_name_with_path() {
        assert_eq!(format_crate_name("crates/aggregate"), "Aggregate");
    }

    #[test]
    fn format_crate_name_with_glob_prefix() {
        assert_eq!(format_crate_name("**/my-crate"), "My-crate");
    }

    #[test]
    fn format_crate_name_nested_path() {
        assert_eq!(format_crate_name("workspace/crates/my-lib"), "My-lib");
    }

    #[test]
    fn format_crate_name_empty() {
        assert_eq!(format_crate_name(""), "");
    }

    #[test]
    fn render_card_with_loc() {
        let info = CrateInfo {
            name: "My-lib".to_string(),
            package_name: "ops-my-lib".to_string(),
            path: "crates/my-lib".to_string(),
            version: Some("0.1.0".to_string()),
            description: Some("A shared library".to_string()),
            loc: Some(4231),
            file_count: None,
            dep_count: None,
        };
        let card = render_card(&info, false);
        assert!(
            card[3].contains("4,231 loc"),
            "card line 3 should contain LOC: {:?}",
            card[3]
        );
    }

    #[test]
    fn render_card_without_loc() {
        let info = CrateInfo {
            name: "My-lib".to_string(),
            package_name: "ops-my-lib".to_string(),
            path: "crates/my-lib".to_string(),
            version: Some("0.1.0".to_string()),
            description: Some("A shared library".to_string()),
            loc: None,
            file_count: None,
            dep_count: None,
        };
        let card = render_card(&info, false);
        let inner = &card[3][3..card[3].len() - 3];
        assert!(
            inner.trim().is_empty(),
            "card line 3 should be empty spacer: {:?}",
            card[3]
        );
    }

    #[test]
    fn render_card_with_loc_and_deps() {
        let info = CrateInfo {
            name: "My-lib".to_string(),
            package_name: "ops-my-lib".to_string(),
            path: "crates/my-lib".to_string(),
            version: Some("0.1.0".to_string()),
            description: Some("A shared library".to_string()),
            loc: Some(4231),
            file_count: None,
            dep_count: Some(12),
        };
        let card = render_card(&info, false);
        assert!(
            card[3].contains("4,231 loc") && card[3].contains("12 deps"),
            "card line 3 should contain LOC and deps: {:?}",
            card[3]
        );
        assert!(
            card[3].contains("\u{00b7}"),
            "card line 3 should contain middle dot separator: {:?}",
            card[3]
        );
    }

    #[test]
    fn render_card_with_deps_only() {
        let info = CrateInfo {
            name: "My-lib".to_string(),
            package_name: "ops-my-lib".to_string(),
            path: "crates/my-lib".to_string(),
            version: Some("0.1.0".to_string()),
            description: Some("A shared library".to_string()),
            loc: None,
            file_count: None,
            dep_count: Some(5),
        };
        let card = render_card(&info, false);
        assert!(
            card[3].contains("5 deps"),
            "card line 3 should contain deps: {:?}",
            card[3]
        );
        assert!(
            !card[3].contains("loc"),
            "card line 3 should not contain loc: {:?}",
            card[3]
        );
    }

    #[test]
    fn build_card_stats_line_none_when_empty() {
        let info = CrateInfo {
            name: "test".to_string(),
            package_name: "test".to_string(),
            path: "test".to_string(),
            version: None,
            description: None,
            loc: None,
            file_count: None,
            dep_count: None,
        };
        assert!(build_card_stats_line(&info).is_none());
    }

    #[test]
    fn build_card_stats_line_loc_only() {
        let info = CrateInfo {
            name: "test".to_string(),
            package_name: "test".to_string(),
            path: "test".to_string(),
            version: None,
            description: None,
            loc: Some(100),
            file_count: None,
            dep_count: None,
        };
        assert_eq!(build_card_stats_line(&info).unwrap(), "100 loc");
    }

    #[test]
    fn build_card_stats_line_file_count_singular() {
        let info = CrateInfo {
            name: "test".to_string(),
            package_name: "test".to_string(),
            path: "test".to_string(),
            version: None,
            description: None,
            loc: None,
            file_count: Some(1),
            dep_count: None,
        };
        assert_eq!(build_card_stats_line(&info).unwrap(), "1 file");
    }

    #[test]
    fn build_card_stats_line_file_count_plural() {
        let info = CrateInfo {
            name: "test".to_string(),
            package_name: "test".to_string(),
            path: "test".to_string(),
            version: None,
            description: None,
            loc: None,
            file_count: Some(5),
            dep_count: None,
        };
        assert_eq!(build_card_stats_line(&info).unwrap(), "5 files");
    }

    #[test]
    fn build_card_stats_line_all_fields() {
        let info = CrateInfo {
            name: "test".to_string(),
            package_name: "test".to_string(),
            path: "test".to_string(),
            version: None,
            description: None,
            loc: Some(1000),
            file_count: Some(10),
            dep_count: Some(3),
        };
        let result = build_card_stats_line(&info).unwrap();
        assert!(result.contains("1,000 loc"));
        assert!(result.contains("10 files"));
        assert!(result.contains("3 deps"));
        assert!(result.contains("\u{00b7}"));
    }

    #[test]
    fn render_card_no_version() {
        let info = CrateInfo {
            name: "My-lib".to_string(),
            package_name: "my-lib".to_string(),
            path: "crates/my-lib".to_string(),
            version: None,
            description: None,
            loc: None,
            file_count: None,
            dep_count: None,
        };
        let card = render_card(&info, false);
        assert!(card[1].contains("My-lib"));
        assert!(!card[1].contains(" v"));
    }

    #[test]
    fn render_card_long_title_truncated() {
        let info = CrateInfo {
            name: "A".repeat(40),
            package_name: "long".to_string(),
            path: "crates/long".to_string(),
            version: Some("1.0.0".to_string()),
            description: None,
            loc: None,
            file_count: None,
            dep_count: None,
        };
        let card = render_card(&info, false);
        assert!(card[1].contains("\u{2026}"));
    }

    #[test]
    fn render_card_long_path_truncated() {
        let info = CrateInfo {
            name: "Short".to_string(),
            package_name: "short".to_string(),
            path: "very/deeply/nested/path/that/exceeds/card/width".to_string(),
            version: None,
            description: None,
            loc: None,
            file_count: None,
            dep_count: None,
        };
        let card = render_card(&info, false);
        assert!(card[2].contains("\u{2026}"));
    }

    #[test]
    fn render_card_with_description() {
        let info = CrateInfo {
            name: "Test".to_string(),
            package_name: "test".to_string(),
            path: "test".to_string(),
            version: Some("1.0.0".to_string()),
            description: Some("A test crate".to_string()),
            loc: None,
            file_count: None,
            dep_count: None,
        };
        let card = render_card(&info, false);
        assert!(card[4].contains("A test crate"));
    }

    #[test]
    fn render_card_line_count() {
        let info = CrateInfo {
            name: "Test".to_string(),
            package_name: "test".to_string(),
            path: "test".to_string(),
            version: None,
            description: None,
            loc: None,
            file_count: None,
            dep_count: None,
        };
        let card = render_card(&info, false);
        assert_eq!(card.len(), 8);
    }

    #[test]
    fn render_card_with_file_count() {
        let info = CrateInfo {
            name: "Test".to_string(),
            package_name: "test".to_string(),
            path: "test".to_string(),
            version: None,
            description: None,
            loc: Some(500),
            file_count: Some(3),
            dep_count: None,
        };
        let card = render_card(&info, false);
        assert!(
            card[3].contains("500 loc") && card[3].contains("3 files"),
            "stats line: {:?}",
            card[3]
        );
    }

    #[test]
    fn layout_cards_empty() {
        let result = layout_cards_in_grid(&[]);
        assert!(result.is_empty());
    }

    #[test]
    fn layout_cards_single() {
        let card = vec!["line1".to_string(), "line2".to_string()];
        let result = layout_cards_in_grid(&[card]);
        assert!(result.iter().any(|l| l.contains("line1")));
    }

    #[test]
    fn layout_cards_multiple_cards() {
        let card1 = vec!["a1".to_string(), "a2".to_string()];
        let card2 = vec!["b1".to_string(), "b2".to_string()];
        let result = layout_cards_in_grid(&[card1, card2]);
        assert!(!result.is_empty());
        let joined = result.join("\n");
        assert!(joined.contains("a1"));
        assert!(joined.contains("b1"));
    }

    #[test]
    fn resolve_crate_display_name_with_toml() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("crates/my-lib")).unwrap();
        std::fs::write(
            root.join("crates/my-lib/Cargo.toml"),
            "[package]\nname = \"ops-my-lib\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let name = resolve_crate_display_name("crates/my-lib", root);
        assert_eq!(name, "ops-my-lib");
    }

    #[test]
    fn resolve_crate_display_name_missing_toml() {
        let dir = tempfile::tempdir().unwrap();
        let name = resolve_crate_display_name("crates/nonexistent", dir.path());
        assert_eq!(name, "Nonexistent");
    }

    #[test]
    fn resolve_crate_display_name_no_package_section() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("crates/ws")).unwrap();
        std::fs::write(
            root.join("crates/ws/Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();
        let name = resolve_crate_display_name("crates/ws", root);
        assert_eq!(name, "Ws");
    }

    #[test]
    fn load_crate_infos_reads_metadata() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname = \"my-foo\"\nversion = \"0.2.0\"\ndescription = \"A foo crate\"\n",
        )
        .unwrap();

        let infos = load_crate_infos(&["crates/foo"], root);
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].name, "Foo");
        assert_eq!(infos[0].package_name, "my-foo");
        assert_eq!(infos[0].version.as_deref(), Some("0.2.0"));
        assert_eq!(infos[0].description.as_deref(), Some("A foo crate"));
        assert_eq!(infos[0].path, "crates/foo");
    }

    #[test]
    fn load_crate_infos_missing_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        let infos = load_crate_infos(&["nonexistent"], root);
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].name, "Nonexistent");
        assert_eq!(infos[0].package_name, "");
        assert!(infos[0].version.is_none());
        assert!(infos[0].description.is_none());
    }

    #[test]
    fn load_crate_infos_malformed_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("crates/bad")).unwrap();
        std::fs::write(root.join("crates/bad/Cargo.toml"), "not valid toml {{{").unwrap();

        let infos = load_crate_infos(&["crates/bad"], root);
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].package_name, "");
        assert!(infos[0].version.is_none());
    }

    #[test]
    fn load_crate_infos_no_package_section() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("crates/ws")).unwrap();
        std::fs::write(
            root.join("crates/ws/Cargo.toml"),
            "[workspace]\nmembers = []\n",
        )
        .unwrap();

        let infos = load_crate_infos(&["crates/ws"], root);
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].package_name, "");
        assert!(infos[0].version.is_none());
        assert!(infos[0].description.is_none());
    }
}
