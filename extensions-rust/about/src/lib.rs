//! Rust stack implementation for the about system.
//!
//! Provides:
//! - `project_identity` data provider (Rust-specific project identity)
//! - `dashboard` command (comprehensive project health page)
//!
//! Split into submodules by responsibility (CQ-001):
//! - `identity`: Rust project_identity data provider
//! - `text_util`: formatting, padding, truncation, wrapping
//! - `cards`: crate card rendering and grid layout
//! - `query`: data fetching from DuckDB/providers
//! - `format`: section formatters for dashboard
//! - `dashboard`: dashboard orchestration and new section formatters

pub(crate) mod cards;
pub mod dashboard;
pub(crate) mod format;
pub(crate) mod identity;
pub(crate) mod query;
pub(crate) mod text_util;

pub use dashboard::{run_dashboard, DashboardOptions};

pub const NAME: &str = "about-rust";
pub const DESCRIPTION: &str = "Rust project identity and dashboard";
pub const SHORTNAME: &str = "about-rs";
pub const DATA_PROVIDER_NAME: &str = "project_identity";

pub struct AboutRustExtension;

ops_extension::impl_extension! {
    AboutRustExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ops_extension::ExtensionType::DATASOURCE | ops_extension::ExtensionType::COMMAND,
    stack: Some(ops_extension::Stack::Rust),
    command_names: &["dashboard"],
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_commands: |_self, _registry| {},
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(identity::RustIdentityProvider));
    },
    factory: ABOUT_RUST_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutRustExtension)))
    },
}

#[cfg(test)]
mod tests {
    use super::cards::*;
    use super::format::*;
    use super::query::*;
    use super::text_util::*;
    use ops_core::output::display_width;
    use ops_duckdb::sql::CrateCoverage;
    use std::collections::HashMap;

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
        assert!(result.len() <= CardLayoutConfig::BOX_WIDTH);
    }

    #[test]
    fn truncate_to_width_short_string() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
    }

    #[test]
    fn truncate_to_width_exact_fit() {
        assert_eq!(truncate_to_width("hello", 5), "hell\u{2026}");
    }

    #[test]
    fn truncate_to_width_needs_truncation() {
        assert_eq!(truncate_to_width("hello world", 6), "hello\u{2026}");
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

    #[test]
    fn format_number_zero() {
        assert_eq!(format_number(0), "0");
    }

    #[test]
    fn format_number_small() {
        assert_eq!(format_number(42), "42");
        assert_eq!(format_number(999), "999");
    }

    #[test]
    fn format_number_thousands() {
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(4231), "4,231");
        assert_eq!(format_number(999999), "999,999");
    }

    #[test]
    fn format_number_millions() {
        assert_eq!(format_number(1000000), "1,000,000");
        assert_eq!(format_number(1234567), "1,234,567");
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
        let inner = &card[3][3..card[3].len() - 3]; // strip borders
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

    fn test_workspace_manifest(members: Vec<String>) -> ops_cargo_toml::CargoToml {
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
    fn resolve_member_globs_expands_glob() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/bar")).unwrap();
        std::fs::write(
            root.join("crates/bar/Cargo.toml"),
            "[package]\nname=\"bar\"\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("crates/not-a-crate")).unwrap();

        let members = vec!["crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);

        assert_eq!(resolved.len(), 2);
        assert!(resolved.contains(&"crates/bar".to_string()));
        assert!(resolved.contains(&"crates/foo".to_string()));
        assert_eq!(resolved[0], "crates/bar");
        assert_eq!(resolved[1], "crates/foo");
    }

    #[test]
    fn resolve_member_globs_non_glob_passthrough() {
        let members = vec!["crates/core".to_string(), "crates/cli".to_string()];
        let resolved = resolve_member_globs(&members, std::path::Path::new("/nonexistent"));
        assert_eq!(
            resolved,
            vec!["crates/cli".to_string(), "crates/core".to_string()]
        );
    }

    #[test]
    fn resolve_member_globs_mixed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();

        std::fs::create_dir_all(root.join("crates/foo")).unwrap();
        std::fs::write(
            root.join("crates/foo/Cargo.toml"),
            "[package]\nname=\"foo\"\n",
        )
        .unwrap();

        let members = vec!["explicit".to_string(), "crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);

        assert_eq!(resolved.len(), 2);
        assert!(resolved.contains(&"explicit".to_string()));
        assert!(resolved.contains(&"crates/foo".to_string()));
    }

    // ── text_util additional tests ──────────────────────────────────────

    #[test]
    fn format_number_negative() {
        assert_eq!(format_number(-42), "-42");
        assert_eq!(format_number(-1234), "-1,234");
    }

    #[test]
    fn char_display_width_ascii() {
        use super::text_util::char_display_width;
        assert_eq!(char_display_width('a'), 1);
        assert_eq!(char_display_width(' '), 1);
    }

    #[test]
    fn char_display_width_wide() {
        use super::text_util::char_display_width;
        // CJK character should be width 2
        assert_eq!(char_display_width('漢'), 2);
    }

    #[test]
    fn char_display_width_zero_width() {
        use super::text_util::char_display_width;
        // Zero-width joiner
        assert_eq!(char_display_width('\u{200D}'), 0);
    }

    #[test]
    fn tty_style_applies_when_tty() {
        let styled = tty_style("hello", ops_core::style::cyan, true);
        assert!(styled.contains("hello"));
        // Should contain ANSI escape sequences
        assert!(styled.contains("\x1b["));
    }

    #[test]
    fn tty_style_passthrough_when_not_tty() {
        let result = tty_style("hello", ops_core::style::cyan, false);
        assert_eq!(result, "hello");
    }

    #[test]
    fn get_terminal_width_default() {
        // When COLUMNS is not set (or invalid), should default to 120
        let saved = std::env::var("COLUMNS").ok();
        std::env::remove_var("COLUMNS");
        let width = get_terminal_width();
        assert_eq!(width, 120);
        if let Some(v) = saved {
            std::env::set_var("COLUMNS", v);
        }
    }

    #[test]
    fn truncate_to_width_very_short_max() {
        // max_width = 1 means only the ellipsis
        let result = truncate_to_width("hello", 1);
        assert_eq!(result, "…");
    }

    #[test]
    fn truncate_to_width_empty() {
        assert_eq!(truncate_to_width("", 10), "");
    }

    #[test]
    fn wrap_text_max_lines_zero() {
        let result = wrap_text("hello world", 20, 0);
        assert!(result.is_empty());
    }

    #[test]
    fn wrap_text_long_word_exceeds_width() {
        let result = wrap_text("superlongword short", 5, 3);
        assert!(!result.is_empty());
        // The long word should still appear (possibly truncated on last line)
        assert!(result[0].contains("superlongword") || result[0].contains("super"));
    }

    #[test]
    fn pad_header_long_strings() {
        // When left + right exceed BOX_WIDTH, should still produce valid output
        let left = "A".repeat(60);
        let right = "B".repeat(60);
        let result = pad_header(&left, &right);
        assert!(result.contains(&left));
        assert!(result.contains(&right));
    }

    // ── cards additional tests ──────────────────────────────────────────

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
        assert!(result.contains("·"));
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
        // Title line should just be the name, no "v"
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
        // Card width is 32, inner is 30 — title should be truncated with ellipsis
        assert!(card[1].contains("…"));
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
        assert!(card[2].contains("…"));
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
        // Card should have: top border, title, path, stats/empty, 3 desc lines, bottom border = 8
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
    fn layout_cards_multiple_cards() {
        let card1 = vec!["a1".to_string(), "a2".to_string()];
        let card2 = vec!["b1".to_string(), "b2".to_string()];
        let result = layout_cards_in_grid(&[card1, card2]);
        assert!(!result.is_empty());
        // Both cards should appear in the grid
        let joined = result.join("\n");
        assert!(joined.contains("a1"));
        assert!(joined.contains("b1"));
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

    // ── format additional tests ─────────────────────────────────────────

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

    #[test]
    fn resolve_member_globs_no_matching_dirs() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        // Create the parent dir but no children with Cargo.toml
        std::fs::create_dir_all(root.join("crates")).unwrap();

        let members = vec!["crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);
        assert!(resolved.is_empty());
    }

    #[test]
    fn resolve_member_globs_nonexistent_glob_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        // Don't create the "crates" directory at all
        let members = vec!["crates/*".to_string()];
        let resolved = resolve_member_globs(&members, root);
        assert!(resolved.is_empty());
    }
}
