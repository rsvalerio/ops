//! Final-summary rendering for classic/compact themes and the tree
//! plan-header style.

use super::*;

#[test]
fn classic_render_summary_success() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let summary = theme.render_summary(true, 1.5);
    assert!(summary.contains("Done"));
    assert!(summary.contains("1.50s"));
    assert!(summary.starts_with(" └──"));
}

#[test]
fn classic_render_summary_failure() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let summary = theme.render_summary(false, 0.75);
    assert!(summary.contains("Failed"));
    assert!(summary.contains("0.75s"));
    assert!(summary.starts_with(" └──"));
}

#[test]
fn classic_render_summary_minutes() {
    let theme = ConfigurableTheme::new(ThemeConfig::classic());
    let summary = theme.render_summary(true, 278.04);
    assert!(summary.contains("Done"));
    assert!(summary.contains("4m38s"));
}

#[test]
fn compact_render_summary_success() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let summary = theme.render_summary(true, 2.0);
    assert!(summary.contains("Done"));
    assert!(summary.contains("2.00s"));
    assert!(!summary.contains("└──"));
}

#[test]
fn compact_render_summary_failure() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let summary = theme.render_summary(false, 0.5);
    assert!(summary.contains("Failed"));
    assert!(summary.contains("0.50s"));
}

#[test]
fn plan_header_style_tree_renders_correctly() {
    let mut theme = ThemeConfig::compact();
    theme.plan_header_style = PlanHeaderStyle::Tree;
    let configurable = ConfigurableTheme::new(theme);
    let lines = configurable.render_plan_header(&["a".into(), "b".into()]);
    assert_eq!(lines[1], " ┌ Running: a, b");
    assert_eq!(lines[2], " │");
}
