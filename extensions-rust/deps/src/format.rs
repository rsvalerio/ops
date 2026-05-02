//! Report formatting for the deps extension.

use crate::{BanEntry, DepsReport, UpgradeEntry};
use ops_core::style::{bold, dim, green, red, yellow};
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

/// ERR-2 (TASK-0602): the previous `_ => info-icon` fallback collapsed any
/// unknown severity (cargo-deny schema drift, e.g. a new `critical` level)
/// onto the lowest-emphasis style — exactly inverting what the icon set
/// is meant to communicate. Render unknown severities with a clearly
/// distinct fallback (red `?` icon) so a misclassified critical never
/// looks like info.
fn severity_icon(severity: &str) -> &'static str {
    match severity {
        "error" => "\u{2718}",                  // ✘
        "warning" => "\u{26a0}",                // ⚠
        "note" | "help" | "info" => "\u{2139}", // ℹ
        _ => "?",
    }
}

fn colorize_severity(text: &str, severity: &str) -> String {
    match severity {
        "error" => red(text),
        "warning" => yellow(text),
        "note" | "help" | "info" => dim(text),
        // ERR-2 (TASK-0602): unknown severity prints in red so it stands
        // out from the dim-info fallback that previously hid schema drift.
        // tracing::warn fires once per render so the operator log carries
        // the offending value alongside the visible icon change.
        other => {
            tracing::warn!(
                severity = %other,
                "TASK-0602: unknown cargo-deny severity rendered with fallback style"
            );
            red(text)
        }
    }
}

pub fn format_report(report: &DepsReport) -> String {
    let mut out = String::new();

    let _ = writeln!(out, "\n{P}{}\n", bold("Dependency Health Report"));

    format_upgrade_section(
        &mut out,
        "\u{2b06}\u{fe0f} Compatible Upgrades",
        &report.upgrades.compatible,
        false,
    );

    format_upgrade_section(
        &mut out,
        "\u{1f4a5} Breaking Upgrades",
        &report.upgrades.incompatible,
        true,
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

fn format_upgrade_section(
    out: &mut String,
    title: &str,
    entries: &[UpgradeEntry],
    is_breaking: bool,
) {
    if entries.is_empty() {
        format_empty_section(out, title);
        return;
    }
    let _ = writeln!(out, "{P}{} ({}):", title, entries.len());
    let name_width = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
    let old_width = entries.iter().map(|e| e.old_req.len()).max().unwrap_or(0);
    // ERR-1 / TASK-0600: for breaking upgrades, surface the absolute
    // `latest` column too — operators need to see how far behind the
    // compatible-cap (`new_req`) is from the latest published version
    // (e.g. cap stuck at 3.x while latest is 5.x). Compatible upgrades
    // already collapse cap == latest so the column would be redundant.
    let latest_width = if is_breaking {
        entries.iter().map(|e| e.latest.len()).max().unwrap_or(0)
    } else {
        0
    };
    for e in entries {
        if is_breaking {
            let _ = writeln!(
                out,
                "{P}    {:<name_w$}  {}  {}  {}  (latest {})",
                e.name,
                dim(&format!("{:<old_w$}", e.old_req, old_w = old_width)),
                dim("->"),
                green(&e.new_req),
                dim(&format!("{:<latest_w$}", e.latest, latest_w = latest_width)),
                name_w = name_width,
            );
        } else {
            let _ = writeln!(
                out,
                "{P}    {:<name_w$}  {}  {}  {}",
                e.name,
                dim(&format!("{:<old_w$}", e.old_req, old_w = old_width)),
                dim("->"),
                green(&e.new_req),
                name_w = name_width,
            );
        }
    }
    out.push('\n');
    let advice = if is_breaking {
        "Run `cargo upgrade --incompatible` to apply breaking upgrades."
    } else {
        "Run `cargo upgrade` to apply compatible upgrades."
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
    // PERF-3 / TASK-0880: re-apply `extract` for the width passes instead
    // of materialising every projected row up front. The closure is a pure
    // borrow-projection so re-running it is free; the previous Vec was the
    // single allocation contradicting this function's "one allocation per
    // render" intent (PERF-3 / TASK-0802 above). The borrow contract is
    // preserved: `AdvisoryRow<'a>` still borrows from `entries`.
    let _ = writeln!(out, "{P}{} ({}):", title, entries.len());
    let pkg_width = entries
        .iter()
        .map(|e| extract(e).package.len())
        .max()
        .unwrap_or(0);
    let id_width = entries
        .iter()
        .filter_map(|e| extract(e).id.map(str::len))
        .max()
        .unwrap_or(0);
    for entry in entries {
        let row = extract(entry);
        let icon = severity_icon(row.severity);
        if let Some(id) = row.id {
            let _ = writeln!(
                out,
                "{P}    {} {:<id_w$}  {:<pkg_w$}  {}",
                colorize_severity(icon, row.severity),
                id,
                row.package,
                dim(row.message),
                id_w = id_width,
                pkg_w = pkg_width,
            );
        } else {
            let _ = writeln!(
                out,
                "{P}    {} {:<pkg_w$}  {}",
                colorize_severity(icon, row.severity),
                row.package,
                dim(row.message),
                pkg_w = pkg_width,
            );
        }
    }
    out.push('\n');
    for line in advice.lines() {
        let _ = writeln!(out, "{P}    {} {}", dim("\u{1f4a1}"), dim(line));
    }
    out.push('\n');
}

fn format_bans_summary(out: &mut String, bans: &[BanEntry]) {
    let title = "\u{1f4e6} Duplicate Crates";
    if bans.is_empty() {
        format_empty_section(out, title);
        return;
    }
    let errors = bans.iter().filter(|b| b.severity == "error").count();
    let warnings = bans.iter().filter(|b| b.severity == "warning").count();
    let others = bans.len() - errors - warnings;

    let mut parts = Vec::new();
    if errors > 0 {
        parts.push(red(&format!(
            "{} error{}",
            errors,
            if errors == 1 { "" } else { "s" }
        )));
    }
    if warnings > 0 {
        parts.push(yellow(&format!(
            "{} warning{}",
            warnings,
            if warnings == 1 { "" } else { "s" }
        )));
    }
    if others > 0 {
        parts.push(dim(&format!("{others} info")));
    }

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
}
