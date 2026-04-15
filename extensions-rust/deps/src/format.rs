//! Report formatting for the deps extension.

use crate::{AdvisoryEntry, BanEntry, DepsReport, UpgradeEntry};
use ops_core::style::{bold, dim, green, red, yellow};

const P: &str = "  "; // left padding for the entire report

fn severity_icon(severity: &str) -> &'static str {
    match severity {
        "error" => "\u{2718}",   // ✘
        "warning" => "\u{26a0}", // ⚠
        _ => "\u{2139}",         // ℹ
    }
}

fn colorize_severity(text: &str, severity: &str) -> String {
    match severity {
        "error" => red(text),
        "warning" => yellow(text),
        _ => dim(text),
    }
}

pub fn format_report(report: &DepsReport) -> String {
    let mut out = String::new();

    out.push_str(&format!("\n{P}{}\n\n", bold("Dependency Health Report")));

    // Compatible upgrades
    format_upgrade_section(
        &mut out,
        "\u{2b06}\u{fe0f} Compatible Upgrades",
        &report.upgrades.compatible,
        false,
    );

    // Breaking upgrades
    format_upgrade_section(
        &mut out,
        "\u{1f4a5} Breaking Upgrades",
        &report.upgrades.incompatible,
        true,
    );

    // Advisories
    format_advisories(&mut out, &report.deny.advisories);

    // License issues
    format_deny_section(
        &mut out,
        "\u{1f4dc} License Issues",
        &report.deny.licenses,
        |l| (&l.package, &l.message, &l.severity),
        "Run `cargo deny check licenses` for details. Configure allowed licenses in deny.toml.",
    );

    // Duplicate crates (bans) — totals only
    format_bans_summary(&mut out, &report.deny.bans);

    // Source issues
    format_deny_section(
        &mut out,
        "\u{1f310} Source Issues",
        &report.deny.sources,
        |s| (&s.package, &s.message, &s.severity),
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
        out.push_str(&format!("{P}{} {}\n\n", title, green("\u{2714} None")));
    } else {
        out.push_str(&format!("{P}{} ({}):\n", title, entries.len()));
        let name_width = entries.iter().map(|e| e.name.len()).max().unwrap_or(0);
        let old_width = entries.iter().map(|e| e.old_req.len()).max().unwrap_or(0);
        for e in entries {
            out.push_str(&format!(
                "{P}    {:<name_w$}  {}  {}  {}\n",
                e.name,
                dim(&format!("{:<old_w$}", e.old_req, old_w = old_width)),
                dim("->"),
                green(&e.new_req),
                name_w = name_width,
            ));
        }
        out.push('\n');
        let advice = if is_breaking {
            "Run `cargo upgrade --incompatible` to apply breaking upgrades."
        } else {
            "Run `cargo upgrade` to apply compatible upgrades."
        };
        out.push_str(&format!("{P}    {} {}\n\n", dim("\u{1f4a1}"), dim(advice)));
    }
}

fn format_advisories(out: &mut String, advisories: &[AdvisoryEntry]) {
    if advisories.is_empty() {
        out.push_str(&format!(
            "{P}\u{1f6e1}\u{fe0f} Advisories {}\n\n",
            green("\u{2714} None")
        ));
    } else {
        out.push_str(&format!(
            "{P}\u{1f6e1}\u{fe0f} Advisories ({}):\n",
            advisories.len()
        ));
        let id_width = advisories.iter().map(|a| a.id.len()).max().unwrap_or(0);
        let pkg_width = advisories
            .iter()
            .map(|a| a.package.len())
            .max()
            .unwrap_or(0);
        for a in advisories {
            let icon = severity_icon(&a.severity);
            out.push_str(&format!(
                "{P}    {} {:<id_w$}  {:<pkg_w$}  {}\n",
                colorize_severity(icon, &a.severity),
                a.id,
                a.package,
                dim(&a.title),
                id_w = id_width,
                pkg_w = pkg_width,
            ));
        }
        out.push('\n');
        out.push_str(&format!(
            "{P}    {} {}\n\n",
            dim("\u{1f4a1}"),
            dim("Run `cargo deny check advisories` for details. Update affected crates or add exceptions to deny.toml.")
        ));
    }
}

fn format_bans_summary(out: &mut String, bans: &[BanEntry]) {
    let title = "\u{1f4e6} Duplicate Crates";
    if bans.is_empty() {
        out.push_str(&format!("{P}{} {}\n\n", title, green("\u{2714} None")));
    } else {
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
            parts.push(dim(&format!("{} info", others)));
        }

        out.push_str(&format!(
            "{P}{}: {} {}\n\n",
            title,
            parts.join(", "),
            dim("(transitive, usually harmless)")
        ));
    }
}

fn format_deny_section<T, F>(out: &mut String, title: &str, entries: &[T], extract: F, advice: &str)
where
    F: Fn(&T) -> (&String, &String, &String),
{
    if entries.is_empty() {
        out.push_str(&format!("{P}{} {}\n\n", title, green("\u{2714} None")));
    } else {
        out.push_str(&format!("{P}{} ({}):\n", title, entries.len()));
        let pkg_width = entries
            .iter()
            .map(|e| extract(e).0.len())
            .max()
            .unwrap_or(0);
        for e in entries {
            let (pkg, msg, sev) = extract(e);
            let icon = severity_icon(sev);
            out.push_str(&format!(
                "{P}    {} {:<pkg_w$}  {}\n",
                colorize_severity(icon, sev),
                pkg,
                dim(msg),
                pkg_w = pkg_width,
            ));
        }
        out.push('\n');
        for line in advice.lines() {
            out.push_str(&format!("{P}    {} {}\n", dim("\u{1f4a1}"), dim(line)));
        }
        out.push('\n');
    }
}
