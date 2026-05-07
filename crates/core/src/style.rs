//! ANSI terminal styling helpers for CLI output.
//!
//! READ-9 / TASK-0950: helpers gate on `stdout().is_terminal() && !NO_COLOR`
//! before emitting SGR escape codes so redirected output (CI logs, pipes,
//! captured test buffers) stays plain text. Mirrors the gating in
//! `theme::style::sgr::color_enabled` so the two color subsystems agree.

use std::io::IsTerminal;
use std::sync::OnceLock;

fn color_enabled() -> bool {
    static IS_TTY: OnceLock<bool> = OnceLock::new();
    *IS_TTY.get_or_init(|| std::io::stdout().is_terminal()) && !no_color_env()
}

fn no_color_env() -> bool {
    std::env::var_os("NO_COLOR")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

macro_rules! ansi_style {
    ($(#[$meta:meta])* $name:ident, $gated_name:ident, $code:expr) => {
        $(#[$meta])*
        pub fn $name(s: &str) -> String {
            style_gated(s, $code, color_enabled())
        }

        /// Same as [`Self::$name`] but with an explicit color-enabled
        /// override — used by callers that compute their own TTY state
        /// (e.g. against an injected writer) and tests that need to
        /// observe the styled-branch output regardless of process stdout.
        pub fn $gated_name(s: &str, enabled: bool) -> String {
            style_gated(s, $code, enabled)
        }
    };
}

fn style_gated(s: &str, code: u8, enabled: bool) -> String {
    if enabled {
        format!("\x1b[{code}m{s}\x1b[0m")
    } else {
        s.to_string()
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
        assert_eq!(style_gated("hi", 36, false), "hi");
    }

    #[test]
    fn gated_enabled_emits_sgr() {
        assert_eq!(style_gated("hi", 36, true), "\x1b[36mhi\x1b[0m");
    }
}
