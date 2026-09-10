//! The `running_template_overhead` mis-budget diagnostic (READ-5 /
//! TASK-1971) as an observable value (TEST-33 / TASK-2096).

use super::*;

/// TEST-33 / TASK-2096 AC#2: constructing a misconfigured theme must expose
/// the diagnostic programmatically — `ConfigurableTheme::new` performs no
/// I/O, so the warning is a value, not a stderr side effect the test has to
/// capture.
#[test]
fn a_misbudgeted_theme_reports_its_diagnostic_as_a_value() {
    // `running_template`'s literal text ("  " + the space before {msg} is
    // inside the placeholder args, so just the leading two spaces) plus the
    // widest tick glyph occupies more than the configured 0 columns.
    let mut config: ThemeConfig = toml::from_str(MINIMAL_THEME_TOML).unwrap();
    config.running_template_overhead = 0;

    let theme = ConfigurableTheme::new(config);

    let diagnostic = theme
        .template_overhead_diagnostic()
        .expect("an overhead below the template's literal width must diagnose");
    assert!(
        diagnostic.contains("running_template_overhead is 0"),
        "diagnostic must name the configured value: {diagnostic}"
    );
    assert!(
        diagnostic.contains("columns"),
        "diagnostic must state the occupied lower bound: {diagnostic}"
    );
}

/// The well-formed counterpart: a theme whose overhead meets the lower
/// bound diagnoses nothing, so the resolution-time warn stays silent for
/// every sane theme.
#[test]
fn a_well_budgeted_theme_diagnoses_nothing() {
    let config: ThemeConfig = toml::from_str(MINIMAL_THEME_TOML).unwrap();
    let theme = ConfigurableTheme::new(config);
    assert!(
        theme.template_overhead_diagnostic().is_none(),
        "the shared minimal fixture is well-budgeted; a diagnostic here would warn on every run"
    );
}
