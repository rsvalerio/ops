//! SGR token parsing and gated style application.
//!
//! ARCH-1 / TASK-0881: split out of `style.rs` so the rendering-private
//! TTY/`NO_COLOR` gating lives next to the SGR application code and stays
//! separable from the read-only ANSI stripping concerns in
//! [`super::strip`].
//!
//! Uses a minimal keyword grammar compatible with `indicatif` templates
//! (e.g. `"bold cyan"`, `"yellow.dim"`). Unknown tokens are ignored so
//! misconfigured themes degrade gracefully to plain text.

use std::borrow::Cow;

/// Wrap `text` in ANSI SGR codes derived from `spec`, if **stderr** is a TTY
/// (and `NO_COLOR` is unset) and the spec contains any recognized tokens.
///
/// Everything this crate renders goes to stderr — the runner writes step
/// lines and error blocks through `write_stderr` and indicatif's
/// stderr-backed `MultiProgress` — so stdout's TTY state is deliberately not
/// consulted. See [`color_enabled`].
///
/// Honors the [NO_COLOR](https://no-color.org) convention: if `NO_COLOR` is
/// set to any non-empty value, styling is disabled regardless of TTY state.
///
/// The `is_terminal()` syscall is cached once per process via `OnceLock`;
/// `NO_COLOR` is re-read on each call so runtime overrides (notably tests
/// using `EnvGuard`) take effect after the TTY check has been cached.
/// Tests should call [`apply_style_gated`] directly.
#[must_use]
pub fn apply_style<'a>(text: &'a str, spec: &str) -> Cow<'a, str> {
    apply_style_gated(text, spec, color_enabled())
}

/// Whether this crate may emit SGR codes.
///
/// CL-3 / TASK-1976: gated on the stream this crate actually renders to.
/// DUP-3 / TASK-1188 routed the theme onto `ops_core::style::color_enabled`,
/// which is `(stdout_is_terminal() || stderr_is_terminal()) && !NO_COLOR` —
/// an OR chosen for the shared, stdout-bound table renderer. For a
/// stderr-only renderer that OR is wrong in both directions:
/// `ops verify 2> build.log` from an interactive shell leaves stdout a TTY,
/// so the log file fills with escape codes no terminal will render, while
/// `ops verify > out.txt` is coloured only by accident of the OR.
///
/// The shared per-stream probes are still used, so both subsystems keep one
/// cached `is_terminal()` answer per stream and one `NO_COLOR` reading; only
/// the stream each consults differs, which is the distinction TASK-1188's
/// single resolver had erased.
#[must_use]
pub fn color_enabled() -> bool {
    color_enabled_for(
        ops_core::style::stderr_is_terminal(),
        ops_core::style::no_color_env(),
    )
}

/// Pure resolver behind [`color_enabled`], split out so the gate is testable
/// without a real terminal on either stream.
#[must_use]
pub const fn color_enabled_for(stderr_tty: bool, no_color: bool) -> bool {
    stderr_tty && !no_color
}

/// Same as [`apply_style`] but with explicit TTY gating — used in tests.
#[must_use]
pub fn apply_style_gated<'a>(text: &'a str, spec: &str, enabled: bool) -> Cow<'a, str> {
    if !enabled || spec.is_empty() {
        return Cow::Borrowed(text);
    }
    let codes = parse_spec(spec);
    if codes.is_empty() {
        return Cow::Borrowed(text);
    }
    Cow::Owned(format!("\x1b[{}m{}\x1b[0m", codes.join(";"), text))
}

fn parse_spec(spec: &str) -> Vec<&'static str> {
    spec.split([' ', '.'])
        .filter(|s| !s.is_empty())
        .filter_map(token_code)
        .collect()
}

/// Precompute the SGR prefix for `spec`.
///
/// For example `"\x1b[1;32m"` for `"bold green"`. Returns `None` when the
/// spec contains no recognized tokens. TASK-0747: callers store this once
/// at construction and reuse per render.
#[must_use]
pub fn precompute_sgr_prefix(spec: &str) -> Option<String> {
    let codes = parse_spec(spec);
    if codes.is_empty() {
        None
    } else {
        Some(format!("\x1b[{}m", codes.join(";")))
    }
}

/// Apply a precomputed SGR prefix to `text`.
///
/// Returns `Cow::Borrowed` when `prefix` is `None` or color is disabled —
/// i.e. stderr is not a TTY, or `NO_COLOR` is set (CL-3 / TASK-1976; see
/// [`color_enabled`]). TASK-0747: paired with [`precompute_sgr_prefix`].
///
/// API-2 / TASK-0893: takes `Option<&str>` rather than `&Option<String>`
/// so callers aren't locked into `String` storage and can pass borrowed
/// slices, `Cow`s, or accessor returns via `.as_deref()`.
#[must_use]
pub fn apply_with_prefix<'a>(text: &'a str, prefix: Option<&str>) -> Cow<'a, str> {
    if !color_enabled() {
        return Cow::Borrowed(text);
    }
    prefix.map_or(Cow::Borrowed(text), |pfx| {
        Cow::Owned(format!("{pfx}{text}\x1b[0m"))
    })
}

fn token_code(token: &str) -> Option<&'static str> {
    Some(match token {
        "bold" => "1",
        "dim" => "2",
        "italic" => "3",
        "underline" => "4",
        "black" => "30",
        "red" => "31",
        "green" => "32",
        "yellow" => "33",
        "blue" => "34",
        "magenta" => "35",
        "cyan" => "36",
        "white" => "37",
        "bright_black" => "90",
        "bright_red" => "91",
        "bright_green" => "92",
        "bright_yellow" => "93",
        "bright_blue" => "94",
        "bright_magenta" => "95",
        "bright_cyan" => "96",
        "bright_white" => "97",
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_spec_returns_borrowed_unchanged() {
        let out = apply_style_gated("hello", "", true);
        assert_eq!(out, "hello");
        assert!(matches!(out, Cow::Borrowed(_)));
    }

    #[test]
    fn disabled_tty_returns_borrowed_unchanged() {
        let out = apply_style_gated("hello", "bold green", false);
        assert_eq!(out, "hello");
        assert!(matches!(out, Cow::Borrowed(_)));
    }

    #[test]
    fn bold_green_emits_sgr() {
        let out = apply_style_gated("Done", "bold green", true);
        assert_eq!(out, "\x1b[1;32mDone\x1b[0m");
    }

    #[test]
    fn dot_separated_tokens_parse() {
        let out = apply_style_gated("x", "yellow.dim", true);
        assert_eq!(out, "\x1b[33;2mx\x1b[0m");
    }

    #[test]
    fn unknown_tokens_ignored() {
        let out = apply_style_gated("y", "not-a-color bold", true);
        assert_eq!(out, "\x1b[1my\x1b[0m");
    }

    #[test]
    fn only_unknown_tokens_returns_plain() {
        let out = apply_style_gated("y", "nope zzz", true);
        assert_eq!(out, "y");
        assert!(matches!(out, Cow::Borrowed(_)));
    }

    /// CL-3 / TASK-1976: the theme renders to stderr only, so the gate must
    /// ignore stdout. The scenario that motivated the finding —
    /// `ops verify 2> build.log` from an interactive shell, i.e. stdout a
    /// TTY and stderr redirected — must resolve to "no colour" here even
    /// though the stdout-bound resolver in `ops-core` says otherwise.
    #[test]
    fn gate_ignores_stdout_and_follows_stderr() {
        // stderr redirected, stdout a TTY: theme is off, core's OR is on.
        assert!(!color_enabled_for(false, false));
        assert!(ops_core::style::color_enabled_from(true, false, false));
        // stderr a TTY: on, unless NO_COLOR.
        assert!(color_enabled_for(true, false));
        assert!(!color_enabled_for(true, true));
        assert!(!color_enabled_for(false, true));
    }

    /// DUP-3 / TASK-1188: both color subsystems must agree on the *styled
    /// output* for the same `enabled` boolean (they resolve that boolean
    /// against different streams — see [`color_enabled`] and CL-3 /
    /// TASK-1976). Pinning equivalence directly via `apply_style_gated` and
    /// `cyan_gated` (the explicit-override variants) ensures neither
    /// subsystem silently emits codes the other skips.
    #[test]
    fn core_and_theme_color_subsystems_agree_on_enablement() {
        // When disabled, both must return plain text.
        let theme_off = apply_style_gated("hi", "cyan", false);
        let core_off = ops_core::style::cyan_gated("hi", false);
        assert_eq!(theme_off, "hi");
        assert_eq!(core_off, "hi");
        // When enabled, both must emit cyan SGR around the same text.
        let theme_on = apply_style_gated("hi", "cyan", true);
        let core_on = ops_core::style::cyan_gated("hi", true);
        assert_eq!(theme_on, "\x1b[36mhi\x1b[0m");
        assert_eq!(core_on, "\x1b[36mhi\x1b[0m");
    }

    #[test]
    fn bright_colors_map_to_9x() {
        let out = apply_style_gated("z", "bright_white", true);
        assert_eq!(out, "\x1b[97mz\x1b[0m");
    }
}
