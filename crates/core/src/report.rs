//! Report data model for "command output" style commands (e.g. `ops deps`).
//!
//! A [`Report`] is a themed checklist: a title, a list of [`ReportRow`]s (each a
//! status + label + result slot + optional detail lines), and a derived footer.
//! Rendering lives in the theme crate (`ConfigurableTheme::render_report`), which
//! maps each [`ReportStatus`] to the theme's configured icon and color via the
//! `[report]` theme block. This type is intentionally theme-agnostic so report
//! producers (`extensions-rust/deps`, `crates/cli/src/sec_cmd.rs`) build reports
//! without depending on the theme crate — the same separation [`StepLine`]
//! (`crate::output`) keeps between the runner's step data and its rendering.

/// Severity of a single report row.
///
/// Purely semantic — the theme owns the glyph and color via its `[report]`
/// config block (see `ThemeConfig::report`). `#[non_exhaustive]` so a future
/// severity (e.g. `Critical`) is additive; out-of-crate `match` sites need a
/// wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReportStatus {
    /// Section is clean / passed (e.g. "None").
    Ok,
    /// Informational finding, not actionable (e.g. available upgrades).
    Info,
    /// Non-fatal finding (e.g. duplicate crates).
    Warning,
    /// Actionable failure (e.g. a security advisory).
    Error,
}

/// One row of a report: a status, a label, a right-aligned result slot
/// (`"None"`, `"28 warnings"`), and optional pre-formatted detail lines.
///
/// `details` are rendered verbatim beneath the row (tables, advice) and never
/// receive the dotted-separator treatment — they carry their own indentation
/// and coloring. `#[non_exhaustive]`: construct via [`ReportRow::new`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct ReportRow {
    pub status: ReportStatus,
    pub label: String,
    pub result: String,
    pub details: Vec<String>,
}

impl ReportRow {
    /// Construct a row with no detail lines.
    #[must_use]
    pub fn new(status: ReportStatus, label: impl Into<String>, result: impl Into<String>) -> Self {
        Self {
            status,
            label: label.into(),
            result: result.into(),
            details: Vec::new(),
        }
    }

    /// Attach pre-formatted detail lines (rendered verbatim below the row).
    #[must_use]
    pub fn with_details(mut self, details: Vec<String>) -> Self {
        self.details = details;
        self
    }
}

/// A full report: a title plus a sequence of rows.
///
/// `#[non_exhaustive]`: construct via [`Report::new`] and [`Report::push`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Report {
    pub title: String,
    pub rows: Vec<ReportRow>,
}

impl Report {
    /// Construct an empty report with the given title.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            rows: Vec::new(),
        }
    }

    /// Append a row.
    pub fn push(&mut self, row: ReportRow) {
        self.rows.push(row);
    }

    /// Number of [`ReportStatus::Warning`] rows.
    #[must_use]
    pub fn warning_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.status == ReportStatus::Warning)
            .count()
    }

    /// Number of [`ReportStatus::Error`] rows.
    #[must_use]
    pub fn error_count(&self) -> usize {
        self.rows
            .iter()
            .filter(|r| r.status == ReportStatus::Error)
            .count()
    }

    /// Summary footer text mirroring the runner's "Done in …" line, but
    /// counting checks instead of timing:
    ///
    /// - all clean/info: `"Done 6 checks"`
    /// - warnings only:  `"Done 6 checks · 1 warning"`
    /// - with errors:    `"Done 6 checks · 2 errors, 1 warning"`
    #[must_use]
    pub fn footer_text(&self) -> String {
        let checks = self.rows.len();
        let mut text = format!("Done {checks} {}", pluralize(checks, "check", "checks"));

        let errors = self.error_count();
        let warnings = self.warning_count();
        let mut issues = Vec::new();
        if errors > 0 {
            issues.push(format!("{errors} {}", pluralize(errors, "error", "errors")));
        }
        if warnings > 0 {
            issues.push(format!(
                "{warnings} {}",
                pluralize(warnings, "warning", "warnings")
            ));
        }
        if !issues.is_empty() {
            text.push_str(" \u{b7} ");
            text.push_str(&issues.join(", "));
        }
        text
    }
}

const fn pluralize(n: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if n == 1 {
        singular
    } else {
        plural
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report_with(statuses: &[ReportStatus]) -> Report {
        let mut report = Report::new("Test Report");
        for (i, &status) in statuses.iter().enumerate() {
            report.push(ReportRow::new(status, format!("row {i}"), "result"));
        }
        report
    }

    #[test]
    fn footer_all_clean_counts_checks_only() {
        let report = report_with(&[ReportStatus::Ok, ReportStatus::Ok, ReportStatus::Info]);
        assert_eq!(report.footer_text(), "Done 3 checks");
    }

    #[test]
    fn footer_single_check_is_singular() {
        let report = report_with(&[ReportStatus::Ok]);
        assert_eq!(report.footer_text(), "Done 1 check");
    }

    #[test]
    fn footer_warnings_only() {
        let report = report_with(&[ReportStatus::Ok, ReportStatus::Warning]);
        assert_eq!(report.footer_text(), "Done 2 checks \u{b7} 1 warning");
    }

    #[test]
    fn footer_errors_listed_before_warnings_and_pluralized() {
        let report = report_with(&[
            ReportStatus::Error,
            ReportStatus::Error,
            ReportStatus::Warning,
            ReportStatus::Ok,
        ]);
        assert_eq!(
            report.footer_text(),
            "Done 4 checks \u{b7} 2 errors, 1 warning"
        );
    }

    #[test]
    fn with_details_attaches_lines() {
        let row = ReportRow::new(ReportStatus::Info, "label", "1 upgrade")
            .with_details(vec!["    detail".to_string()]);
        assert_eq!(row.details, vec!["    detail".to_string()]);
    }
}
