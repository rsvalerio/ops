//! SEC-21 / TASK-1965: the error block and report details are the crate's
//! largest untrusted-input surface — everything else it renders comes from
//! config, these come from arbitrary child processes. Nothing on that path
//! called the project's own `ui::sanitise_line` defence, so a failing command
//! could repaint the operator's terminal (and break this crate's own layout,
//! because `visible_width` scores an escape as zero columns while the
//! terminal acts on it).

use super::*;
use crate::render::render_error_block_gated;
use crate::style::visible_width;
use ops_core::config::theme_types::{ErrorBlockChars, LayoutKind};
use ops_core::report::{Report, ReportRow, ReportStatus};

/// A stderr tail carrying the three classic terminal-hijack payloads.
fn hostile_detail() -> ErrorDetail {
    ErrorDetail::new(
        "exit status: 1\x1b[2J".to_string(),
        vec![
            "\x1b[2Jcleared the screen".to_string(),
            "overwrite\rthe frame".to_string(),
            "\x1b]8;;https://evil.example\x1b\\click me\x1b]8;;\x1b\\".to_string(),
        ],
    )
}

fn chars() -> ErrorBlockChars {
    ErrorBlockChars {
        top: "┌─".into(),
        mid: "│".into(),
        bottom: "└─".into(),
        rail: "│".into(),
        color: String::new(),
    }
}

/// AC#2: no ESC, no CR, no other C0 byte survives into a rendered line.
#[test]
fn error_block_neutralises_control_bytes_from_subprocess_stderr() {
    let lines = render_error_block_gated(&hostile_detail(), 2, &chars(), 0, false);
    assert!(!lines.is_empty());
    for line in &lines {
        assert!(!line.contains('\x1b'), "ESC survived: {line:?}");
        assert!(!line.contains('\r'), "CR survived: {line:?}");
        assert!(
            !line.chars().any(|c| c.is_control() && c != '\t'),
            "control byte survived: {line:?}"
        );
    }
    // The text is still readable — it is escaped, not dropped.
    let joined = lines.join("\n");
    assert!(joined.contains("cleared the screen"), "{joined}");
    assert!(
        joined.contains("\\x1b"),
        "escape rendered literally: {joined}"
    );
}

/// AC#3: in boxed layout the sanitised (and therefore wider) text is clamped,
/// so the closing frame bar still lands at the frame width.
#[test]
fn boxed_error_block_keeps_frame_width_with_hostile_stderr() {
    let theme = ConfigurableTheme::new(ThemeConfig {
        layout_kind: LayoutKind::Boxed,
        left_pad: 0,
        error_block: chars(),
        ..ThemeConfig::compact()
    });
    let columns = 60u16;
    let lines = theme.render_error_detail(&hostile_detail(), columns);
    assert!(!lines.is_empty());
    for line in &lines {
        assert_eq!(
            visible_width(line),
            usize::from(columns),
            "error detail line must match the frame width: {line:?}"
        );
        assert!(line.ends_with(" │"), "closing bar: {line:?}");
        assert!(!line.contains('\x1b'), "ESC survived: {line:?}");
    }
}

/// AC#1: report row details take the same treatment on both layout paths.
#[test]
fn report_details_are_sanitised_on_both_layouts() {
    let mut report = Report::new("deps");
    report.push(
        ReportRow::new(ReportStatus::Error, "audit", "1 issue")
            .with_details(vec!["\x1b[2Jhostile\rdetail".to_string()]),
    );

    let flat = ConfigurableTheme::new(ThemeConfig::compact());
    for line in flat.render_report(&report, 80) {
        assert!(!line.contains('\x1b'), "flat: {line:?}");
        assert!(!line.contains('\r'), "flat: {line:?}");
    }

    let boxed = ConfigurableTheme::new(ThemeConfig {
        layout_kind: LayoutKind::Boxed,
        left_pad: 0,
        ..ThemeConfig::compact()
    });
    for line in boxed.render_report(&report, 80) {
        assert!(!line.contains('\x1b'), "boxed: {line:?}");
        assert!(!line.contains('\r'), "boxed: {line:?}");
    }
}
