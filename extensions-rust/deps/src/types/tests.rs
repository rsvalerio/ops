//! Serialization contract for the report types.

use super::*;

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
            unused_license_allowances: vec![],
            bans: vec![],
            sources: vec![],
        },
        unused: UnusedDepsResult::Checked {
            entries: vec![UnusedDepEntry {
                package: "app".into(),
                manifest_path: "./Cargo.toml".into(),
                dependency: "serde".into(),
            }],
        },
    };
    let json = serde_json::to_value(&report).unwrap();
    let deserialized: DepsReport = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.upgrades.compatible.len(), 1);
    assert_eq!(deserialized.deny.advisories.len(), 1);
    assert_eq!(deserialized.unused, report.unused);
}

/// The skipped check survives the cache round trip as skipped, so a cached
/// report never re-renders a check that did not run as "None".
#[test]
fn unused_not_installed_round_trips() {
    let report = DepsReport {
        unused: UnusedDepsResult::NotInstalled,
        ..Default::default()
    };
    let json = serde_json::to_value(&report).unwrap();
    assert_eq!(json["unused"]["status"], "not_installed");
    let deserialized: DepsReport = serde_json::from_value(json).unwrap();
    assert_eq!(deserialized.unused, UnusedDepsResult::NotInstalled);
}

/// A payload cached before `unused_license_allowances` existed still decodes,
/// with the section empty, rather than forcing an `ops deps --refresh`.
#[test]
fn deny_result_without_unused_license_allowances_decodes() {
    let json = serde_json::json!({
        "advisories": [],
        "licenses": [],
        "bans": [],
        "sources": [],
    });
    let deny: DenyResult = serde_json::from_value(json).unwrap();
    assert!(deny.unused_license_allowances.is_empty());
}
