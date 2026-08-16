//! Tests for the report as an operator sees it: `build_report` rendered
//! through the shared theme machinery, exactly as `run_deps` renders it.

use super::*;
use crate::{AdvisoryEntry, DenyEntry, DenyResult, LicenseEntry, SourceEntry, UpgradeResult};

/// Render a report through the shared theme machinery, mirroring `run_deps`.
/// Color is gated off in tests (no TTY), so the output is plain text and these
/// substring assertions are stable. Section "(N)" headers are gone — counts now
/// live in each row's result slot — so the assertions check label + result.
fn render(report: &DepsReport) -> String {
    use ops_core::config::theme_types::ThemeConfig;
    use ops_theme::ConfigurableTheme;
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    theme.render_report(&build_report(report), 100).join("\n")
}

#[test]
fn format_report_all_clean() {
    let report = DepsReport::default();
    let output = render(&report);
    assert!(output.contains("Dependency Health Report"));
    assert!(output.contains("Compatible Upgrades"));
    assert!(output.contains("None"));
    assert!(output.contains("Advisories"));
    assert!(output.contains("License Issues"));
    assert!(output.contains("Duplicate Crates"));
    assert!(output.contains("Source Issues"));
    // Footer reuses the runner's summary chrome, counting checks not time.
    assert!(output.contains("Done 6 checks"));
}

#[test]
fn format_report_with_upgrades() {
    let report = DepsReport {
        upgrades: UpgradeResult {
            compatible: vec![UpgradeEntry {
                name: "serde".into(),
                old_req: "1.0.100".into(),
                compatible: "1.0.228".into(),
                latest: "1.0.228".into(),
                new_req: "1.0.228".into(),
                note: None,
            }],
            incompatible: vec![],
        },
        deny: DenyResult::default(),
    };
    let output = render(&report);
    assert!(output.contains("Compatible Upgrades"));
    assert!(output.contains("1 upgrade"));
    assert!(output.contains("serde"));
    assert!(output.contains("1.0.100"));
    assert!(output.contains("1.0.228"));
    assert!(output.contains("cargo upgrade"));
}

#[test]
fn format_report_with_breaking_upgrades_shows_advice() {
    let report = DepsReport {
        upgrades: UpgradeResult {
            compatible: vec![],
            incompatible: vec![UpgradeEntry {
                name: "clap".into(),
                old_req: "3.0.0".into(),
                compatible: "3.2.25".into(),
                latest: "4.6.0".into(),
                new_req: "4.6.0".into(),
                note: Some("incompatible".into()),
            }],
        },
        deny: DenyResult::default(),
    };
    let output = render(&report);
    assert!(output.contains("Breaking Upgrades"));
    assert!(output.contains("1 upgrade"));
    assert!(output.contains("cargo upgrade --incompatible"));
    // ERR-1 / TASK-0600: a breaking-upgrade row must surface the absolute
    // `latest` so operators see how far behind the compatible cap is.
    assert!(
        output.contains("4.6.0") && output.contains("(latest"),
        "breaking row must include 'latest' column: {output}"
    );
}

#[test]
fn format_report_with_advisory() {
    let report = DepsReport {
        upgrades: UpgradeResult::default(),
        deny: DenyResult {
            advisories: vec![AdvisoryEntry {
                id: "RUSTSEC-2024-0001".into(),
                package: "foo".into(),
                severity: "error".into(),
                title: "something bad".into(),
            }],
            ..Default::default()
        },
    };
    let output = render(&report);
    assert!(output.contains("Advisories"));
    assert!(output.contains("1 error"));
    assert!(output.contains("RUSTSEC-2024-0001"));
    assert!(output.contains("foo"));
    assert!(output.contains("cargo deny check advisories"));
}

#[test]
fn format_report_duplicate_crates_shows_totals_only() {
    let report = DepsReport {
        upgrades: UpgradeResult::default(),
        deny: DenyResult {
            bans: vec![
                BanEntry(DenyEntry {
                    package: "hashbrown".into(),
                    message: "found 3 duplicate entries".into(),
                    severity: "warning".into(),
                }),
                BanEntry(DenyEntry {
                    package: "syn".into(),
                    message: "found 2 duplicate entries".into(),
                    severity: "error".into(),
                }),
            ],
            ..Default::default()
        },
    };
    let output = render(&report);
    assert!(output.contains("Duplicate Crates"));
    // Should NOT list individual crate names
    assert!(!output.contains("hashbrown"));
    assert!(!output.contains("syn"));
    // Should show severity totals in the result slot
    assert!(output.contains("1 error"));
    assert!(output.contains("1 warning"));
    assert!(output.contains("transitive, usually harmless"));
}

// -- Format: license issues with entries --

#[test]
fn format_report_with_license_issues() {
    let report = DepsReport {
        upgrades: UpgradeResult::default(),
        deny: DenyResult {
            licenses: vec![
                LicenseEntry(DenyEntry {
                    package: "gpl-crate".into(),
                    message: "license rejected: GPL-3.0".into(),
                    severity: "error".into(),
                }),
                LicenseEntry(DenyEntry {
                    package: "unknown-lic".into(),
                    message: "no license field".into(),
                    severity: "warning".into(),
                }),
            ],
            ..Default::default()
        },
    };
    let output = render(&report);
    assert!(output.contains("License Issues"));
    assert!(output.contains("1 error"));
    assert!(output.contains("1 warning"));
    assert!(output.contains("gpl-crate"));
    assert!(output.contains("unknown-lic"));
    assert!(output.contains("deny.toml"));
}

#[test]
fn format_report_with_source_issues() {
    let report = DepsReport {
        upgrades: UpgradeResult::default(),
        deny: DenyResult {
            sources: vec![SourceEntry(DenyEntry {
                package: "sketchy-src".into(),
                message: "source not allowed".into(),
                severity: "error".into(),
            })],
            ..Default::default()
        },
    };
    let output = render(&report);
    assert!(output.contains("Source Issues"));
    assert!(output.contains("1 error"));
    assert!(output.contains("sketchy-src"));
    assert!(output.contains("trusted sources"));
}

// -- Format: bans summary variants --

#[test]
fn format_report_bans_info_only() {
    let report = DepsReport {
        upgrades: UpgradeResult::default(),
        deny: DenyResult {
            bans: vec![BanEntry(DenyEntry {
                package: "hashbrown".into(),
                message: "found 2 duplicate entries".into(),
                severity: "info".into(),
            })],
            ..Default::default()
        },
    };
    let output = render(&report);
    assert!(output.contains("Duplicate Crates"));
    assert!(output.contains("1 info"));
    assert!(!output.contains("error"));
    assert!(!output.contains("warning"));
}

#[test]
fn format_report_bans_plural_errors_and_warnings() {
    let report = DepsReport {
        upgrades: UpgradeResult::default(),
        deny: DenyResult {
            bans: vec![
                BanEntry(DenyEntry {
                    package: "a".into(),
                    message: "banned".into(),
                    severity: "error".into(),
                }),
                BanEntry(DenyEntry {
                    package: "b".into(),
                    message: "banned".into(),
                    severity: "error".into(),
                }),
                BanEntry(DenyEntry {
                    package: "c".into(),
                    message: "dup".into(),
                    severity: "warning".into(),
                }),
                BanEntry(DenyEntry {
                    package: "d".into(),
                    message: "dup".into(),
                    severity: "warning".into(),
                }),
                BanEntry(DenyEntry {
                    package: "e".into(),
                    message: "dup".into(),
                    severity: "warning".into(),
                }),
            ],
            ..Default::default()
        },
    };
    let output = render(&report);
    assert!(output.contains("2 errors"));
    assert!(output.contains("3 warnings"));
}

// -- Format: advisories with mixed severities --

#[test]
fn format_report_advisories_mixed_severities() {
    let report = DepsReport {
        upgrades: UpgradeResult::default(),
        deny: DenyResult {
            advisories: vec![
                AdvisoryEntry {
                    id: "RUSTSEC-2024-0001".into(),
                    package: "foo".into(),
                    severity: "error".into(),
                    title: "critical vuln".into(),
                },
                AdvisoryEntry {
                    id: "RUSTSEC-2024-0002".into(),
                    package: "bar".into(),
                    severity: "warning".into(),
                    title: "unmaintained".into(),
                },
                AdvisoryEntry {
                    id: "RUSTSEC-2024-0003".into(),
                    package: "baz".into(),
                    severity: "info".into(),
                    title: "informational".into(),
                },
            ],
            ..Default::default()
        },
    };
    let output = render(&report);
    assert!(output.contains("Advisories"));
    assert!(output.contains("1 error"));
    assert!(output.contains("1 warning"));
    assert!(output.contains("RUSTSEC-2024-0001"));
    assert!(output.contains("RUSTSEC-2024-0002"));
    assert!(output.contains("RUSTSEC-2024-0003"));
}

// -- Format: multiple compatible and breaking upgrades --

#[test]
fn format_report_multiple_upgrades_aligned() {
    let report = DepsReport {
        upgrades: UpgradeResult {
            compatible: vec![
                UpgradeEntry {
                    name: "serde".into(),
                    old_req: "1.0.0".into(),
                    compatible: "1.0.228".into(),
                    latest: "1.0.228".into(),
                    new_req: "1.0.228".into(),
                    note: None,
                },
                UpgradeEntry {
                    name: "tokio-stream".into(),
                    old_req: "0.1.0".into(),
                    compatible: "0.1.17".into(),
                    latest: "0.1.17".into(),
                    new_req: "0.1.17".into(),
                    note: None,
                },
            ],
            incompatible: vec![UpgradeEntry {
                name: "clap".into(),
                old_req: "3.0.0".into(),
                compatible: "3.2.25".into(),
                latest: "4.6.0".into(),
                new_req: "4.6.0".into(),
                note: Some("incompatible".into()),
            }],
        },
        deny: DenyResult::default(),
    };
    let output = render(&report);
    assert!(output.contains("Compatible Upgrades"));
    assert!(output.contains("2 upgrades"));
    assert!(output.contains("Breaking Upgrades"));
    assert!(output.contains("serde"));
    assert!(output.contains("tokio-stream"));
    assert!(output.contains("clap"));
}
