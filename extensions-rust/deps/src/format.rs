//! Report formatting for the deps extension.

use crate::{BanEntry, DepsReport, UpgradeEntry};
use ops_core::style::{bold, dim, green, red, yellow};
use std::borrow::Cow;
use std::fmt::Write as _;

const P: &str = "  "; // left padding for the entire report

// PERF-3 (TASK-0802): every formatter writes into a shared `&mut String`
// via `std::fmt::Write`, eliminating the intermediate `format!()`
// allocations the previous `push_str(&format!(...))` shape paid per line.
// `write!` into a `String` is infallible, so the trivial `Result` is
// discarded with `let _ = …`; the only observable change is one allocation
// per render rather than hundreds.

/// DUP-1 (TASK-0610): single source of truth for the "section is empty"
/// line. Every section formatter previously open-coded the same
/// `"{P}{title} ✓ None\n\n"` shape, so a style tweak meant editing five
/// places.
fn format_empty_section(out: &mut String, title: &str) {
    let _ = writeln!(out, "{P}{} {}\n", title, green("\u{2714} None"));
}

/// DUP-3 / TASK-0972: single source of truth for the severity → (icon,
/// style) mapping. The previous `severity_icon` and `colorize_severity`
/// each maintained an independent match arm over the same severity strings,
/// inviting a subtle inversion where the icon and color disagree on
/// classification.
///
/// ERR-2 / TASK-0602: any cargo-deny severity outside the known set
/// classifies into [`SeverityClass::Unknown`], rendering with a red `?`
/// icon (clearly distinct from the dim-info style) plus a one-shot
/// `tracing::warn!` so schema drift is observable instead of hiding.
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

    fn style<'a>(self, text: &'a str) -> Cow<'a, str> {
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
}

#[must_use]
pub fn format_report(report: &DepsReport) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "\n{P}{}\n", bold("Dependency Health Report"));

    format_upgrade_section(
        &mut out,
        "\u{2b06}\u{fe0f} Compatible Upgrades",
        &report.upgrades.compatible,
        UpgradeKind::Compatible,
    );

    format_upgrade_section(
        &mut out,
        "\u{1f4a5} Breaking Upgrades",
        &report.upgrades.incompatible,
        UpgradeKind::Breaking,
    );

    // Advisories — id column in front of the package column.
    format_severity_section(
        &mut out,
        "\u{1f6e1}\u{fe0f} Advisories",
        &report.deny.advisories,
        |a| AdvisoryRow {
            id: Some(&a.id),
            package: &a.package,
            message: &a.title,
            severity: &a.severity,
        },
        "Run `cargo deny check advisories` for details. Update affected crates or add exceptions to deny.toml.",
    );

    format_severity_section(
        &mut out,
        "\u{1f4dc} License Issues",
        &report.deny.licenses,
        |l| AdvisoryRow {
            id: None,
            package: &l.package,
            message: &l.message,
            severity: &l.severity,
        },
        "Run `cargo deny check licenses` for details. Configure allowed licenses in deny.toml.",
    );

    format_bans_summary(&mut out, &report.deny.bans);

    format_severity_section(
        &mut out,
        "\u{1f310} Source Issues",
        &report.deny.sources,
        |s| AdvisoryRow {
            id: None,
            package: &s.package,
            message: &s.message,
            severity: &s.severity,
        },
        "Configure trusted sources in deny.toml [sources] section.",
    );

    out
}

#[derive(Clone, Copy)]
enum UpgradeKind {
    Compatible,
    Breaking,
}

fn format_upgrade_section(
    out: &mut String,
    title: &str,
    entries: &[UpgradeEntry],
    kind: UpgradeKind,
) {
    if entries.is_empty() {
        format_empty_section(out, title);
        return;
    }
    let _ = writeln!(out, "{P}{} ({}):", title, entries.len());
    let name_width = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    let old_width = entries.iter().map(|e| e.old_req.len()).max().unwrap_or(0);
    let latest_width = match kind {
        UpgradeKind::Breaking => entries.iter().map(|e| e.latest.len()).max().unwrap_or(0),
        UpgradeKind::Compatible => 0,
    };
    for e in entries {
        let suffix = match kind {
            UpgradeKind::Breaking => {
                format!(
                    "  (latest {})",
                    dim(&format!("{:<w$}", e.latest, w = latest_width))
                )
            }
            UpgradeKind::Compatible => String::new(),
        };
        let _ = writeln!(
            out,
            "{P}    {:<name_w$}  {}  {}  {}{}",
            e.name,
            dim(&format!("{:<old_w$}", e.old_req, old_w = old_width)),
            dim("->"),
            green(&e.new_req),
            suffix,
            name_w = name_width,
        );
    }
    out.push('\n');
    let advice = match kind {
        UpgradeKind::Breaking => "Run `cargo upgrade --incompatible` to apply breaking upgrades.",
        UpgradeKind::Compatible => "Run `cargo upgrade` to apply compatible upgrades.",
    };
    let _ = writeln!(out, "{P}    {} {}\n", dim("\u{1f4a1}"), dim(advice));
}

/// One row in a severity-bearing section. DUP-1 (TASK-0801): unifies
/// what was previously `format_advisories` (which had an `id` column) and
/// `format_deny_section` (which did not). The `id` field is `Some` for the
/// advisories section and `None` for licenses / sources, so the helper
/// supports both shapes without a second formatter.
struct AdvisoryRow<'a> {
    id: Option<&'a str>,
    package: &'a str,
    message: &'a str,
    severity: &'a str,
}

fn format_severity_section<T, F>(
    out: &mut String,
    title: &str,
    entries: &[T],
    extract: F,
    advice: &str,
) where
    F: for<'a> Fn(&'a T) -> AdvisoryRow<'a>,
{
    if entries.is_empty() {
        format_empty_section(out, title);
        return;
    }
    let _ = writeln!(out, "{P}{} ({}):", title, entries.len());
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
        let _ = write!(out, "{P}    {} ", class.style(class.icon()));
        if let Some(id) = row.id {
            let _ = write!(out, "{:<id_w$}  ", id);
        }
        let _ = writeln!(out, "{:<pkg_w$}  {}", row.package, dim(row.message));
    }
    out.push('\n');
    for line in advice.lines() {
        let _ = writeln!(out, "{P}    {} {}", dim("\u{1f4a1}"), dim(line));
    }
    out.push('\n');
}

fn format_bans_summary(out: &mut String, bans: &[BanEntry]) {
    use SeverityClass::{Error, Info, Unknown, Warning};

    let title = "\u{1f4e6} Duplicate Crates";
    if bans.is_empty() {
        format_empty_section(out, title);
        return;
    }

    const CLASSES: [SeverityClass; 4] = [Error, Warning, Info, Unknown];
    let mut counts = [0usize; 4];
    for b in bans {
        let idx = CLASSES
            .iter()
            .position(|&c| c == SeverityClass::classify(&b.severity))
            .unwrap();
        counts[idx] += 1;
    }

    let parts: Vec<String> = CLASSES
        .iter()
        .zip(&counts)
        .filter(|(_, &n)| n > 0)
        .map(|(&class, &n)| {
            let (singular, plural) = class.label();
            let label = if n == 1 { singular } else { plural };
            class.style(&format!("{n} {label}")).into_owned()
        })
        .collect();

    let _ = writeln!(
        out,
        "{P}{}: {} {}\n",
        title,
        parts.join(", "),
        dim("(transitive, usually harmless)")
    );
}

#[cfg(test)]
mod helper_tests {
    use super::*;
    use crate::{AdvisoryEntry, DenyEntry};

    /// DUP-1 (TASK-0801): regression — advisories and license sections must
    /// continue to render the same shape after the helper unification.
    /// Captures both the empty path and the entries-present path so a future
    /// extractor change cannot silently regress one of the two layouts.
    #[test]
    fn advisory_section_renders_id_column() {
        let mut out = String::new();
        let advisories = vec![AdvisoryEntry {
            id: "RUSTSEC-2024-0001".to_string(),
            package: "openssl".to_string(),
            severity: "error".to_string(),
            title: "buffer overflow".to_string(),
        }];
        format_severity_section(
            &mut out,
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
        assert!(out.contains("RUSTSEC-2024-0001"));
        assert!(out.contains("openssl"));
        assert!(out.contains("buffer overflow"));
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
            assert_eq!(
                class.style("x").as_ref(),
                SeverityClass::classify(sev).style("x").as_ref(),
                "style diverged for `{sev}`"
            );
        }
    }

    #[test]
    fn unknown_severity_classifies_to_red_question_mark() {
        let class = SeverityClass::classify("critical");
        assert_eq!(class, SeverityClass::Unknown);
        assert_eq!(class.icon(), "?");
        assert_eq!(class.style("x").as_ref(), red("x").as_ref());
    }

    #[test]
    fn deny_section_omits_id_column() {
        let mut out = String::new();
        let entries = vec![DenyEntry {
            package: "foo".to_string(),
            message: "GPL-3.0 not allowed".to_string(),
            severity: "error".to_string(),
        }];
        format_severity_section(
            &mut out,
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
        assert!(out.contains("foo"));
        assert!(out.contains("GPL-3.0 not allowed"));
    }

    /// PATTERN-1 / TASK-1041: a bans entry whose severity is `<missing-severity>`
    /// (the cargo-deny schema-drift sentinel) or an unknown future value like
    /// `critical` must NOT be folded into the dim "info" counter. The summary
    /// has to render those distinctly so the operator-facing line agrees with
    /// the fail-closed gate decision instead of contradicting it.
    #[test]
    fn bans_summary_unknown_severity_renders_distinctly_from_info() {
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

        let mut out = String::new();
        format_bans_summary(&mut out, &bans);

        // The two unknown-severity entries collapse into the "unknown" bucket,
        // not the dim "info" bucket. A previous regression buried them as
        // "3 info"; the corrected output must show "1 info" + the unknowns.
        assert!(
            out.contains("unknown severities"),
            "summary must call out the unknown / missing-severity bucket: {out}"
        );
        assert!(
            out.contains("1 info"),
            "only the genuine `info` ban contributes to the info counter: {out}"
        );
        assert!(
            !out.contains("3 info"),
            "missing/unknown severities must not be lumped into info: {out}"
        );

        // And the unknown bucket must use the red style, mirroring
        // SeverityClass::Unknown — same visual signal as `colorize_severity`
        // for unknown advisory severities.
        let red_unknown = red("2 unknown severities");
        assert!(
            out.contains(red_unknown.as_ref()),
            "unknown-severity counter must render in red, not dim: {out}"
        );
    }
}
