//! ANSI styling subsystem.
//!
//! Two cohesive submodules, so the read-only ANSI grammar (used for width
//! measurement across crate boundaries) is decoupled from the
//! rendering-private TTY/`NO_COLOR` gating that owns the SGR application
//! path.
//!
//! - [`sgr`]   — SGR token parsing, gated style application,
//!   `precompute_sgr_prefix` / `apply_with_prefix_gated`.
//! - [`strip`] — ANSI escape stripping, visible-width measurement and
//!   width-bounded truncation (cross-crate read-only API; no TTY/env
//!   coupling).
//!
//! The flat `ops_theme::style::*` re-exports below are the module's API and
//! the only public path to these items: the crate root does not re-export
//! them, so each helper is named one way across the workspace.

mod sgr;
mod strip;

pub(crate) use sgr::color_enabled;
pub use sgr::{apply_style, apply_style_gated, apply_with_prefix_gated, precompute_sgr_prefix};
pub use strip::{
    strip_ansi, strip_ansi_preserving_raw, truncate_to_width, visible_width, ELLIPSIS,
    TAB_REPLACEMENT,
};

#[cfg(test)]
mod tests {
    use super::*;

    /// An OSC-8 hyperlink wraps visible text in
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

    /// `visible_width` must produce identical results to
    /// `display_width(&strip_ansi(s))` across the `strip_ansi` corpus — that is
    /// the contract that lets every hot-path call site swap the allocating
    /// pair for the inline scan without a behaviour change.
    /// The proptest over a CSI/OSC/two-byte-escape corpus locks in that
    /// `visible_width` and `display_width(&strip_ansi(_))` agree on
    /// inputs that mix escapes with arbitrary visible text.
    /// Inputs mixing every grammar arm (complete escapes, unterminated
    /// escapes, bare controls, tab) with arbitrary visible text.
    fn ansi_corpus() -> impl proptest::strategy::Strategy<Value = String> {
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
        // Escapes that never terminate, or that a following character
        // proves malformed: each yields a Raw piece.
        let unterminated = prop_oneof![
            Just("\x1b".to_string()),
            Just("\x1b[".to_string()),
            Just("\x1b[12345678".to_string()),
            Just("\x1b[1;".to_string()),
            Just("\x1b(".to_string()),
            Just("\x1b(#".to_string()),
            Just("\x1b]unterminated".to_string()),
            Just("\u{9b}12".to_string()),
        ];
        // Raw C0 and C1 code points are part of the corpus even though
        // `UnicodeWidthStr` and `UnicodeWidthChar` disagree on them: none
        // survives `strip_ansi` (which drops every control character,
        // rewriting tab to a space) nor `visible_width` (which scores the
        // same way), so the contract holds over them too — tab included.
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
            Just("\t".to_string()),
        ];
        let visible = "[a-zA-Z0-9 résumé café 🚀ビルド\\[(#;]{0,8}";
        let chunk = prop_oneof![escapes, unterminated, controls, visible.prop_map(String::from)];
        proptest::collection::vec(chunk, 0..12).prop_map(|v| v.concat())
    }

    /// `visible_width` must produce identical results to
    /// `display_width(&strip_ansi(s))` across the `strip_ansi` corpus — that is
    /// the contract that lets every hot-path call site swap the allocating
    /// pair for the inline scan without a behaviour change.
    #[test]
    fn visible_width_matches_display_width_proptest() {
        use ops_core::output::display_width;
        use proptest::prelude::*;

        let strategy = ansi_corpus();

        proptest!(|(s in strategy)| {
            prop_assert_eq!(
                visible_width(&s),
                display_width(&strip_ansi(&s)),
                "visible_width disagrees with display_width(&strip_ansi(_)) for {:?}",
                s
            );
            let stripped = strip_ansi(&s);
            prop_assert!(!stripped.contains('\x1b'));
            // No C0 byte, no
            // DEL and no C1 code point survives, for any input — tab
            // included, since it is rewritten to a space.
            prop_assert!(
                !stripped
                    .chars()
                    .any(|c| c.is_control() || ('\u{80}'..='\u{9f}').contains(&c)),
                "control byte survived strip_ansi for {:?} -> {:?}",
                s,
                stripped
            );
        });
    }

    /// The 8-bit C1 introducers are equivalent to
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

    /// Bare C0 bytes never survive stripping, so a
    /// caller cannot print a "stripped" string that still moves the cursor.
    #[test]
    fn bare_control_bytes_are_stripped() {
        assert_eq!(strip_ansi("a\rb\nc\u{8}d\u{7f}e"), "abcde");
        assert_eq!(strip_ansi("\u{85}next"), "next");
        assert_eq!(visible_width("a\rb"), 2);
    }

    /// Tab is rewritten to one space by every helper that shares the
    /// grammar, so measurement and painting agree. A tab that survived
    /// stripping while measuring as zero columns would paint a tabbed line
    /// wider than it measured and push the boxed frame's closing bar out of
    /// column.
    #[test]
    fn tab_is_rewritten_to_one_space_in_every_helper() {
        assert_eq!(strip_ansi("a\tb"), "a b");
        assert_eq!(visible_width("a\tb"), 3);
        assert_eq!(truncate_to_width("a\tb", 10), "a b");
        assert_eq!(truncate_to_width("a\tb", 2), "a\u{2026}");
        assert!(!strip_ansi("\tstart").contains('\t'));
        assert_eq!(TAB_REPLACEMENT, ' ');
    }

    /// The truncation policy — visible width is bounded,
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

    /// No byte of an unterminated escape reaches the output: `visible_width`
    /// scores it zero columns, so a surviving byte would be visible text the
    /// budget never paid for. A control character next to the stray
    /// introducer is not part of the run and is dropped by the Char policy.
    #[test]
    fn truncate_to_width_drops_unterminated_escapes() {
        assert_eq!(truncate_to_width("abc\x1b[12345678", 3), "abc");
        assert_eq!(truncate_to_width("ab\x1b(\u{e9}xyz", 3), "ab\u{2026}");
        assert_eq!(truncate_to_width("\x1b(\rtext", 10), "text");
        assert_eq!(truncate_to_width("\x1b(\rtext", 3), "te\u{2026}");
        for (s, n) in [("abc\x1b[12345678", 3), ("ab\x1b(\u{e9}xyz", 3)] {
            let out = truncate_to_width(s, n);
            assert!(visible_width(&out) <= n, "{s:?} at {n} cols -> {out:?}");
            assert!(!out.contains('['), "unterminated CSI bytes survived: {out:?}");
        }
        for cols in [0, 1, 3, 10] {
            let out = truncate_to_width("\x1b(\rte\nxt", cols);
            assert!(
                !out.chars().any(|c| matches!(c, '\r' | '\n')),
                "control survived at {cols} cols: {out:?}"
            );
        }
    }

    /// The truncation contract, over the full grammar corpus: the result
    /// never measures wider than the budget it was given.
    #[test]
    fn truncate_to_width_bound_proptest() {
        use proptest::prelude::*;

        proptest!(|(s in ansi_corpus(), n in 0usize..40)| {
            let out = truncate_to_width(&s, n);
            prop_assert!(
                visible_width(&out) <= n,
                "visible_width(truncate_to_width({:?}, {})) = {} for {:?}",
                s,
                n,
                visible_width(&out),
                out
            );
        });
    }

    /// A stray `ESC [` before non-ASCII text ends at the first character
    /// outside the CSI parameter/intermediate range instead of swallowing
    /// everything up to the next ASCII letter.
    #[test]
    fn stray_csi_before_non_ascii_does_not_swallow_text() {
        let s = "\x1b[日本語abc";
        assert_eq!(strip_ansi(s), "日本語abc");
        assert_eq!(visible_width(s), 9);
        assert_eq!(strip_ansi_preserving_raw(s), s);
        // A non-ASCII character after parameter bytes ends it the same way.
        assert_eq!(strip_ansi("\x1b[1;\u{2014}abc"), "\u{2014}abc");
    }

    /// The character that proves an nF escape malformed is re-emitted, not
    /// folded into the raw run.
    #[test]
    fn stray_nf_before_non_ascii_keeps_the_character() {
        let s = "ab\x1b(\u{e9}xyz";
        assert_eq!(strip_ansi(s), "ab\u{e9}xyz");
        assert_eq!(visible_width(s), 6);
        assert_eq!(strip_ansi_preserving_raw(s), s);
    }

    /// `strip_ansi_preserving_raw` removes complete sequences only; the
    /// bytes of truncated or stray escapes, control characters and tab
    /// survive verbatim for the caller's own validator.
    #[test]
    fn strip_ansi_preserving_raw_removes_only_complete_sequences() {
        // Complete CSI, OSC (BEL- and ST-terminated) and 8-bit C1 forms.
        assert_eq!(strip_ansi_preserving_raw("\x1b[1;31mred\x1b[0m"), "red");
        assert_eq!(strip_ansi_preserving_raw("\x1b]0;title\x07after"), "after");
        assert_eq!(
            strip_ansi_preserving_raw("\x1b]8;;https://example.com\x1b\\link\x1b]8;;\x1b\\"),
            "link"
        );
        assert_eq!(strip_ansi_preserving_raw("\u{9b}2Jhello"), "hello");
        assert_eq!(strip_ansi_preserving_raw("\u{9d}0;t\u{9c}x"), "x");
        // Truncated and stray escapes are kept verbatim.
        assert_eq!(strip_ansi_preserving_raw("abc\x1b[12"), "abc\x1b[12");
        assert_eq!(strip_ansi_preserving_raw("trailing\x1b"), "trailing\x1b");
        assert_eq!(strip_ansi_preserving_raw("\x1b]8;;no-terminator"), "\x1b]8;;no-terminator");
        // Control characters and tab are kept, unlike `strip_ansi`.
        assert_eq!(strip_ansi_preserving_raw("\x1b[1ma\rb\tc\u{85}d"), "a\rb\tc\u{85}d");
    }

    /// Input with no escape introducer is returned borrowed, even when it
    /// carries control characters or tab.
    #[test]
    fn strip_ansi_preserving_raw_borrows_without_an_introducer() {
        let s = "plain\ttext\r";
        let out = strip_ansi_preserving_raw(s);
        assert!(matches!(out, std::borrow::Cow::Borrowed(_)), "{out:?}");
        assert_eq!(out, s);
        assert!(matches!(strip_ansi_preserving_raw("x\x1b[1my"), std::borrow::Cow::Owned(_)));
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
