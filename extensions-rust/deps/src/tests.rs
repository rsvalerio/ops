//! Tests for the deps extension.

use super::*;

// -- Extension trait tests --

mod extension_tests {
    use super::*;

    ops_extension::test_datasource_extension!(
        DepsExtension,
        name: "deps",
        data_provider: "deps"
    );
}

// -- Upgrade table parser tests --

#[test]
fn parse_upgrade_table_basic() {
    let stdout = "\
name   old req compatible latest  new req
====   ======= ========== ======  =======
serde  1.0.100 1.0.228    1.0.228 1.0.228
tokio  1.35.0  1.38.0     1.38.0  1.38.0
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "serde");
    assert_eq!(entries[0].old_req, "1.0.100");
    assert_eq!(entries[0].new_req, "1.0.228");
    assert!(entries[0].note.is_none());
    assert_eq!(entries[1].name, "tokio");
}

#[test]
fn parse_upgrade_table_with_notes() {
    let stdout = "\
name   old req compatible latest  new req note
====   ======= ========== ======  ======= ====
clap   3.0.0   3.2.25     4.6.0   3.2.25  incompatible
serde  1.0.100 1.0.228    1.0.228 1.0.228
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "clap");
    assert_eq!(entries[0].note.as_deref(), Some("incompatible"));
    assert!(entries[1].note.is_none());
}

#[test]
fn parse_upgrade_table_empty() {
    let stdout = "";
    let entries = parse_upgrade_table(stdout);
    assert!(entries.is_empty());
}

#[test]
fn parse_upgrade_table_no_data_rows() {
    let stdout = "\
name   old req compatible latest  new req
====   ======= ========== ======  =======
";
    let entries = parse_upgrade_table(stdout);
    assert!(entries.is_empty());
}

#[test]
fn categorize_upgrades_splits_correctly() {
    let entries = vec![
        UpgradeEntry {
            name: "serde".into(),
            old_req: "1.0.100".into(),
            compatible: "1.0.228".into(),
            latest: "1.0.228".into(),
            new_req: "1.0.228".into(),
            note: None,
        },
        UpgradeEntry {
            name: "clap".into(),
            old_req: "3.0.0".into(),
            compatible: "3.2.25".into(),
            latest: "4.6.0".into(),
            new_req: "3.2.25".into(),
            note: Some("incompatible".into()),
        },
    ];
    let result = categorize_upgrades(entries);
    assert_eq!(result.compatible.len(), 1);
    assert_eq!(result.compatible[0].name, "serde");
    assert_eq!(result.incompatible.len(), 1);
    assert_eq!(result.incompatible[0].name, "clap");
}

// -- Deny output parser tests --

#[test]
fn parse_deny_advisory() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"`atty` is unmaintained","code":"unmaintained","advisory":{"id":"RUSTSEC-2024-0375","package":"atty","title":"`atty` is unmaintained","description":"...","date":"2024-09-25","informational":"unmaintained","url":"https://example.com","aliases":[],"categories":[],"cvss":null,"keywords":[],"references":[],"related":[],"withdrawn":null},"labels":[],"graphs":[{"Krate":{"name":"atty","version":"0.2.14"},"parents":[]}],"notes":["ID: RUSTSEC-2024-0375"]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(result.advisories[0].id, "RUSTSEC-2024-0375");
    assert_eq!(result.advisories[0].package, "atty");
    assert_eq!(result.advisories[0].severity, "error");
    assert_eq!(result.advisories[0].title, "`atty` is unmaintained");
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

#[test]
fn parse_deny_license() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"failed to satisfy license requirements","code":"rejected","labels":[{"message":"rejected","span":"MIT"}],"graphs":[{"Krate":{"name":"some-crate","version":"1.0.0"},"parents":[]}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.licenses.len(), 1);
    assert_eq!(result.licenses[0].package, "some-crate");
    assert_eq!(result.licenses[0].severity, "error");
}

#[test]
fn parse_deny_ban() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"crate is banned","code":"banned","labels":[],"graphs":[{"Krate":{"name":"bad-crate","version":"0.1.0"},"parents":[]}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.bans.len(), 1);
    assert_eq!(result.bans[0].package, "bad-crate");
}

#[test]
fn parse_deny_source() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"source not allowed","code":"source-not-allowed","labels":[],"graphs":[{"Krate":{"name":"sketchy-crate","version":"0.1.0"},"parents":[]}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.sources.len(), 1);
    assert_eq!(result.sources[0].package, "sketchy-crate");
}

#[test]
fn parse_deny_skips_log_and_summary() {
    let stderr = r#"{"type":"log","fields":{"timestamp":"2024-01-01","level":"INFO","message":"checking"}}
{"type":"summary","fields":{"advisories":{"errors":0},"bans":{"errors":0},"licenses":{"errors":0},"sources":{"errors":0}}}"#;
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

#[test]
fn parse_deny_empty() {
    let result = parse_deny_output("");
    assert!(result.advisories.is_empty());
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

#[test]
fn parse_deny_skips_invalid_json() {
    let stderr = "not json\n{broken\n";
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
}

#[test]
fn parse_deny_mixed_diagnostics() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"unmaintained","code":"unmaintained","advisory":{"id":"RUSTSEC-2024-0001","package":"foo","title":"foo is old"},"labels":[],"graphs":[],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"error","message":"license rejected","code":"rejected","labels":[],"graphs":[{"Krate":{"name":"bar","version":"1.0.0"}}],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"warning","message":"duplicate","code":"duplicate","labels":[],"graphs":[{"Krate":{"name":"baz","version":"2.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(result.licenses.len(), 1);
    assert_eq!(result.bans.len(), 1);
}

// -- Upgrade table parser edge cases --

#[test]
fn parse_upgrade_table_lines_before_header_ignored() {
    let stdout = "\
some preamble text
another line
name   old req compatible latest  new req
====   ======= ========== ======  =======
serde  1.0.100 1.0.228    1.0.228 1.0.228
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "serde");
}

#[test]
fn parse_upgrade_table_row_too_few_columns_skipped() {
    let stdout = "\
name   old req compatible latest  new req
====   ======= ========== ======  =======
serde  1.0.100 1.0.228    1.0.228 1.0.228
bad    1.0.0   1.0.1
tokio  1.35.0  1.38.0     1.38.0  1.38.0
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].name, "serde");
    assert_eq!(entries[1].name, "tokio");
}

#[test]
fn parse_upgrade_table_multi_word_note() {
    let stdout = "\
name   old req compatible latest  new req note
====   ======= ========== ======  ======= ====
clap   3.0.0   3.2.25     4.6.0   3.2.25  pinned by user
";
    let entries = parse_upgrade_table(stdout);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].note.as_deref(), Some("pinned by user"));
}

// -- Deny parser edge cases --

#[test]
fn parse_deny_no_code_field_skipped() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"something","labels":[],"graphs":[{"Krate":{"name":"pkg","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

#[test]
fn parse_deny_no_severity_defaults_to_error() {
    let stderr = r#"{"type":"diagnostic","fields":{"message":"license rejected","code":"rejected","labels":[],"graphs":[{"Krate":{"name":"some-crate","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.licenses.len(), 1);
    assert_eq!(result.licenses[0].severity, "error");
}

#[test]
fn parse_deny_advisory_without_advisory_field_uses_code_as_id() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"warning","message":"crate is yanked","code":"yanked","labels":[],"graphs":[{"Krate":{"name":"old-crate","version":"0.1.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(result.advisories[0].id, "yanked");
    assert_eq!(result.advisories[0].title, "crate is yanked");
    assert_eq!(result.advisories[0].package, "old-crate");
}

#[test]
fn parse_deny_package_from_graphs_when_no_advisory_package() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"vulnerable","code":"vulnerability","advisory":{"id":"RUSTSEC-2024-0099","title":"vuln title"},"labels":[],"graphs":[{"Krate":{"name":"vuln-pkg","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 1);
    assert_eq!(result.advisories[0].package, "vuln-pkg");
    assert_eq!(result.advisories[0].id, "RUSTSEC-2024-0099");
}

#[test]
fn parse_deny_package_unknown_when_no_graphs_or_advisory_package() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"bad license","code":"unlicensed","labels":[],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.licenses.len(), 1);
    assert_eq!(result.licenses[0].package, "unknown");
}

#[test]
fn parse_deny_additional_advisory_codes() {
    // Test "vulnerability", "notice", "unsound" codes
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"vuln found","code":"vulnerability","advisory":{"id":"RUSTSEC-2024-0010","package":"pkg-a","title":"vuln"},"labels":[],"graphs":[],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"warning","message":"notice issued","code":"notice","advisory":{"id":"RUSTSEC-2024-0011","package":"pkg-b","title":"notice"},"labels":[],"graphs":[],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"error","message":"unsound code","code":"unsound","advisory":{"id":"RUSTSEC-2024-0012","package":"pkg-c","title":"unsound"},"labels":[],"graphs":[],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.advisories.len(), 3);
    assert_eq!(result.advisories[0].id, "RUSTSEC-2024-0010");
    assert_eq!(result.advisories[1].id, "RUSTSEC-2024-0011");
    assert_eq!(result.advisories[2].id, "RUSTSEC-2024-0012");
}

#[test]
fn parse_deny_additional_license_codes() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"no license","code":"unlicensed","labels":[],"graphs":[{"Krate":{"name":"pkg-a","version":"1.0.0"}}],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"warning","message":"missing field","code":"no-license-field","labels":[],"graphs":[{"Krate":{"name":"pkg-b","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.licenses.len(), 2);
    assert_eq!(result.licenses[0].package, "pkg-a");
    assert_eq!(result.licenses[1].package, "pkg-b");
}

#[test]
fn parse_deny_additional_ban_codes() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"crate not allowed","code":"not-allowed","labels":[],"graphs":[{"Krate":{"name":"pkg-a","version":"1.0.0"}}],"notes":[]}}
{"type":"diagnostic","fields":{"severity":"warning","message":"workspace dup","code":"workspace-duplicate","labels":[],"graphs":[{"Krate":{"name":"pkg-b","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.bans.len(), 2);
    assert_eq!(result.bans[0].package, "pkg-a");
    assert_eq!(result.bans[1].package, "pkg-b");
}

#[test]
fn parse_deny_git_source_underspecified() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"warning","message":"git source underspecified","code":"git-source-underspecified","labels":[],"graphs":[{"Krate":{"name":"git-dep","version":"0.1.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert_eq!(result.sources.len(), 1);
    assert_eq!(result.sources[0].package, "git-dep");
}

#[test]
fn parse_deny_unknown_code_ignored() {
    let stderr = r#"{"type":"diagnostic","fields":{"severity":"error","message":"something new","code":"future-check-type","labels":[],"graphs":[{"Krate":{"name":"pkg","version":"1.0.0"}}],"notes":[]}}"#;
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
    assert!(result.licenses.is_empty());
    assert!(result.bans.is_empty());
    assert!(result.sources.is_empty());
}

#[test]
fn parse_deny_fields_deserialization_failure_skipped() {
    // Valid JSON line but fields can't deserialize to DiagnosticFields
    let stderr = r#"{"type":"diagnostic","fields":"not an object"}"#;
    let result = parse_deny_output(stderr);
    assert!(result.advisories.is_empty());
}

// -- Report formatting tests --

#[test]
fn format_report_all_clean() {
    let report = DepsReport::default();
    let output = format_report(&report);
    assert!(output.contains("Compatible Upgrades"));
    assert!(output.contains("None"));
    assert!(output.contains("Advisories"));
    assert!(output.contains("License Issues"));
    assert!(output.contains("Duplicate Crates"));
    assert!(output.contains("Source Issues"));
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
    let output = format_report(&report);
    assert!(output.contains("Compatible Upgrades (1):"));
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
    let output = format_report(&report);
    assert!(output.contains("Breaking Upgrades (1):"));
    assert!(output.contains("cargo upgrade --incompatible"));
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
    let output = format_report(&report);
    assert!(output.contains("Advisories (1):"));
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
                BanEntry {
                    package: "hashbrown".into(),
                    message: "found 3 duplicate entries".into(),
                    severity: "warning".into(),
                },
                BanEntry {
                    package: "syn".into(),
                    message: "found 2 duplicate entries".into(),
                    severity: "error".into(),
                },
            ],
            ..Default::default()
        },
    };
    let output = format_report(&report);
    assert!(output.contains("Duplicate Crates:"));
    // Should NOT list individual crate names
    assert!(!output.contains("hashbrown"));
    assert!(!output.contains("syn"));
    // Should show severity totals on same line
    assert!(output.contains("1 error"));
    assert!(output.contains("1 warning"));
    assert!(output.contains("transitive, usually harmless"));
}

// -- has_issues tests --

#[test]
fn has_issues_clean_report() {
    let report = DepsReport::default();
    assert!(!has_issues(&report));
}

#[test]
fn has_issues_advisory_error() {
    let report = DepsReport {
        deny: DenyResult {
            advisories: vec![AdvisoryEntry {
                id: "RUSTSEC-2024-0001".into(),
                package: "foo".into(),
                severity: "error".into(),
                title: "bad".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_advisory_warning() {
    let report = DepsReport {
        deny: DenyResult {
            advisories: vec![AdvisoryEntry {
                id: "RUSTSEC-2024-0002".into(),
                package: "bar".into(),
                severity: "warning".into(),
                title: "meh".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_advisory_info_not_actionable() {
    let report = DepsReport {
        deny: DenyResult {
            advisories: vec![AdvisoryEntry {
                id: "RUSTSEC-2024-0003".into(),
                package: "baz".into(),
                severity: "info".into(),
                title: "fyi".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(!has_issues(&report));
}

#[test]
fn has_issues_license_error() {
    let report = DepsReport {
        deny: DenyResult {
            licenses: vec![LicenseEntry {
                package: "evil".into(),
                message: "rejected".into(),
                severity: "error".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_license_warning() {
    let report = DepsReport {
        deny: DenyResult {
            licenses: vec![LicenseEntry {
                package: "sketchy".into(),
                message: "unclear".into(),
                severity: "warning".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_ban_error() {
    let report = DepsReport {
        deny: DenyResult {
            bans: vec![BanEntry {
                package: "banned".into(),
                message: "not allowed".into(),
                severity: "error".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_ban_warning_not_actionable() {
    let report = DepsReport {
        deny: DenyResult {
            bans: vec![BanEntry {
                package: "dup".into(),
                message: "duplicate".into(),
                severity: "warning".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(!has_issues(&report));
}

#[test]
fn has_issues_source_error() {
    let report = DepsReport {
        deny: DenyResult {
            sources: vec![SourceEntry {
                package: "untrusted".into(),
                message: "bad source".into(),
                severity: "error".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

#[test]
fn has_issues_source_warning() {
    let report = DepsReport {
        deny: DenyResult {
            sources: vec![SourceEntry {
                package: "sketchy".into(),
                message: "underspecified".into(),
                severity: "warning".into(),
            }],
            ..Default::default()
        },
        ..Default::default()
    };
    assert!(has_issues(&report));
}

// -- Format: license issues with entries --

#[test]
fn format_report_with_license_issues() {
    let report = DepsReport {
        upgrades: UpgradeResult::default(),
        deny: DenyResult {
            licenses: vec![
                LicenseEntry {
                    package: "gpl-crate".into(),
                    message: "license rejected: GPL-3.0".into(),
                    severity: "error".into(),
                },
                LicenseEntry {
                    package: "unknown-lic".into(),
                    message: "no license field".into(),
                    severity: "warning".into(),
                },
            ],
            ..Default::default()
        },
    };
    let output = format_report(&report);
    assert!(output.contains("License Issues (2):"));
    assert!(output.contains("gpl-crate"));
    assert!(output.contains("unknown-lic"));
    assert!(output.contains("deny.toml"));
}

#[test]
fn format_report_with_source_issues() {
    let report = DepsReport {
        upgrades: UpgradeResult::default(),
        deny: DenyResult {
            sources: vec![SourceEntry {
                package: "sketchy-src".into(),
                message: "source not allowed".into(),
                severity: "error".into(),
            }],
            ..Default::default()
        },
    };
    let output = format_report(&report);
    assert!(output.contains("Source Issues (1):"));
    assert!(output.contains("sketchy-src"));
    assert!(output.contains("trusted sources"));
}

// -- Format: bans summary variants --

#[test]
fn format_report_bans_info_only() {
    let report = DepsReport {
        upgrades: UpgradeResult::default(),
        deny: DenyResult {
            bans: vec![BanEntry {
                package: "hashbrown".into(),
                message: "found 2 duplicate entries".into(),
                severity: "info".into(),
            }],
            ..Default::default()
        },
    };
    let output = format_report(&report);
    assert!(output.contains("Duplicate Crates:"));
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
                BanEntry {
                    package: "a".into(),
                    message: "banned".into(),
                    severity: "error".into(),
                },
                BanEntry {
                    package: "b".into(),
                    message: "banned".into(),
                    severity: "error".into(),
                },
                BanEntry {
                    package: "c".into(),
                    message: "dup".into(),
                    severity: "warning".into(),
                },
                BanEntry {
                    package: "d".into(),
                    message: "dup".into(),
                    severity: "warning".into(),
                },
                BanEntry {
                    package: "e".into(),
                    message: "dup".into(),
                    severity: "warning".into(),
                },
            ],
            ..Default::default()
        },
    };
    let output = format_report(&report);
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
    let output = format_report(&report);
    assert!(output.contains("Advisories (3):"));
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
    let output = format_report(&report);
    assert!(output.contains("Compatible Upgrades (2):"));
    assert!(output.contains("Breaking Upgrades (1):"));
    assert!(output.contains("serde"));
    assert!(output.contains("tokio-stream"));
    assert!(output.contains("clap"));
}

// -- Schema tests --

#[test]
fn schema_has_expected_fields() {
    use ops_extension::DataProvider;
    let schema = DepsProvider.schema();
    assert_eq!(schema.fields.len(), 6);
    let field_names: Vec<&str> = schema.fields.iter().map(|f| f.name).collect();
    assert!(field_names.contains(&"upgrades.compatible"));
    assert!(field_names.contains(&"upgrades.incompatible"));
    assert!(field_names.contains(&"deny.advisories"));
    assert!(field_names.contains(&"deny.licenses"));
    assert!(field_names.contains(&"deny.bans"));
    assert!(field_names.contains(&"deny.sources"));
}

// -- Serialization round-trip --

#[test]
fn deps_report_serialization_round_trip() {
    let report = DepsReport {
        upgrades: UpgradeResult {
            compatible: vec![UpgradeEntry {
                name: "serde".into(),
                old_req: "1.0.0".into(),
                compatible: "1.0.1".into(),
                latest: "1.0.1".into(),
                new_req: "1.0.1".into(),
                note: None,
            }],
            incompatible: vec![],
        },
        deny: DenyResult {
            advisories: vec![AdvisoryEntry {
                id: "RUSTSEC-2024-0001".into(),
                package: "foo".into(),
                severity: "error".into(),
                title: "bad thing".into(),
            }],
            licenses: vec![],
            bans: vec![],
            sources: vec![],
        },
    };
    let json = serde_json::to_value(&report).unwrap();
    let deserialized: DepsReport = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.upgrades.compatible.len(), 1);
    assert_eq!(deserialized.deny.advisories.len(), 1);
}

#[test]
fn parse_deny_output_skips_malformed_json_with_tracing() {
    use std::io::Write;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);
    impl Write for BufWriter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for BufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    let buf = BufWriter::default();
    let subscriber = tracing_subscriber::fmt()
        .with_writer(buf.clone())
        .with_max_level(tracing::Level::DEBUG)
        .with_ansi(false)
        .finish();

    tracing::subscriber::with_default(subscriber, || {
        // First line is malformed JSON; second has valid envelope but bad fields
        // shape. Both should be skipped; both should log.
        let stderr = "{not json\n{\"type\":\"diagnostic\",\"fields\":42}\n";
        let result = parse::parse_deny_output(stderr);
        assert!(result.advisories.is_empty());
    });

    let logged = String::from_utf8(buf.0.lock().unwrap().clone()).unwrap();
    assert!(logged.contains("ERR-1"), "missing ERR-1 marker: {logged}");
    assert!(
        logged.contains("malformed cargo-deny JSON line"),
        "missing malformed-line message: {logged}"
    );
    assert!(
        logged.contains("unexpected fields shape"),
        "missing fields-shape message: {logged}"
    );
}
