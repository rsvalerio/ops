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
