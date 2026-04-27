//! ANSI color styling for theme output.
//!
//! Uses a minimal keyword grammar compatible with `indicatif` templates
//! (e.g. `"bold cyan"`, `"yellow.dim"`). Unknown tokens are ignored so
//! misconfigured themes degrade gracefully to plain text.

use std::borrow::Cow;
use std::io::IsTerminal;
use std::sync::OnceLock;

/// Wrap `text` in ANSI SGR codes derived from `spec`, if stderr is a TTY
/// (and `NO_COLOR` is unset) and the spec contains any recognized tokens.
///
/// Honors the [NO_COLOR](https://no-color.org) convention: if `NO_COLOR` is
/// set to any non-empty value, styling is disabled regardless of TTY state.
///
/// The TTY/`NO_COLOR` decision is computed once per process via [`OnceLock`]
/// so we don't issue an `is_terminal()` syscall and read `NO_COLOR` on every
/// rendered step line. Tests should call [`apply_style_gated`] directly.
pub fn apply_style<'a>(text: &'a str, spec: &str) -> Cow<'a, str> {
    apply_style_gated(text, spec, color_enabled())
}

fn color_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::io::stderr().is_terminal() && !no_color_env())
}

/// True if `NO_COLOR` is set to any non-empty value.
fn no_color_env() -> bool {
    std::env::var_os("NO_COLOR")
        .map(|v| !v.is_empty())
        .unwrap_or(false)
}

/// Same as [`apply_style`] but with explicit TTY gating — used in tests.
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

/// Strip ANSI escape sequences from `s`, returning the visible text.
///
/// Handles the families that callers in this crate may encounter:
///
/// - **CSI** (`ESC [ … final`): any final byte in `0x40..=0x7E`, not just `m`.
///   Covers SGR (`m`), cursor moves (`H`/`A`/etc.), and other CSI commands.
/// - **OSC / DCS / SOS / PM / APC** (`ESC ]`/`ESC P`/`ESC X`/`ESC ^`/`ESC _`):
///   string-introducer sequences terminated by `BEL` (0x07) or the two-byte
///   ST `ESC \`. Required so terminal hyperlinks (`ESC ]8;;url ESC \\`) and
///   similar OSC payloads do not contribute to the visible width.
/// - **Two-byte escapes** (`ESC` followed by a single intermediate/final
///   byte): `ESC N`/`ESC O` (single shifts), `ESC ( c`/`ESC ) c` (charset
///   selection — the trailing `c` is consumed below), and bare `ESC <final>`.
///
/// SGR-only callers can still rely on the result being free of ANSI bytes.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\x1b' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            // CSI: ESC [ ... <final byte 0x40..=0x7E>
            Some('[') => {
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if matches!(c, '\x40'..='\x7E') {
                        break;
                    }
                }
            }
            // String-introducer sequences: terminated by BEL or ESC \.
            Some(']' | 'P' | 'X' | '^' | '_') => {
                while let Some(c) = chars.next() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' {
                        // Consume the ST terminator's trailing byte (typically '\\').
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        }
                        break;
                    }
                }
            }
            // Two-byte sequences with an intermediate selector (charset, etc.):
            // consume the following byte so it isn't emitted as visible text.
            Some('(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' | ' ') => {
                chars.next();
            }
            // Bare ESC <byte> (single shifts, simple controls): swallow the byte.
            Some(_) | None => {}
        }
    }
    out
}

fn parse_spec(spec: &str) -> Vec<&'static str> {
    spec.split([' ', '.'])
        .filter(|s| !s.is_empty())
        .filter_map(token_code)
        .collect()
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

    #[test]
    fn bright_colors_map_to_9x() {
        let out = apply_style_gated("z", "bright_white", true);
        assert_eq!(out, "\x1b[97mz\x1b[0m");
    }

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
        // Cursor move: ESC [ 2 J (clear screen). Final byte is 'J', not 'm'.
        let s = "\x1b[2Jhello\x1b[1;2Hworld";
        assert_eq!(strip_ansi(s), "helloworld");
    }
}
