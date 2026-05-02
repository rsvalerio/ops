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
//! - [`strip`] — ANSI escape stripping and visible-width measurement
//!   (cross-crate read-only API; no TTY/env coupling).
//!
//! The flat `theme::style::*` re-exports below preserve the previous
//! module-level API so consumers do not need to import the submodules
//! directly.

mod sgr;
mod strip;

pub use sgr::{apply_style, apply_style_gated, apply_with_prefix, precompute_sgr_prefix};
pub use strip::{strip_ansi, visible_width};

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
    /// `display_width(&strip_ansi(s))` across the strip_ansi corpus — that is
    /// the contract that lets every hot-path call site swap the allocating
    /// pair for the inline scan without a behaviour change.
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
