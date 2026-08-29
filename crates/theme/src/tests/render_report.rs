//! Report rendering: the `render_slot` seam shared with the runner step line,
//! `render_report`, and the `render_summary_text` split. These pin the
//! "command output" path (`ops deps`) onto the same theme machinery the runner
//! commands use.

use super::*;
use crate::SlotLine;
use ops_core::config::theme_types::LayoutKind;
use ops_core::report::{Report, ReportRow, ReportStatus};

/// The dotted separator width is a function of the trailing slot's *display
/// width* only — not its contents. Two trailings of equal width must produce
/// byte-identical lines, and a wider trailing must eat into the dots. This is
/// what lets a report result string ("None") reuse the runner's duration slot.
#[test]
fn render_slot_separator_depends_only_on_trailing_width() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let slot = |trailing: &'static str| {
        theme.render_slot(
            &SlotLine {
                icon: "\u{2713}",
                label: "Section",
                trailing,
                trailing_prefix: None,
                is_running: false,
            },
            80,
        )
    };

    // Equal-width trailings → identical separator (only the trailing text
    // itself differs).
    assert_eq!(
        slot("None").matches('.').count(),
        slot("abcd").matches('.').count()
    );
    assert_eq!(
        slot("None").replace("None", "abcd"),
        slot("abcd"),
        "lines must be identical apart from the trailing text"
    );

    // A wider trailing consumes separator dots.
    let narrow_dots = slot("None").matches('.').count();
    let wide_dots = slot("1 error, 1 warning").matches('.').count();
    assert!(
        wide_dots < narrow_dots,
        "wider trailing must shorten the dotted separator: {wide_dots} !< {narrow_dots}"
    );
}

/// `render_report` emits: blank, title, blank, one slot line per row (with the
/// row's result in the trailing slot), each row's details verbatim, then the
/// footer. Detail lines are passed through untouched (no dotted separator).
#[test]
fn render_report_emits_title_rows_details_and_footer() {
    let theme = ConfigurableTheme::new(ThemeConfig::compact());
    let mut report = Report::new("Health Report");
    report.push(ReportRow::new(ReportStatus::Ok, "Clean Section", "None"));
    report.push(
        ReportRow::new(ReportStatus::Warning, "Dups", "1 warning")
            .with_details(vec!["      note line".to_string()]),
    );

    let lines = theme.render_report(&report, 80);
    let joined = lines.join("\n");

    assert_eq!(lines[0], "");
    assert!(lines[1].contains("Health Report"));
    assert_eq!(lines[2], "");

    // Ok row: icon + label + result in the slot.
    assert!(joined.contains("\u{2713}"));
    assert!(joined.contains("Clean Section"));
    assert!(joined.contains("None"));

    // Warning row + verbatim detail line. CL-3: the detail text itself is
    // verbatim (no dotted separator, no reflow), but it carries the same left
    // margin as the title and the rows — otherwise it hangs a column left of
    // the row it belongs to.
    assert!(joined.contains("\u{26a0}"));
    assert!(joined.contains("1 warning"));
    let expected_detail = format!("{}      note line", theme.left_pad_str());
    assert!(
        lines.contains(&expected_detail),
        "detail line must be emitted verbatim behind the theme's left pad: {lines:?}"
    );

    // Footer: 2 checks, 1 warning.
    assert!(joined.contains("Done 2 checks \u{b7} 1 warning"));
}

/// Report icons and colors are theme-defined: overriding the `[report]` block
/// changes the glyph in the output. This is the headline behavior — a custom
/// theme restyles `ops deps` the same way it restyles `ops verify`.
#[test]
fn report_icons_come_from_theme_block() {
    let mut cfg = ThemeConfig::compact();
    cfg.report.icon_ok = "PASS".into();
    let theme = ConfigurableTheme::new(cfg);

    let mut report = Report::new("R");
    report.push(ReportRow::new(ReportStatus::Ok, "Section", "None"));
    let joined = theme.render_report(&report, 80).join("\n");

    assert!(
        joined.contains("PASS"),
        "custom report icon must appear in output: {joined}"
    );
    assert!(
        !joined.contains("\u{2713}"),
        "default glyph must be replaced"
    );
}

/// A boxed theme draws a full frame around the report: top border with the
/// title, every content line wrapped to the same width with `│ … │`, and a
/// bottom border carrying the footer. Mirrors the runner's boxed layout.
#[test]
fn boxed_theme_frames_the_report() {
    let theme = ConfigurableTheme::new(ThemeConfig {
        layout_kind: LayoutKind::Boxed,
        left_pad: 0,
        ..ThemeConfig::compact()
    });
    let cols: u16 = 70;
    let mut report = Report::new("Dependency Health Report");
    report.push(ReportRow::new(ReportStatus::Ok, "Advisories", "None"));
    report.push(
        ReportRow::new(ReportStatus::Warning, "Duplicate Crates", "1 warning")
            .with_details(vec!["      (transitive, usually harmless)".to_string()]),
    );

    let lines = theme.render_report(&report, cols);
    // Lines: ["", top, row, row, detail, bottom].
    assert_eq!(lines[0], "");
    let body = &lines[1..];

    let top = strip_ansi(body.first().unwrap());
    assert!(top.starts_with("╭─"), "top corner: {top}");
    assert!(top.ends_with('╮'), "top corner end: {top}");
    assert!(top.contains("Dependency Health Report"));

    let bottom = strip_ansi(body.last().unwrap());
    assert!(bottom.starts_with("╰─"), "bottom corner: {bottom}");
    assert!(bottom.ends_with('╯'), "bottom corner end: {bottom}");
    assert!(bottom.contains("Done 2 checks \u{b7} 1 warning"));

    // Every framed line spans exactly `cols` columns and ends in the right bar,
    // so the frame's right edge is straight.
    for line in body {
        let plain = strip_ansi(line);
        assert_eq!(
            ops_core::output::display_width(&plain),
            usize::from(cols),
            "framed line must span full width: {plain:?}"
        );
    }
    for inner in &body[1..body.len() - 1] {
        assert!(
            strip_ansi(inner).ends_with('│'),
            "interior line must close with a frame bar: {inner:?}"
        );
    }

    // Visual aid under `--nocapture`.
    for line in &lines {
        println!("{line}");
    }
}

/// `render_summary_text` must reproduce the chrome of the legacy
/// `render_summary` for the "Done in …" body — the split that lets reports
/// reuse the runner's footer styling cannot drift.
#[test]
fn render_summary_text_matches_render_summary_done_case() {
    for theme in [
        ConfigurableTheme::new(ThemeConfig::classic()),
        ConfigurableTheme::new(ThemeConfig::compact()),
    ] {
        let via_summary = theme.render_summary(true, 1.5);
        let via_text = theme.render_summary_text("Done in 1.50s");
        assert_eq!(via_summary, via_text);
    }
}

/// READ-6 / TASK-1973 AC#3: a report row label that already carries an SGR
/// sequence must not change the geometry. The layout used to measure the
/// label prefix with the ANSI-blind `display_width` and the assembled line
/// with the ANSI-aware `visible_width`, so the two halves of the same line
/// disagreed by exactly the escape's byte length and the frame bent.
#[test]
fn boxed_report_row_with_sgr_label_matches_border_width() {
    use crate::style::visible_width;
    let theme = ConfigurableTheme::new(ThemeConfig {
        layout_kind: LayoutKind::Boxed,
        left_pad: 0,
        ..ThemeConfig::compact()
    });
    let cols: u16 = 70;

    let mut plain = Report::new("Health");
    plain.push(ReportRow::new(ReportStatus::Ok, "Advisories", "None"));
    let mut styled = Report::new("Health");
    styled.push(ReportRow::new(
        ReportStatus::Ok,
        "\x1b[1;31mAdvisories\x1b[0m",
        "None",
    ));

    let plain_lines = theme.render_report(&plain, cols);
    let styled_lines = theme.render_report(&styled, cols);
    assert_eq!(plain_lines.len(), styled_lines.len());
    for (p, s) in plain_lines.iter().zip(styled_lines.iter()) {
        assert_eq!(
            visible_width(p),
            visible_width(s),
            "an SGR-carrying label must not change the geometry: {p:?} vs {s:?}"
        );
    }
    // And the framed lines are still exactly the border width.
    for line in styled_lines.iter().skip(1) {
        assert_eq!(visible_width(line), usize::from(cols), "{line:?}");
    }
}

/// SEC-21 / TASK-1965 (extended): the report title and each row's label and
/// result are producer-supplied text on the same footing as the detail lines,
/// so a captured tool output carrying ESC or a control byte must not reach the
/// terminal raw. Sanitisation happens *before* `render_slot` measures and
/// truncates, so the layout math sees the same string that is emitted.
#[test]
fn report_title_label_and_result_are_sanitised() {
    for layout in [LayoutKind::Flat, LayoutKind::Boxed] {
        let theme = ConfigurableTheme::new(ThemeConfig {
            layout_kind: layout,
            left_pad: 0,
            ..ThemeConfig::compact()
        });
        let mut report = Report::new("Health\u{1b}[2J");
        report.push(ReportRow::new(
            ReportStatus::Ok,
            "Advisories\u{1b}[31m",
            "None\u{7}",
        ));

        let lines = theme.render_report(&report, 200);
        let joined = lines.join("\n");
        // Assert the exact raw payloads are gone rather than that no ESC
        // appears anywhere: sanitising renders the title escape as the
        // literal text `\x1b[2J`, so an "any ESC" check collapses to "the
        // theme emitted no SGR of its own" and would fail whenever this
        // process renders with colour on — a reason unrelated to sanitising.
        assert!(
            !joined.contains("\u{1b}[2J"),
            "raw ESC from the title must not survive: {joined:?}"
        );
        assert!(
            !joined.contains("\u{1b}[31m"),
            "raw ESC from the label must not survive: {joined:?}"
        );
        assert!(
            !joined.contains('\u{7}'),
            "raw control byte from the result must not survive: {joined:?}"
        );
        assert!(
            joined.contains("\\x1b[2J"),
            "title ESC must be escaped: {joined:?}"
        );
        assert!(
            joined.contains("\\x1b[31m"),
            "label ESC must be escaped: {joined:?}"
        );
        assert!(
            joined.contains("\\x07"),
            "result control byte must be escaped: {joined:?}"
        );
    }
}
