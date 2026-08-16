//! Tests for the individual row builders and the severity rollup, driven
//! directly rather than through `build_report`.

use super::*;
use crate::{AdvisoryEntry, DenyEntry};

/// DUP-1 (TASK-0801): advisories carry an id column in their detail rows;
/// licenses/sources do not. Pin both shapes through `build_report`'s row
/// builder so a future extractor change cannot silently regress one layout.
#[test]
fn advisory_row_renders_id_column_in_details() {
    let advisories = vec![AdvisoryEntry {
        id: "RUSTSEC-2024-0001".to_string(),
        package: "openssl".to_string(),
        severity: "error".to_string(),
        title: "buffer overflow".to_string(),
    }];
    let row = severity_row(
        "\u{1f6e1}\u{fe0f} Advisories",
        &advisories,
        |a| AdvisoryRow {
            id: Some(&a.id),
            package: &a.package,
            message: &a.title,
            severity: &a.severity,
        },
        "advice",
    );
    assert_eq!(row.status, ReportStatus::Error);
    assert_eq!(row.result, "1 error");
    let body = row.details.join("\n");
    assert!(body.contains("RUSTSEC-2024-0001"));
    assert!(body.contains("openssl"));
    assert!(body.contains("buffer overflow"));
}

#[test]
fn license_row_omits_id_column() {
    let entries = vec![DenyEntry {
        package: "foo".to_string(),
        message: "GPL-3.0 not allowed".to_string(),
        severity: "error".to_string(),
    }];
    let row = severity_row(
        "\u{1f4dc} License Issues",
        &entries,
        |l| AdvisoryRow {
            id: None,
            package: &l.package,
            message: &l.message,
            severity: &l.severity,
        },
        "advice",
    );
    let body = row.details.join("\n");
    assert!(body.contains("foo"));
    assert!(body.contains("GPL-3.0 not allowed"));
    assert!(!body.contains("RUSTSEC"));
}

#[test]
fn empty_section_is_ok_none() {
    let entries: Vec<DenyEntry> = vec![];
    let row = severity_row(
        "\u{1f4dc} License Issues",
        &entries,
        |l| AdvisoryRow {
            id: None,
            package: &l.package,
            message: &l.message,
            severity: &l.severity,
        },
        "advice",
    );
    assert_eq!(row.status, ReportStatus::Ok);
    assert_eq!(row.result, "None");
    assert!(row.details.is_empty());
}

#[test]
fn known_severities_round_trip_through_classifier() {
    for sev in ["error", "warning", "note", "help", "info"] {
        let class = SeverityClass::classify(sev);
        let icon_class = match class.icon() {
            "\u{2718}" => SeverityClass::Error,
            "\u{26a0}" => SeverityClass::Warning,
            "\u{2139}" => SeverityClass::Info,
            other => panic!("unexpected icon `{other}` for severity `{sev}`"),
        };
        assert_eq!(
            class, icon_class,
            "icon and classifier disagree for severity `{sev}`"
        );
    }
}

#[test]
fn unknown_severity_classifies_to_error_status() {
    let class = SeverityClass::classify("critical");
    assert_eq!(class, SeverityClass::Unknown);
    assert_eq!(class.icon(), "?");
    assert_eq!(class.report_status(), ReportStatus::Error);
}

/// PATTERN-1 / TASK-1041: a bans entry whose severity is the cargo-deny
/// schema-drift sentinel or an unknown future value like `critical` must NOT
/// be folded into the `info` counter. The rollup has to surface those in the
/// `unknown` bucket so the result slot agrees with the fail-closed gate.
#[test]
fn bans_rollup_keeps_unknown_distinct_from_info() {
    let bans = vec![
        BanEntry(DenyEntry {
            package: "dup-a".to_string(),
            message: "duplicate".to_string(),
            severity: crate::parse::MISSING_SEVERITY_SENTINEL.to_string(),
        }),
        BanEntry(DenyEntry {
            package: "dup-b".to_string(),
            message: "duplicate".to_string(),
            severity: "critical".to_string(),
        }),
        BanEntry(DenyEntry {
            package: "dup-c".to_string(),
            message: "duplicate".to_string(),
            severity: "info".to_string(),
        }),
    ];

    let row = bans_row(&bans);
    // Two unknown-severity entries land in the unknown bucket, one genuine
    // info entry in the info bucket. Any unknown forces an Error status.
    assert_eq!(row.status, ReportStatus::Error);
    assert!(
        row.result.contains("2 unknown severities"),
        "result must call out the unknown bucket: {}",
        row.result
    );
    assert!(
        row.result.contains("1 info"),
        "only the genuine info ban contributes to the info counter: {}",
        row.result
    );
    assert!(
        !row.result.contains("3 info"),
        "missing/unknown severities must not be lumped into info: {}",
        row.result
    );
}
