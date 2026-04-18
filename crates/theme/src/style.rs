//! ANSI color styling for theme output.
//!
//! Uses a minimal keyword grammar compatible with `indicatif` templates
//! (e.g. `"bold cyan"`, `"yellow.dim"`). Unknown tokens are ignored so
//! misconfigured themes degrade gracefully to plain text.

use std::borrow::Cow;
use std::io::IsTerminal;

/// Wrap `text` in ANSI SGR codes derived from `spec`, if stderr is a TTY
/// and the spec contains any recognized tokens.
pub fn apply_style<'a>(text: &'a str, spec: &str) -> Cow<'a, str> {
    apply_style_gated(text, spec, std::io::stderr().is_terminal())
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

/// Strip ANSI SGR escape sequences (`ESC [ … m`) from `s`.
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next();
            while let Some(&c) = chars.peek() {
                chars.next();
                if c == 'm' {
                    break;
                }
            }
        } else {
            out.push(ch);
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
}
