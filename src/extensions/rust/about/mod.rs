//! About command: displays workspace/project information in a formatted dashboard view.

use std::io::{self, IsTerminal};

use crate::extension::{Context, DataRegistry, Extension};
use crate::extensions::cargo_toml::CargoToml;
use crate::output::display_width;
use crate::style::dim;

/// Width of the header box in characters.
const BOX_WIDTH: usize = 100;

/// Width of each crate card in characters.
const CARD_WIDTH: usize = 32;

/// Maximum lines for description in a card.
const CARD_DESC_LINES: usize = 2;

/// Spacing between cards horizontally.
const CARD_SPACING: usize = 2;

/// Minimum terminal width to show 3 cards per row.
const MIN_WIDTH_3_CARDS: usize = 105;

/// Minimum terminal width to show 2 cards per row.
const MIN_WIDTH_2_CARDS: usize = 70;

struct CrateInfo {
    name: String,
    path: String,
    version: Option<String>,
    description: Option<String>,
}

pub fn run_about() -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = std::sync::Arc::new(crate::config::Config::default());
    let mut ctx = Context::new(config, cwd.clone());

    let exts = crate::extensions::builtin_extensions(&ctx.config, &ctx.working_directory)?;
    let ext_refs: Vec<&dyn Extension> = exts.iter().map(|b| b.as_ref()).collect();
    let mut data_registry = DataRegistry::new();
    crate::extensions::register_extension_data_providers(&ext_refs, &mut data_registry);

    let value = ctx.get_or_provide("cargo_toml", &data_registry)?;
    let manifest: CargoToml = serde_json::from_value((*value).clone())?;

    let output = format_about(&manifest, &cwd);
    println!("{}", output);

    Ok(())
}

fn format_about(manifest: &CargoToml, cwd: &std::path::Path) -> String {
    let mut lines = Vec::new();

    lines.extend(format_header(&manifest.package));
    lines.extend(format_description(&manifest.package));
    lines.extend(format_workspace_info(manifest, cwd));
    lines.extend(format_crates_section(manifest, cwd));

    lines.join("\n")
}

fn format_header(root_pkg: &Option<crate::extensions::cargo_toml::Package>) -> Vec<String> {
    let pkg_name = root_pkg
        .as_ref()
        .map(|p| p.name.as_str())
        .unwrap_or("workspace");
    let pkg_version = root_pkg
        .as_ref()
        .and_then(|p| p.version.as_str())
        .unwrap_or("unknown");
    let edition = root_pkg
        .as_ref()
        .and_then(|p| p.edition.as_str())
        .unwrap_or("2021");
    let license = root_pkg
        .as_ref()
        .and_then(|p| p.license.as_str())
        .unwrap_or("");

    let header_left = format!("🦀 {} v{}", pkg_name, pkg_version);
    let header_right = format!("Edition {} · {}", edition, license);
    let header_content = pad_header(&header_left, &header_right);

    vec![
        format!("┌{}┐", "─".repeat(BOX_WIDTH)),
        format!("│  {}│", header_content),
        format!("└{}┘", "─".repeat(BOX_WIDTH)),
    ]
}

fn format_description(root_pkg: &Option<crate::extensions::cargo_toml::Package>) -> Vec<String> {
    match root_pkg.as_ref().and_then(|p| p.description.as_str()) {
        Some(desc) => vec![String::new(), format!("  {}", desc)],
        None => vec![],
    }
}

fn format_workspace_info(manifest: &CargoToml, cwd: &std::path::Path) -> Vec<String> {
    let member_count = manifest
        .workspace
        .as_ref()
        .map(|w| w.members.len())
        .unwrap_or(0);

    let mut lines = vec![String::new()];
    lines.push(format!(
        "  ▸ workspace   {}",
        dim(&cwd.display().to_string())
    ));
    lines.push(format!(
        "  ▸ members     {} crate{}",
        dim(&member_count.to_string()),
        if member_count != 1 { "s" } else { "" }
    ));
    lines
}

fn format_crates_section(manifest: &CargoToml, workspace_root: &std::path::Path) -> Vec<String> {
    let members = match &manifest.workspace {
        Some(ws) if !ws.members.is_empty() => &ws.members,
        _ => return vec![],
    };

    let mut sorted_members: Vec<&str> = members.iter().map(|s| s.as_str()).collect();
    sorted_members.sort();

    let crate_infos = load_crate_infos(&sorted_members, workspace_root);
    let cards: Vec<Vec<String>> = crate_infos.iter().map(render_card).collect();

    let mut lines = vec![String::new(), "  CRATES".to_string(), String::new()];
    lines.extend(layout_cards_in_grid(&cards));

    lines
}

fn load_crate_infos(members: &[&str], workspace_root: &std::path::Path) -> Vec<CrateInfo> {
    members
        .iter()
        .map(|member| {
            let crate_path = workspace_root.join(member).join("Cargo.toml");
            let (version, description) = read_crate_metadata(&crate_path);

            CrateInfo {
                name: format_crate_name(member),
                path: member.to_string(),
                version,
                description,
            }
        })
        .collect()
}

fn read_crate_metadata(crate_toml_path: &std::path::Path) -> (Option<String>, Option<String>) {
    use std::fs;

    let content = match fs::read_to_string(crate_toml_path) {
        Ok(c) => c,
        Err(_) => return (None, None),
    };

    let parsed: Result<toml::Value, _> = toml::from_str(&content);
    let parsed = match parsed {
        Ok(p) => p,
        Err(_) => return (None, None),
    };

    let package = parsed.get("package");

    let version = package
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let description = package
        .and_then(|p| p.get("description"))
        .and_then(|d| d.as_str())
        .map(|s| s.to_string());

    (version, description)
}

fn format_crate_name(member: &str) -> String {
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

fn render_card(info: &CrateInfo) -> Vec<String> {
    let is_tty = io::stdout().is_terminal();
    let inner_width = CARD_WIDTH - 2;

    let title = if let Some(ref v) = info.version {
        format!("{} v{}", info.name, v)
    } else {
        info.name.clone()
    };

    let title_display = display_width(&title);
    let title_truncated = if title_display > inner_width {
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
        CARD_DESC_LINES,
    );

    let top_border = format!("╭{}╮", "─".repeat(inner_width));
    let bottom_border = format!("╰{}╯", "─".repeat(inner_width));

    let title_padded = pad_to_width_plain(&title_truncated, inner_width);
    let path_padded = pad_to_width_plain(&path_truncated, inner_width);

    let title_line = if is_tty {
        format!("\x1b[36m{}\x1b[0m", title_padded)
    } else {
        title_padded
    };

    let path_line = if is_tty {
        format!("\x1b[90m{}\x1b[0m", path_padded)
    } else {
        path_padded
    };

    let mut lines = vec![top_border];
    lines.push(format!("│{}│", title_line));
    lines.push(format!("│{}│", path_line));

    let empty_line = " ".repeat(inner_width);
    lines.push(format!("│{}│", empty_line));

    for i in 0..CARD_DESC_LINES {
        let desc_line = desc_lines.get(i).map(|s| s.as_str()).unwrap_or("");
        let desc_padded = if desc_line.is_empty() {
            empty_line.clone()
        } else {
            pad_to_width_plain(desc_line, inner_width)
        };
        let desc_styled = if is_tty && !desc_line.is_empty() {
            format!("\x1b[37m{}\x1b[0m", desc_padded)
        } else {
            desc_padded
        };
        lines.push(format!("│{}│", desc_styled));
    }

    lines.push(bottom_border);
    lines
}

fn layout_cards_in_grid(cards: &[Vec<String>]) -> Vec<String> {
    if cards.is_empty() {
        return vec![];
    }

    let term_width = get_terminal_width();
    let cards_per_row = if term_width >= MIN_WIDTH_3_CARDS {
        3
    } else if term_width >= MIN_WIDTH_2_CARDS {
        2
    } else {
        1
    };

    let mut result = Vec::new();
    let spacing = " ".repeat(CARD_SPACING);

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

fn get_terminal_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(120)
}

fn pad_to_width_plain(s: &str, width: usize) -> String {
    let current_width = s
        .chars()
        .map(|c| unicode_width::UnicodeWidthChar::width(c).unwrap_or(0))
        .sum::<usize>();
    if current_width >= width {
        s.to_string()
    } else {
        format!("{}{}", s, " ".repeat(width - current_width))
    }
}

fn truncate_to_width(s: &str, max_width: usize) -> String {
    let mut result = String::new();
    let mut width = 0;

    for c in s.chars() {
        let c_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);
        if width + c_width > max_width.saturating_sub(1) {
            result.push('…');
            break;
        }
        result.push(c);
        width += c_width;
    }

    result
}

fn wrap_text(text: &str, max_width: usize, max_lines: usize) -> Vec<String> {
    if text.is_empty() || max_lines == 0 {
        return vec![];
    }

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines = Vec::new();
    let mut current_line = String::new();

    for word in words {
        let word_width = display_width(word);
        let current_width = display_width(&current_line);

        if current_line.is_empty() {
            current_line = word.to_string();
        } else if current_width + 1 + word_width <= max_width {
            current_line = format!("{} {}", current_line, word);
        } else {
            lines.push(current_line);
            current_line = word.to_string();

            if lines.len() >= max_lines {
                break;
            }
        }
    }

    if !current_line.is_empty() && lines.len() < max_lines {
        lines.push(current_line);
    }

    lines.truncate(max_lines);

    if let Some(last) = lines.last_mut() {
        if display_width(last) > max_width.saturating_sub(1) {
            *last = truncate_to_width(last, max_width);
        }
    }

    lines
}

fn pad_header(left: &str, right: &str) -> String {
    let left_display = display_width(left);
    let right_display = display_width(right);
    let target_content_width = BOX_WIDTH - 2;

    let padding = target_content_width.saturating_sub(left_display + right_display + 1);
    format!("{}{}{} ", left, " ".repeat(padding), right)
}

pub const NAME: &str = "about";
pub const DESCRIPTION: &str = "Cargo workspace about command and data provider";
pub const SHORTNAME: &str = "about";
pub const DATA_PROVIDER_NAME: &str = "about";

pub struct AboutExtension;

impl Extension for AboutExtension {
    fn name(&self) -> &'static str {
        NAME
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn shortname(&self) -> &'static str {
        SHORTNAME
    }

    fn types(&self) -> crate::extension::ExtensionType {
        crate::extension::ExtensionType::DATASOURCE | crate::extension::ExtensionType::COMMAND
    }

    fn command_names(&self) -> &'static [&'static str] {
        &["about"]
    }

    fn data_provider_name(&self) -> Option<&'static str> {
        Some(DATA_PROVIDER_NAME)
    }

    fn register_commands(&self, registry: &mut crate::extension::CommandRegistry) {
        use crate::config::ExecCommandSpec;
        use std::collections::HashMap;

        registry.insert(
            "about".to_string(),
            crate::config::CommandSpec::Exec(ExecCommandSpec {
                program: "cargo-ops".to_string(),
                args: vec!["about".to_string()],
                env: HashMap::new(),
                cwd: None,
                timeout_secs: None,
            }),
        );
    }

    fn register_data_providers(&self, _registry: &mut DataRegistry) {}
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
    fn pad_header_balances_left_and_right() {
        let result = pad_header("Left", "Right");
        assert!(result.starts_with("Left"));
        assert!(result.ends_with("Right "));
        assert!(result.len() <= BOX_WIDTH);
    }

    #[test]
    fn truncate_to_width_short_string() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
    }

    #[test]
    fn truncate_to_width_exact_fit() {
        assert_eq!(truncate_to_width("hello", 5), "hell…");
    }

    #[test]
    fn truncate_to_width_needs_truncation() {
        assert_eq!(truncate_to_width("hello world", 6), "hello…");
    }

    #[test]
    fn wrap_text_single_line() {
        let result = wrap_text("hello world", 20, 2);
        assert_eq!(result, vec!["hello world"]);
    }

    #[test]
    fn wrap_text_multiple_lines() {
        let result = wrap_text("one two three four five", 10, 3);
        assert!(result.len() <= 3);
        for line in &result {
            assert!(display_width(line) <= 10);
        }
    }

    #[test]
    fn wrap_text_respects_max_lines() {
        let result = wrap_text("one two three four five six seven eight", 5, 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn wrap_text_empty() {
        let result = wrap_text("", 10, 2);
        assert!(result.is_empty());
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
    fn pad_to_width_adds_padding() {
        let result = pad_to_width_plain("hi", 5);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn pad_to_width_already_wide() {
        let result = pad_to_width_plain("hello", 3);
        assert_eq!(result, "hello");
    }
}
