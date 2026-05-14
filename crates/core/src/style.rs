//! ANSI terminal styling helpers for CLI output.
//!
//! READ-9 / TASK-0950: helpers gate on `stdout().is_terminal() && !NO_COLOR`
//! before emitting SGR escape codes so redirected output (CI logs, pipes,
//! captured test buffers) stays plain text. Mirrors the gating in
//! `theme::style::sgr::color_enabled` so the two color subsystems agree.

use std::borrow::Cow;
use std::io::IsTerminal;
use std::sync::OnceLock;

/// DUP-3 / TASK-1188: shared color-enablement resolver. Both
/// `core::style::cyan` (stdout-bound) and `theme::style::sgr::apply_style`
/// (stderr-bound) used to compute their own `OnceLock<bool>` cache against
/// different streams, so a terminal where stdout is a TTY but stderr is
/// piped (or vice versa) silently disagreed on whether to emit SGR codes.
/// The shared resolver caches `is_terminal()` for **both** streams once per
/// process and enables color when **either** is a TTY (and `NO_COLOR` is
/// unset). Either stream being a real terminal means there is a human
/// reader who benefits from styling; emitting SGR into the other stream is
/// the same risk the per-stream-only gate already accepted on the styled
/// branch.
#[must_use]
pub fn color_enabled() -> bool {
    static STDOUT_TTY: OnceLock<bool> = OnceLock::new();
    static STDERR_TTY: OnceLock<bool> = OnceLock::new();
    let stdout = *STDOUT_TTY.get_or_init(|| std::io::stdout().is_terminal());
    let stderr = *STDERR_TTY.get_or_init(|| std::io::stderr().is_terminal());
    (stdout || stderr) && !no_color_env()
}

/// True if `NO_COLOR` is set to any non-empty value (per
/// <https://no-color.org/>). Public so the theme crate's SGR application
/// can route through the same env probe.
#[must_use]
pub fn no_color_env() -> bool {
    std::env::var_os("NO_COLOR")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

macro_rules! ansi_style {
    ($(#[$meta:meta])* $name:ident, $gated_name:ident, $code:expr) => {
        $(#[$meta])*
        pub fn $name(s: &str) -> Cow<'_, str> {
            style_gated(s, $code, color_enabled())
        }

        /// Same as [`$name`] but with an explicit color-enabled override —
        /// used by callers that compute their own TTY state (e.g. against
        /// an injected writer) and tests that need to observe the styled-
        /// branch output regardless of process stdout.
        pub fn $gated_name(s: &str, enabled: bool) -> Cow<'_, str> {
            style_gated(s, $code, enabled)
        }
    };
}

// PERF-5 / TASK-1397: color-disabled output is the dominant CI / piped case;
// returning `Cow::Borrowed` then skips the per-call heap allocation that
// `s.to_string()` previously forced on every `cyan`/`grey`/`dim` invocation.
fn style_gated(s: &str, code: u8, enabled: bool) -> Cow<'_, str> {
    if enabled {
        Cow::Owned(format!("\x1b[{code}m{s}\x1b[0m"))
    } else {
        Cow::Borrowed(s)
    }
}

ansi_style!(cyan, cyan_gated, 36);
ansi_style!(white, white_gated, 37);
ansi_style!(grey, grey_gated, 90);
ansi_style!(dim, dim_gated, 2);
ansi_style!(green, green_gated, 32);
ansi_style!(red, red_gated, 31);
ansi_style!(yellow, yellow_gated, 33);
ansi_style!(bold, bold_gated, 1);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gated_disabled_returns_plain() {
        let out = style_gated("hi", 36, false);
        assert_eq!(out, "hi");
        assert!(matches!(out, Cow::Borrowed(_)));
    }

    #[test]
    fn gated_enabled_emits_sgr() {
        assert_eq!(style_gated("hi", 36, true), "\x1b[36mhi\x1b[0m");
    }
}
