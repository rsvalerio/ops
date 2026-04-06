//! Formatting: section renderers for the about dashboard output.

use std::collections::HashMap;
use std::io::{self, IsTerminal};

use ops_core::style::{cyan, dim};

use ops_cargo_toml::CargoToml;

use super::cards::{
    format_crate_name, layout_cards_in_grid, load_crate_infos, render_card, CardLayoutConfig,
};
use super::query::{CoverageData, DepsTreeData};
use super::text_util::{format_number, pad_header, tty_style};

/// Extract an optional field from the root package, returning `default` if absent.
fn pkg_field<'a>(
    root_pkg: &'a Option<ops_cargo_toml::Package>,
    f: impl FnOnce(&'a ops_cargo_toml::Package) -> Option<&'a str>,
    default: &'a str,
) -> &'a str {
    root_pkg.as_ref().and_then(f).unwrap_or(default)
}

pub(crate) fn format_header(root_pkg: &Option<ops_cargo_toml::Package>) -> Vec<String> {
    let pkg_name = pkg_field(root_pkg, |p| Some(p.name.as_str()), "workspace");
    let pkg_version = pkg_field(root_pkg, |p| p.version.as_str(), "unknown");
    let edition = pkg_field(root_pkg, |p| p.edition.as_str(), "2021");
    let license = pkg_field(root_pkg, |p| p.license.as_str(), "");

    let header_left = format!("\u{1f980} {} v{}", pkg_name, pkg_version);
    let header_right = format!("Edition {} \u{00b7} {}", edition, license);
    let header_content = pad_header(&header_left, &header_right);

    vec![
        format!(
            "\u{250c}{}\u{2510}",
            "\u{2500}".repeat(CardLayoutConfig::BOX_WIDTH)
        ),
        format!("\u{2502}  {}\u{2502}", header_content),
        format!(
            "\u{2514}{}\u{2518}",
            "\u{2500}".repeat(CardLayoutConfig::BOX_WIDTH)
        ),
    ]
}

pub(crate) fn format_description(root_pkg: &Option<ops_cargo_toml::Package>) -> Vec<String> {
    match root_pkg.as_ref().and_then(|p| p.description.as_str()) {
        Some(desc) => vec![String::new(), format!("  {}", desc)],
        None => vec![],
    }
}

pub(crate) fn format_workspace_info(
    manifest: &CargoToml,
    cwd: &std::path::Path,
    project_loc: Option<i64>,
    project_file_count: Option<i64>,
    coverage_data: Option<&CoverageData>,
) -> Vec<String> {
    let member_count = manifest
        .workspace
        .as_ref()
        .map(|w| w.members.len())
        .unwrap_or(1);

    let mut lines = vec![String::new()];
    lines.push(format!(
        "  \u{25b8} workspace   {}",
        dim(&cwd.display().to_string())
    ));
    lines.push(format!(
        "  \u{25b8} members     {} crate{}",
        dim(&member_count.to_string()),
        if member_count != 1 { "s" } else { "" }
    ));
    if let Some(loc) = project_loc {
        lines.push(format!(
            "  \u{25b8} code        {} loc",
            dim(&format_number(loc))
        ));
    }
    if let Some(files) = project_file_count {
        if files > 0 {
            lines.push(format!(
                "  \u{25b8} files       {} file{}",
                dim(&format_number(files)),
                if files != 1 { "s" } else { "" }
            ));
        }
    }
    if let Some(cov_data) = coverage_data {
        if cov_data.project.lines_count > 0 {
            lines.push(format!(
                "  \u{25b8} coverage    {}% lines",
                dim(&format!("{:.1}", cov_data.project.lines_percent))
            ));

            if let Some(ws) = &manifest.workspace {
                if !ws.members.is_empty() && !cov_data.per_crate.is_empty() {
                    lines.push(String::new());
                    lines.extend(
                        format_coverage_table(ws, cov_data)
                            .lines()
                            .map(|l| format!("    {l}")),
                    );
                }
            }
        }
    }
    lines
}

/// Status icon for coverage percentage.
pub(crate) fn coverage_icon(pct: f64) -> &'static str {
    if pct < 50.0 {
        "\u{1f480}" // skull
    } else if pct < 80.0 {
        "\u{26a0}\u{fe0f}" // warning
    } else {
        "\u{2705}" // check mark
    }
}

/// Color for coverage percentage.
pub(crate) fn coverage_color(pct: f64) -> ops_core::table::Color {
    use ops_core::table::Color;
    if pct < 50.0 {
        Color::Red
    } else if pct < 80.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

pub(crate) fn format_coverage_table(
    ws: &ops_cargo_toml::Workspace,
    cov_data: &CoverageData,
) -> String {
    use ops_core::table::OpsTable;

    let mut table = OpsTable::new();
    table.set_header(vec!["", "Crate", "Coverage", "Covered", "Total"]);

    let mut sorted_members: Vec<&str> = ws.members.iter().map(|s| s.as_str()).collect();
    sorted_members.sort();

    for member in &sorted_members {
        if let Some(cov) = cov_data.per_crate.get(*member) {
            if cov.lines_count == 0 {
                continue;
            }
            let icon = coverage_icon(cov.lines_percent);
            let color = coverage_color(cov.lines_percent);
            let name = format_crate_name(member);
            let pct = format!("{:.1}%", cov.lines_percent);

            table.add_row(vec![
                table.cell(icon, color),
                table.cell(&name, color),
                table.cell(&pct, color),
                table.cell(&format_number(cov.lines_covered), color),
                table.cell(&format_number(cov.lines_count), color),
            ]);
        }
    }

    table.to_string()
}

pub(crate) fn format_crates_section(
    manifest: &CargoToml,
    workspace_root: &std::path::Path,
    crate_locs: Option<&HashMap<String, i64>>,
    crate_file_counts: Option<&HashMap<String, i64>>,
    crate_deps: Option<&HashMap<String, i64>>,
) -> Vec<String> {
    let members = match &manifest.workspace {
        Some(ws) if !ws.members.is_empty() => &ws.members,
        _ => return vec![],
    };

    let mut sorted_members: Vec<&str> = members.iter().map(|s| s.as_str()).collect();
    sorted_members.sort();

    let mut crate_infos = load_crate_infos(&sorted_members, workspace_root);

    if let Some(locs) = crate_locs {
        for info in &mut crate_infos {
            info.loc = locs.get(&info.path).copied();
        }
    }

    if let Some(files) = crate_file_counts {
        for info in &mut crate_infos {
            info.file_count = files.get(&info.path).copied();
        }
    }

    if let Some(deps) = crate_deps {
        for info in &mut crate_infos {
            info.dep_count = deps.get(&info.package_name).copied();
        }
    }

    let is_tty = io::stdout().is_terminal();
    let cards: Vec<Vec<String>> = crate_infos
        .iter()
        .map(|info| render_card(info, is_tty))
        .collect();

    let mut lines = vec![String::new(), "  CRATES".to_string(), String::new()];
    lines.extend(layout_cards_in_grid(&cards));

    lines
}

pub(crate) fn format_dependencies_section(
    manifest: &CargoToml,
    deps_tree: Option<&DepsTreeData>,
) -> Vec<String> {
    let deps_tree = match deps_tree {
        Some(d) if !d.per_crate.is_empty() => d,
        _ => return vec![],
    };

    if manifest
        .workspace
        .as_ref()
        .is_none_or(|ws| ws.members.is_empty())
    {
        return vec![];
    }

    let is_tty = io::stdout().is_terminal();

    let mut crate_names: Vec<&String> = deps_tree.per_crate.keys().collect();
    crate_names.sort();

    let mut lines = vec![String::new(), "  DEPENDENCIES".to_string()];

    for crate_name in crate_names {
        let deps = &deps_tree.per_crate[crate_name];
        if deps.is_empty() {
            continue;
        }

        lines.push(String::new());
        lines.push(format!("  {}", tty_style(crate_name, cyan, is_tty)));

        let last_idx = deps.len() - 1;
        for (i, (dep_name, version_req)) in deps.iter().enumerate() {
            let connector = if i == last_idx {
                "\u{2514}\u{2500}\u{2500}"
            } else {
                "\u{251c}\u{2500}\u{2500}"
            };
            lines.push(format!(
                "  {}",
                tty_style(
                    &format!("{} {} {}", connector, dep_name, version_req),
                    dim,
                    is_tty
                )
            ));
        }
    }

    lines
}
