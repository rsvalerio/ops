//! Formatting: section renderers for the about dashboard output.

use std::collections::HashMap;
use std::io::{self, IsTerminal};

use ops_core::style::{cyan, dim};

use ops_cargo_toml::CargoToml;

use super::cards::{
    layout_cards_in_grid, load_crate_infos, render_card, resolve_crate_display_name,
    CardLayoutConfig,
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
                        format_coverage_table(ws, cov_data, cwd)
                            .lines()
                            .map(|l| format!("    {l}")),
                    );
                }
            }
        }
    }
    lines
}

enum CoverageTier {
    Low,
    Medium,
    High,
}

fn coverage_tier(pct: f64) -> CoverageTier {
    if pct < 50.0 {
        CoverageTier::Low
    } else if pct < 80.0 {
        CoverageTier::Medium
    } else {
        CoverageTier::High
    }
}

/// Status icon for coverage percentage.
pub(crate) fn coverage_icon(pct: f64) -> &'static str {
    match coverage_tier(pct) {
        CoverageTier::Low => "\u{1f480}",           // skull
        CoverageTier::Medium => "\u{26a0}\u{fe0f}", // warning
        CoverageTier::High => "\u{2705}",           // check mark
    }
}

/// Color for coverage percentage.
pub(crate) fn coverage_color(pct: f64) -> ops_core::table::Color {
    use ops_core::table::Color;
    match coverage_tier(pct) {
        CoverageTier::Low => Color::Red,
        CoverageTier::Medium => Color::Yellow,
        CoverageTier::High => Color::Green,
    }
}

pub(crate) fn format_coverage_table(
    ws: &ops_cargo_toml::Workspace,
    cov_data: &CoverageData,
    workspace_root: &std::path::Path,
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
            let name = resolve_crate_display_name(member, workspace_root);
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

/// Create a test workspace manifest (shared test helper).
#[cfg(test)]
pub(crate) fn test_workspace_manifest(members: Vec<String>) -> ops_cargo_toml::CargoToml {
    use std::collections::BTreeMap;
    ops_cargo_toml::CargoToml {
        package: None,
        workspace: Some(ops_cargo_toml::Workspace {
            members,
            resolver: None,
            dependencies: BTreeMap::new(),
            default_members: vec![],
            exclude: vec![],
            package: None,
        }),
        dependencies: BTreeMap::new(),
        dev_dependencies: BTreeMap::new(),
        build_dependencies: BTreeMap::new(),
        features: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{CoverageData, DepsTreeData};
    use ops_duckdb::sql::CrateCoverage;
    use std::collections::HashMap;

    fn test_package(name: &str, version: &str, desc: Option<&str>) -> ops_cargo_toml::Package {
        let desc_line = match desc {
            Some(d) => format!("description = \"{}\"", d),
            None => String::new(),
        };
        let toml_str = format!(
            "[package]\nname = \"{}\"\nversion = \"{}\"\nedition = \"2021\"\nlicense = \"MIT\"\n{}",
            name, version, desc_line
        );
        let manifest = ops_cargo_toml::CargoToml::parse(&toml_str).unwrap();
        manifest.package.unwrap()
    }

    #[test]
    fn format_dependencies_section_none_returns_empty() {
        let manifest = test_workspace_manifest(vec!["crates/a".to_string()]);
        let result = format_dependencies_section(&manifest, None);
        assert!(result.is_empty());
    }

    #[test]
    fn format_dependencies_section_renders_tree() {
        let manifest = test_workspace_manifest(vec!["crates/a".to_string()]);
        let mut per_crate = HashMap::new();
        per_crate.insert(
            "ops-core".to_string(),
            vec![
                ("anyhow".to_string(), "^1.0".to_string()),
                ("serde".to_string(), "^1.0".to_string()),
                ("toml".to_string(), "^0.8".to_string()),
            ],
        );
        per_crate.insert(
            "ops-cli".to_string(),
            vec![
                ("clap".to_string(), "^4.0".to_string()),
                ("tokio".to_string(), "^1.0".to_string()),
            ],
        );
        let deps_tree = DepsTreeData { per_crate };
        let result = format_dependencies_section(&manifest, Some(&deps_tree));
        let output = result.join("\n");

        assert!(output.contains("DEPENDENCIES"));
        assert!(output.contains("ops-cli"));
        assert!(output.contains("ops-core"));
        assert!(output.contains("\u{251c}\u{2500}\u{2500} clap"));
        assert!(output.contains("\u{2514}\u{2500}\u{2500} tokio"));
        assert!(output.contains("\u{251c}\u{2500}\u{2500} anyhow"));
        assert!(output.contains("\u{251c}\u{2500}\u{2500} serde"));
        assert!(output.contains("\u{2514}\u{2500}\u{2500} toml"));
        assert!(output.contains("^1.0"));
        assert!(output.contains("^4.0"));
        assert!(output.contains("^0.8"));

        let cli_pos = output.find("ops-cli").unwrap();
        let core_pos = output.find("ops-core").unwrap();
        assert!(cli_pos < core_pos, "crate names should be sorted");
    }

    #[test]
    fn workspace_info_coverage_shows_project_total() {
        let manifest =
            test_workspace_manifest(vec!["crates/core".to_string(), "crates/cli".to_string()]);
        let per_crate = HashMap::new();
        let coverage_data = CoverageData {
            project: CrateCoverage {
                lines_count: 2608,
                lines_covered: 2126,
                lines_percent: 81.5,
            },
            per_crate,
        };
        let cwd = std::path::PathBuf::from("/test/workspace");
        let result = format_workspace_info(&manifest, &cwd, None, None, Some(&coverage_data));
        let output = result.join("\n");

        assert!(output.contains("81.5"), "should contain project coverage");
    }

    #[test]
    fn coverage_table_shows_per_crate() {
        let ws = ops_cargo_toml::Workspace {
            members: vec!["crates/core".to_string(), "crates/cli".to_string()],
            resolver: None,
            dependencies: std::collections::BTreeMap::new(),
            default_members: vec![],
            exclude: vec![],
            package: None,
        };
        let mut per_crate = HashMap::new();
        per_crate.insert(
            "crates/core".to_string(),
            CrateCoverage {
                lines_count: 1383,
                lines_covered: 1234,
                lines_percent: 89.2,
            },
        );
        per_crate.insert(
            "crates/cli".to_string(),
            CrateCoverage {
                lines_count: 1225,
                lines_covered: 892,
                lines_percent: 72.8,
            },
        );
        let coverage_data = CoverageData {
            project: CrateCoverage {
                lines_count: 2608,
                lines_covered: 2126,
                lines_percent: 81.5,
            },
            per_crate,
        };
        let output = format_coverage_table(&ws, &coverage_data, std::path::Path::new("/tmp"));

        assert!(output.contains("Core"), "should contain crate name");
        assert!(output.contains("Cli"), "should contain crate name");
        assert!(output.contains("89.2%"), "should contain crate percentage");
        assert!(output.contains("72.8%"), "should contain crate percentage");
        assert!(output.contains("1,234"), "should contain covered count");
        assert!(output.contains("1,383"), "should contain total count");

        let cli_pos = output.find("Cli").unwrap();
        let core_pos = output.find("Core").unwrap();
        assert!(cli_pos < core_pos, "crate names should be sorted");
    }

    #[test]
    fn coverage_table_skips_zero_count_crates() {
        let ws = ops_cargo_toml::Workspace {
            members: vec!["crates/core".to_string(), "crates/cli".to_string()],
            resolver: None,
            dependencies: std::collections::BTreeMap::new(),
            default_members: vec![],
            exclude: vec![],
            package: None,
        };
        let mut per_crate = HashMap::new();
        per_crate.insert(
            "crates/core".to_string(),
            CrateCoverage {
                lines_count: 100,
                lines_covered: 80,
                lines_percent: 80.0,
            },
        );
        per_crate.insert(
            "crates/cli".to_string(),
            CrateCoverage {
                lines_count: 0,
                lines_covered: 0,
                lines_percent: 0.0,
            },
        );
        let coverage_data = CoverageData {
            project: CrateCoverage {
                lines_count: 100,
                lines_covered: 80,
                lines_percent: 80.0,
            },
            per_crate,
        };
        let output = format_coverage_table(&ws, &coverage_data, std::path::Path::new("/tmp"));

        assert!(output.contains("Core"), "should contain crate with data");
        assert!(!output.contains("Cli"), "should skip crate with zero lines");
    }

    #[test]
    fn coverage_table_shows_status_icons() {
        let ws = ops_cargo_toml::Workspace {
            members: vec![
                "crates/good".to_string(),
                "crates/warn".to_string(),
                "crates/bad".to_string(),
            ],
            resolver: None,
            dependencies: std::collections::BTreeMap::new(),
            default_members: vec![],
            exclude: vec![],
            package: None,
        };
        let mut per_crate = HashMap::new();
        per_crate.insert(
            "crates/good".to_string(),
            CrateCoverage {
                lines_count: 100,
                lines_covered: 90,
                lines_percent: 90.0,
            },
        );
        per_crate.insert(
            "crates/warn".to_string(),
            CrateCoverage {
                lines_count: 100,
                lines_covered: 60,
                lines_percent: 60.0,
            },
        );
        per_crate.insert(
            "crates/bad".to_string(),
            CrateCoverage {
                lines_count: 100,
                lines_covered: 30,
                lines_percent: 30.0,
            },
        );
        let coverage_data = CoverageData {
            project: CrateCoverage {
                lines_count: 300,
                lines_covered: 180,
                lines_percent: 60.0,
            },
            per_crate,
        };
        let output = format_coverage_table(&ws, &coverage_data, std::path::Path::new("/tmp"));

        assert!(
            output.contains("\u{2705}"),
            "should contain check mark for >= 80%"
        );
        assert!(
            output.contains("\u{26a0}"),
            "should contain warning for 50-80%"
        );
        assert!(
            output.contains("\u{1f480}"),
            "should contain skull for < 50%"
        );
    }

    #[test]
    fn coverage_icon_thresholds() {
        assert_eq!(coverage_icon(0.0), "\u{1f480}");
        assert_eq!(coverage_icon(49.9), "\u{1f480}");
        assert_eq!(coverage_icon(50.0), "\u{26a0}\u{fe0f}");
        assert_eq!(coverage_icon(79.9), "\u{26a0}\u{fe0f}");
        assert_eq!(coverage_icon(80.0), "\u{2705}");
        assert_eq!(coverage_icon(100.0), "\u{2705}");
    }

    #[test]
    fn coverage_color_thresholds() {
        use ops_core::table::Color;
        assert!(matches!(coverage_color(0.0), Color::Red));
        assert!(matches!(coverage_color(49.9), Color::Red));
        assert!(matches!(coverage_color(50.0), Color::Yellow));
        assert!(matches!(coverage_color(79.9), Color::Yellow));
        assert!(matches!(coverage_color(80.0), Color::Green));
        assert!(matches!(coverage_color(100.0), Color::Green));
    }

    #[test]
    fn format_crates_section_no_workspace() {
        let manifest = ops_cargo_toml::CargoToml {
            package: None,
            workspace: None,
            dependencies: std::collections::BTreeMap::new(),
            dev_dependencies: std::collections::BTreeMap::new(),
            build_dependencies: std::collections::BTreeMap::new(),
            features: std::collections::BTreeMap::new(),
        };
        let result =
            format_crates_section(&manifest, std::path::Path::new("/tmp"), None, None, None);
        assert!(result.is_empty());
    }

    #[test]
    fn format_crates_section_empty_members() {
        let manifest = test_workspace_manifest(vec![]);
        let result =
            format_crates_section(&manifest, std::path::Path::new("/tmp"), None, None, None);
        assert!(result.is_empty());
    }

    #[test]
    fn format_crates_section_with_loc_enrichment() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("crates/core")).unwrap();
        std::fs::write(
            root.join("crates/core/Cargo.toml"),
            "[package]\nname = \"ops-core\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();

        let manifest = test_workspace_manifest(vec!["crates/core".to_string()]);
        let mut locs = HashMap::new();
        locs.insert("crates/core".to_string(), 2500i64);
        let mut file_counts = HashMap::new();
        file_counts.insert("crates/core".to_string(), 15i64);
        let mut deps = HashMap::new();
        deps.insert("ops-core".to_string(), 8i64);

        let result = format_crates_section(
            &manifest,
            root,
            Some(&locs),
            Some(&file_counts),
            Some(&deps),
        );
        let output = result.join("\n");
        assert!(output.contains("CRATES"), "should contain CRATES header");
        assert!(
            output.contains("2,500 loc"),
            "should contain enriched LOC: {output}"
        );
        assert!(
            output.contains("15 files"),
            "should contain enriched file count: {output}"
        );
        assert!(
            output.contains("8 deps"),
            "should contain enriched deps: {output}"
        );
    }

    #[test]
    fn format_header_with_package() {
        let pkg = Some(test_package("my-project", "1.2.3", None));
        let result = format_header(&pkg);
        assert_eq!(result.len(), 3);
        let joined = result.join("\n");
        assert!(joined.contains("my-project"));
        assert!(joined.contains("v1.2.3"));
        assert!(joined.contains("Edition 2021"));
        assert!(joined.contains("MIT"));
    }

    #[test]
    fn format_header_without_package() {
        let result = format_header(&None);
        let joined = result.join("\n");
        assert!(joined.contains("workspace"));
        assert!(joined.contains("unknown"));
    }

    #[test]
    fn format_description_with_desc() {
        let pkg = Some(test_package("test", "0.1.0", Some("My description")));
        let result = format_description(&pkg);
        assert_eq!(result.len(), 2);
        assert!(result[1].contains("My description"));
    }

    #[test]
    fn format_description_without_desc() {
        // When no description in TOML, InheritableField defaults to Value(""),
        // and as_str() returns Some(""), so format_description still produces output.
        // This tests that behavior: empty description => still renders (2 lines).
        let pkg = Some(test_package("test", "0.1.0", None));
        let result = format_description(&pkg);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn format_description_no_package() {
        let result = format_description(&None);
        assert!(result.is_empty());
    }

    #[test]
    fn format_workspace_info_no_workspace() {
        let manifest = ops_cargo_toml::CargoToml {
            package: None,
            workspace: None,
            dependencies: std::collections::BTreeMap::new(),
            dev_dependencies: std::collections::BTreeMap::new(),
            build_dependencies: std::collections::BTreeMap::new(),
            features: std::collections::BTreeMap::new(),
        };
        let cwd = std::path::PathBuf::from("/test");
        let result = format_workspace_info(&manifest, &cwd, None, None, None);
        let output = result.join("\n");
        // dim() wraps numbers in ANSI, so "1" and "crate" may not be adjacent
        assert!(output.contains("crate"));
        assert!(!output.contains("crates"));
    }

    #[test]
    fn format_workspace_info_with_loc_and_files() {
        let manifest =
            test_workspace_manifest(vec!["crates/a".to_string(), "crates/b".to_string()]);
        let cwd = std::path::PathBuf::from("/test/workspace");
        let result = format_workspace_info(&manifest, &cwd, Some(5000), Some(42), None);
        let output = result.join("\n");
        assert!(output.contains("5,000"));
        assert!(output.contains("file"));
        assert!(output.contains("crates"));
    }

    #[test]
    fn format_workspace_info_single_file() {
        let manifest = test_workspace_manifest(vec!["crates/a".to_string()]);
        let cwd = std::path::PathBuf::from("/test");
        let result = format_workspace_info(&manifest, &cwd, Some(100), Some(1), None);
        // Find the files line specifically and verify singular
        let files_line = result.iter().find(|l| l.contains("file")).unwrap();
        assert!(
            files_line.ends_with("file"),
            "should be singular 'file', got: {:?}",
            files_line
        );
    }

    #[test]
    fn format_workspace_info_zero_files_hidden() {
        let manifest = test_workspace_manifest(vec!["crates/a".to_string()]);
        let cwd = std::path::PathBuf::from("/test");
        let result = format_workspace_info(&manifest, &cwd, Some(100), Some(0), None);
        let output = result.join("\n");
        assert!(!output.contains("file"));
    }

    #[test]
    fn format_workspace_info_with_zero_coverage() {
        let manifest = test_workspace_manifest(vec!["crates/a".to_string()]);
        let cwd = std::path::PathBuf::from("/test");
        let coverage_data = CoverageData {
            project: CrateCoverage {
                lines_count: 0,
                lines_covered: 0,
                lines_percent: 0.0,
            },
            per_crate: HashMap::new(),
        };
        let result = format_workspace_info(&manifest, &cwd, None, None, Some(&coverage_data));
        let output = result.join("\n");
        // Zero lines_count should not show coverage
        assert!(!output.contains("coverage"));
    }

    #[test]
    fn format_dependencies_section_empty_deps() {
        let manifest = test_workspace_manifest(vec!["crates/a".to_string()]);
        let deps_tree = DepsTreeData {
            per_crate: HashMap::new(),
        };
        let result = format_dependencies_section(&manifest, Some(&deps_tree));
        assert!(result.is_empty());
    }

    #[test]
    fn format_dependencies_section_no_workspace() {
        let manifest = ops_cargo_toml::CargoToml {
            package: None,
            workspace: None,
            dependencies: std::collections::BTreeMap::new(),
            dev_dependencies: std::collections::BTreeMap::new(),
            build_dependencies: std::collections::BTreeMap::new(),
            features: std::collections::BTreeMap::new(),
        };
        let mut per_crate = HashMap::new();
        per_crate.insert(
            "some-crate".to_string(),
            vec![("dep".to_string(), "^1".to_string())],
        );
        let deps_tree = DepsTreeData { per_crate };
        let result = format_dependencies_section(&manifest, Some(&deps_tree));
        assert!(result.is_empty());
    }

    #[test]
    fn format_dependencies_section_skips_empty_dep_list() {
        let manifest = test_workspace_manifest(vec!["crates/a".to_string()]);
        let mut per_crate = HashMap::new();
        per_crate.insert("empty-crate".to_string(), vec![]);
        per_crate.insert(
            "has-deps".to_string(),
            vec![("serde".to_string(), "^1".to_string())],
        );
        let deps_tree = DepsTreeData { per_crate };
        let result = format_dependencies_section(&manifest, Some(&deps_tree));
        let output = result.join("\n");
        assert!(output.contains("has-deps"));
        assert!(output.contains("serde"));
        // empty-crate name shouldn't appear as a header (it has no deps to show)
        assert!(!output.contains("empty-crate"));
    }

    #[test]
    fn coverage_table_empty_per_crate() {
        let ws = ops_cargo_toml::Workspace {
            members: vec!["crates/core".to_string()],
            resolver: None,
            dependencies: std::collections::BTreeMap::new(),
            default_members: vec![],
            exclude: vec![],
            package: None,
        };
        let coverage_data = CoverageData {
            project: CrateCoverage {
                lines_count: 100,
                lines_covered: 80,
                lines_percent: 80.0,
            },
            per_crate: HashMap::new(),
        };
        let output = format_coverage_table(&ws, &coverage_data, std::path::Path::new("/tmp"));
        // Should still produce a table header but no data rows
        assert!(!output.contains("Core"));
    }
}
