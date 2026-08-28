//! ANSI styling subsystem.
//!
//! ARCH-1 / TASK-0881: split into two cohesive submodules so the
//! read-only ANSI grammar (used for width measurement across crate
//! boundaries) is decoupled from the rendering-private TTY/`NO_COLOR`
//! gating that owns the SGR application path.
//!
//! - [`sgr`]   — SGR token parsing, gated style application,
//!   `precompute_sgr_prefix` / `apply_with_prefix` (rendering crate
//!   internal API).
//! - [`strip`] — ANSI escape stripping, visible-width measurement and
//!   width-bounded truncation (cross-crate read-only API; no TTY/env
//!   coupling).
//!
//! The flat `theme::style::*` re-exports below preserve the previous
//! module-level API so consumers do not need to import the submodules
//! directly.

mod sgr;
mod strip;

pub(crate) use sgr::color_enabled;
pub use sgr::{apply_style, apply_style_gated, apply_with_prefix, precompute_sgr_prefix};
// The pure gate resolver is exercised by the crate's own tests (CL-3 /
// TASK-1976); production code always goes through `color_enabled`.
#[cfg(test)]
pub(crate) use sgr::color_enabled_for;
pub use strip::{strip_ansi, truncate_to_width, visible_width, ELLIPSIS};

#[cfg(test)]
mod tests {
    use super::*;

    /// READ-5/TASK-0355: an OSC-8 hyperlink wraps visible text in
    /// `ESC ] 8 ; ; <url> ESC \\ <text> ESC ] 8 ; ; ESC \\`. `strip_ansi`
    /// must remove both OSC introducers so the visible portion has zero
    /// ANSI bytes left, matching what a width-sensitive caller expects.
    #[test]
    fn strip_ansi_removes_osc8_hyperlink_escapes() {
        let link = "\x1b]8;;https://example.com\x1b\\click\x1b]8;;\x1b\\ next";
        let out = strip_ansi(link);
        assert_eq!(out, "click next");
        assert!(!out.contains('\x1b'));
    }

    #[test]
    fn strip_ansi_removes_osc_terminated_by_bel() {
        let s = "\x1b]0;window-title\x07after";
        assert_eq!(strip_ansi(s), "after");
    }

    #[test]
    fn strip_ansi_handles_csi_with_non_m_final() {
        let s = "\x1b[2Jhello\x1b[1;2Hworld";
        assert_eq!(strip_ansi(s), "helloworld");
    }

    /// PERF-3 / TASK-0746: `visible_width` must produce identical results to
    /// `display_width(&strip_ansi(s))` across the `strip_ansi` corpus — that is
    /// the contract that lets every hot-path call site swap the allocating
    /// pair for the inline scan without a behaviour change.
    /// DUP-1 / TASK-0978: with the ANSI grammar parser deduplicated, a
    /// proptest over a CSI/OSC/two-byte-escape corpus locks in that
    /// `visible_width` and `display_width(&strip_ansi(_))` agree on
    /// inputs that mix escapes with arbitrary visible text.
    #[test]
    fn visible_width_matches_display_width_proptest() {
        use ops_core::output::display_width;
        use proptest::prelude::*;

        // Escape-sequence atoms that exercise each grammar arm.
        let escapes = prop_oneof![
            // CSI with various finals.
            Just("\x1b[m".to_string()),
            Just("\x1b[1;31m".to_string()),
            Just("\x1b[2J".to_string()),
            Just("\x1b[1;2H".to_string()),
            // OSC terminated by BEL or ST.
            Just("\x1b]0;title\x07".to_string()),
            Just("\x1b]8;;https://example.com\x1b\\".to_string()),
            Just("\x1b]8;;\x1b\\".to_string()),
            // DCS / SOS / PM / APC.
            Just("\x1bPpayload\x1b\\".to_string()),
            Just("\x1bX\x07".to_string()),
            // Two-byte escapes (single shift, charset selectors).
            Just("\x1bN".to_string()),
            Just("\x1b(B".to_string()),
            Just("\x1b)0".to_string()),
            Just("\x1b#8".to_string()),
        ];
        // SEC-11 / TASK-1967: raw C0 and C1 code points used to be excluded
        // from this corpus because `UnicodeWidthStr` and `UnicodeWidthChar`
        // disagree on them. They no longer survive `strip_ansi` (which drops
        // every control character except tab) nor `visible_width` (which
        // skips the same set), so the contract now holds over them too and
        // they are part of the corpus rather than a documented wart.
        let controls = prop_oneof![
            Just("\r".to_string()),
            Just("\n".to_string()),
            Just("\u{8}".to_string()),
            Just("\u{7}".to_string()),
            Just("\u{7f}".to_string()),
            Just("\u{85}".to_string()),
            Just("\u{9b}2J".to_string()),
            Just("\u{9d}0;title\u{9c}".to_string()),
            Just("\u{90}payload\u{9c}".to_string()),
        ];
        let visible = "[a-zA-Z0-9 résumé café 🚀ビルド]{0,8}";
        let chunk = prop_oneof![escapes, controls, visible.prop_map(String::from)];
        let strategy = proptest::collection::vec(chunk, 0..12).prop_map(|v| v.concat());

        proptest!(|(s in strategy)| {
            prop_assert_eq!(
                visible_width(&s),
                display_width(&strip_ansi(&s)),
                "visible_width disagrees with display_width(&strip_ansi(_)) for {:?}",
                s
            );
            let stripped = strip_ansi(&s);
            prop_assert!(!stripped.contains('\x1b'));
            // SEC-11 / TASK-1967 AC#2: no C0 byte other than tab, no DEL and
            // no C1 code point survives, for any input.
            prop_assert!(
                !stripped
                    .chars()
                    .any(|c| (c.is_control() && c != '\t') || ('\u{80}'..='\u{9f}').contains(&c)),
                "control byte survived strip_ansi for {:?} -> {:?}",
                s,
                stripped
            );
        });
    }

    /// SEC-11 / TASK-1967 AC#1: the 8-bit C1 introducers are equivalent to
    /// their two-byte `ESC` forms, so their payloads must be consumed rather
    /// than counted as visible text.
    #[test]
    fn c1_introducers_are_consumed_like_their_esc_forms() {
        assert_eq!(strip_ansi("\u{9b}2Jhello"), "hello");
        assert_eq!(strip_ansi("\u{9b}1;31mred\u{9b}0m"), "red");
        assert_eq!(strip_ansi("\u{9d}0;window-title\u{7}after"), "after");
        assert_eq!(strip_ansi("\u{9d}8;;https://example.com\u{9c}link"), "link");
        assert_eq!(strip_ansi("\u{90}payload\u{9c}tail"), "tail");
        assert_eq!(strip_ansi("\u{9e}pm\u{9c}x"), "x");
        assert_eq!(strip_ansi("\u{9f}apc\u{9c}x"), "x");
        assert_eq!(visible_width("\u{9b}2Jhello"), 5);
    }

    /// SEC-11 / TASK-1967 AC#2: bare C0 bytes never survive stripping, so a
    /// caller cannot print a "stripped" string that still moves the cursor.
    #[test]
    fn bare_control_bytes_are_stripped() {
        assert_eq!(strip_ansi("a\rb\nc\u{8}d\u{7f}e"), "abcde");
        // Tab is the one control character that is kept: it is legitimate
        // layout, and `sanitise_line` in ops-core preserves it too.
        assert_eq!(strip_ansi("a\tb"), "a\tb");
        assert_eq!(strip_ansi("\u{85}next"), "next");
        assert_eq!(visible_width("a\rb"), 2);
    }

    /// CL-3 / TASK-1969: the truncation policy — visible width is bounded,
    /// escapes survive, and a cut string is marked with the ellipsis and
    /// reset.
    #[test]
    fn truncate_to_width_bounds_visible_width() {
        assert_eq!(truncate_to_width("hello", 10), "hello");
        assert_eq!(truncate_to_width("hello", 5), "hello");
        assert_eq!(truncate_to_width("hello", 3), "he\u{2026}");
        assert_eq!(truncate_to_width("hello", 1), "\u{2026}");
        assert_eq!(truncate_to_width("hello", 0), "");
        // Wide glyphs never straddle the budget.
        let cjk = truncate_to_width("构建项目", 5);
        assert_eq!(visible_width(&cjk), 5);
        assert_eq!(cjk, "构建\u{2026}");
        // Escapes are preserved and the cut is reset.
        let styled = truncate_to_width("\x1b[31mredtext\x1b[0m", 4);
        assert_eq!(visible_width(&styled), 4);
        assert!(styled.starts_with("\x1b[31m"));
        assert!(styled.ends_with("\x1b[0m"));
        // Control characters are dropped even when the string already fits.
        assert_eq!(truncate_to_width("a\rb", 10), "ab");
    }

    #[test]
    fn visible_width_matches_display_width_of_stripped() {
        use ops_core::output::display_width;
        let cases: &[&str] = &[
            "",
            "plain",
            "plain ascii",
            "\x1b[1;31mred bold\x1b[0m",
            "\x1b[2Jhello\x1b[1;2Hworld",
            "\x1b]8;;https://example.com\x1b\\click\x1b]8;;\x1b\\ next",
            "\x1b]0;window-title\x07after",
            "résumé café",
            "🚀 deploy",
            "ビルド",
            "mix \x1b[33mwarn\x1b[0m and 🚀 emoji",
            "trailing-esc\x1b",
            "\x1bN single-shift two-byte",
            "\x1b(B charset selector",
        ];
        for s in cases {
            assert_eq!(
                visible_width(s),
                display_width(&strip_ansi(s)),
                "visible_width disagrees with display_width(&strip_ansi(_)) for {s:?}"
            );
        }
    }
}
