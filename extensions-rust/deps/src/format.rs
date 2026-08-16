//! Report construction for the deps extension.
//!
//! `build_report` turns a [`DepsReport`] into a theme-agnostic
//! [`ops_core::report::Report`]: one [`ReportRow`] per section (Compatible /
//! Breaking Upgrades, Advisories, License Issues, Duplicate Crates, Source
//! Issues). The theme (`ConfigurableTheme::render_report`) owns all icons,
//! colors, separators, and the footer — `ops deps` no longer hand-rolls its
//! layout, so it shares the exact rendering system the runner commands
//! (`ops verify`, `ops qa`) use.
//!
//! Each row carries a status (→ icon + color via the theme's `[report]` block),
//! a label, a right-aligned result slot (`"None"`, `"28 warnings"`), and
//! pre-formatted `details` (the per-entry tables and `💡` advice) that the
//! renderer emits verbatim beneath the row.

use crate::{BanEntry, DepsReport, UpgradeEntry};
use ops_core::report::{Report, ReportRow, ReportStatus};
use ops_core::style::{dim, green};
use std::borrow::Cow;
use std::fmt::Write as _;

/// Indent for detail/advice lines, sitting them under the row label. Detail
/// lines are emitted verbatim (no dotted separator), so they carry their own
/// indentation and coloring.
const DETAIL_INDENT: &str = "      ";

/// DUP-3 / TASK-0972: single source of truth for the severity → (icon, style)
/// mapping used by the per-entry detail rows, and → [`ReportStatus`] for the
/// row-level status/result rollup.
///
/// ERR-2 / TASK-0602: any cargo-deny severity outside the known set classifies
/// into [`SeverityClass::Unknown`], rendering with a red `?` icon plus a
/// one-shot `tracing::warn!` so schema drift is observable instead of hiding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SeverityClass {
    Error,
    Warning,
    Info,
    Unknown,
}

impl SeverityClass {
    fn classify(severity: &str) -> Self {
        match severity {
            "error" => Self::Error,
            "warning" => Self::Warning,
            "note" | "help" | "info" => Self::Info,
            _ => Self::Unknown,
        }
    }

    fn icon(self) -> &'static str {
        match self {
            Self::Error => "\u{2718}",   // ✘
            Self::Warning => "\u{26a0}", // ⚠
            Self::Info => "\u{2139}",    // ℹ
            Self::Unknown => "?",
        }
    }

    /// Color applied to the per-entry detail icon. The row-level result slot is
    /// colored by the theme from the row's [`ReportStatus`]; this only styles
    /// the inline detail glyph.
    fn style(self, text: &str) -> Cow<'_, str> {
        use ops_core::style::{dim, red, yellow};
        match self {
            Self::Error | Self::Unknown => red(text),
            Self::Warning => yellow(text),
            Self::Info => dim(text),
        }
    }

    fn label(self) -> (&'static str, &'static str) {
        match self {
            Self::Error => ("error", "errors"),
            Self::Warning => ("warning", "warnings"),
            Self::Info => ("info", "info"),
            Self::Unknown => ("unknown severity", "unknown severities"),
        }
    }

    /// Map to the report status that drives the row icon + color.
    /// Unknown severities are treated as errors (fail-loud), mirroring the
    /// `has_issues` gate's fail-closed posture.
    fn report_status(self) -> ReportStatus {
        match self {
            Self::Error | Self::Unknown => ReportStatus::Error,
            Self::Warning => ReportStatus::Warning,
            Self::Info => ReportStatus::Info,
        }
    }
}

/// Worst-to-least order used by the result-slot count rollup.
const SUMMARY_CLASSES: [SeverityClass; 4] = [
    SeverityClass::Error,
    SeverityClass::Warning,
    SeverityClass::Info,
    SeverityClass::Unknown,
];

/// Severity rank for picking the most severe [`ReportStatus`] in a section.
fn status_rank(status: ReportStatus) -> u8 {
    match status {
        ReportStatus::Error => 3,
        ReportStatus::Warning => 2,
        ReportStatus::Info => 1,
        ReportStatus::Ok => 0,
        // `ReportStatus` is `#[non_exhaustive]`; treat any future severity as
        // most-severe so it cannot be silently downgraded.
        _ => u8::MAX,
    }
}

/// Roll a set of severity classes up into a single row status and a plain
/// (uncolored) count summary like `"1 error, 2 warnings"`. The theme colors
/// the whole result slot from the returned status, so the string stays plain.
fn rollup(classes: &[SeverityClass]) -> (ReportStatus, String) {
    let mut counts = [0usize; SUMMARY_CLASSES.len()];
    for c in classes {
        // A class outside SUMMARY_CLASSES contributes no count rather than
        // panicking the report over a presentation detail.
        if let Some(idx) = SUMMARY_CLASSES.iter().position(|x| x == c) {
            counts[idx] += 1;
        }
    }
    let parts: Vec<String> = SUMMARY_CLASSES
        .iter()
        .zip(&counts)
        .filter(|(_, &n)| n > 0)
        .map(|(&class, &n)| {
            let (singular, plural) = class.label();
            format!("{n} {}", if n == 1 { singular } else { plural })
        })
        .collect();

    // Row status is the most severe class present (error/unknown → Error;
    // warning → Warning; info → Info). The empty case is handled by callers.
    let status = classes
        .iter()
        .map(|c| c.report_status())
        .max_by_key(|s| status_rank(*s))
        .unwrap_or(ReportStatus::Ok);
    (status, parts.join(", "))
}

/// Build the structured dependency health report. Rendering is the theme's job.
#[must_use]
pub fn build_report(report: &DepsReport) -> Report {
    let mut out = Report::new("Dependency Health Report");

    out.push(upgrade_row(
        "Compatible Upgrades",
        &report.upgrades.compatible,
        UpgradeKind::Compatible,
    ));
    out.push(upgrade_row(
        "Breaking Upgrades",
        &report.upgrades.incompatible,
        UpgradeKind::Breaking,
    ));

    // Advisories — id column in front of the package column.
    out.push(severity_row(
        "Advisories",
        &report.deny.advisories,
        |a| AdvisoryRow {
            id: Some(&a.id),
            package: &a.package,
            message: &a.title,
            severity: &a.severity,
        },
        "Run `cargo deny check advisories` for details. Update affected crates or add exceptions to deny.toml.",
    ));

    out.push(severity_row(
        "License Issues",
        &report.deny.licenses,
        |l| AdvisoryRow {
            id: None,
            package: &l.package,
            message: &l.message,
            severity: &l.severity,
        },
        "Run `cargo deny check licenses` for details. Configure allowed licenses in deny.toml.",
    ));

    out.push(bans_row(&report.deny.bans));

    out.push(severity_row(
        "Source Issues",
        &report.deny.sources,
        |s| AdvisoryRow {
            id: None,
            package: &s.package,
            message: &s.message,
            severity: &s.severity,
        },
        "Configure trusted sources in deny.toml [sources] section.",
    ));

    out
}

#[derive(Clone, Copy)]
enum UpgradeKind {
    Compatible,
    Breaking,
}

fn upgrade_row(title: &str, entries: &[UpgradeEntry], kind: UpgradeKind) -> ReportRow {
    if entries.is_empty() {
        return ReportRow::new(ReportStatus::Ok, title, "None");
    }

    let result = format!(
        "{} {}",
        entries.len(),
        if entries.len() == 1 {
            "upgrade"
        } else {
            "upgrades"
        }
    );

    let name_width = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    let old_width = entries.iter().map(|e| e.old_req.len()).max().unwrap_or(0);
    let latest_width = match kind {
        UpgradeKind::Breaking => entries.iter().map(|e| e.latest.len()).max().unwrap_or(0),
        UpgradeKind::Compatible => 0,
    };

    let mut details = Vec::with_capacity(entries.len() + 1);
    for e in entries {
        let suffix = match kind {
            UpgradeKind::Breaking => format!(
                "  (latest {})",
                dim(&format!("{:<w$}", e.latest, w = latest_width))
            ),
            UpgradeKind::Compatible => String::new(),
        };
        let mut line = String::new();
        let _ = write!(
            line,
            "{DETAIL_INDENT}{:<name_w$}  {}  {}  {}{}",
            e.name,
            dim(&format!("{:<old_w$}", e.old_req, old_w = old_width)),
            dim("->"),
            green(&e.new_req),
            suffix,
            name_w = name_width,
        );
        details.push(line);
    }

    let advice = match kind {
        UpgradeKind::Breaking => "Run `cargo upgrade --incompatible` to apply breaking upgrades.",
        UpgradeKind::Compatible => "Run `cargo upgrade` to apply compatible upgrades.",
    };
    details.push(format!(
        "{DETAIL_INDENT}{} {}",
        dim("\u{1f4a1}"),
        dim(advice)
    ));

    ReportRow::new(ReportStatus::Info, title, result).with_details(details)
}

/// One row in a severity-bearing section. DUP-1 (TASK-0801): unifies the
/// advisories shape (with an `id` column) and the licenses/sources shape
/// (without). `id` is `Some` only for advisories.
struct AdvisoryRow<'a> {
    id: Option<&'a str>,
    package: &'a str,
    message: &'a str,
    severity: &'a str,
}

fn severity_row<T, F>(title: &str, entries: &[T], extract: F, advice: &str) -> ReportRow
where
    F: for<'a> Fn(&'a T) -> AdvisoryRow<'a>,
{
    if entries.is_empty() {
        return ReportRow::new(ReportStatus::Ok, title, "None");
    }

    let classes: Vec<SeverityClass> = entries
        .iter()
        .map(|e| SeverityClass::classify(extract(e).severity))
        .collect();
    let (status, result) = rollup(&classes);

    let pkg_w = entries
        .iter()
        .map(|e| extract(e).package.len())
        .max()
        .unwrap_or(0);
    let id_w = entries
        .iter()
        .filter_map(|e| extract(e).id.map(str::len))
        .max()
        .unwrap_or(0);

    let mut details = Vec::with_capacity(entries.len() + 2);
    let mut warned_unknown = false;
    for entry in entries {
        let row = extract(entry);
        let class = SeverityClass::classify(row.severity);
        if class == SeverityClass::Unknown && !warned_unknown {
            warned_unknown = true;
            tracing::warn!(
                severity = %row.severity,
                "TASK-0602: unknown cargo-deny severity rendered with fallback style"
            );
        }
        let mut line = String::new();
        let _ = write!(line, "{DETAIL_INDENT}{} ", class.style(class.icon()));
        if let Some(id) = row.id {
            let _ = write!(line, "{id:<id_w$}  ");
        }
        let _ = write!(line, "{:<pkg_w$}  {}", row.package, dim(row.message));
        details.push(line);
    }
    for line in advice.lines() {
        details.push(format!("{DETAIL_INDENT}{} {}", dim("\u{1f4a1}"), dim(line)));
    }

    ReportRow::new(status, title, result).with_details(details)
}

fn bans_row(bans: &[BanEntry]) -> ReportRow {
    let title = "Duplicate Crates";
    if bans.is_empty() {
        return ReportRow::new(ReportStatus::Ok, title, "None");
    }

    let classes: Vec<SeverityClass> = bans
        .iter()
        .map(|b| SeverityClass::classify(&b.severity))
        .collect();
    let (status, result) = rollup(&classes);

    let details = vec![format!(
        "{DETAIL_INDENT}{}",
        dim("(transitive, usually harmless)")
    )];

    ReportRow::new(status, title, result).with_details(details)
}

#[cfg(test)]
mod helper_tests {
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
}
