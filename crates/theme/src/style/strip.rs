//! ANSI escape stripping and visible-width measurement.
//!
//! ARCH-1 / TASK-0881: split out of `style.rs` so this concern (read-only
//! ANSI grammar handling) is reusable without dragging in the rendering
//! crate's TTY/`NO_COLOR` gating logic.
//!
//! DUP-1 / TASK-0978: the ANSI grammar lives in a single iterator
//! [`ansi_visible_chars`]; both [`visible_width`] and [`strip_ansi`]
//! consume it so any future grammar fix lands in one place.
//!
//! The grammar covers:
//!
//! - **CSI** (`ESC [ … final`): any final byte in `0x40..=0x7E`, not just `m`.
//!   Covers SGR (`m`), cursor moves (`H`/`A`/etc.), and other CSI commands.
//! - **OSC / DCS / SOS / PM / APC** (`ESC ]`/`ESC P`/`ESC X`/`ESC ^`/`ESC _`):
//!   string-introducer sequences terminated by `BEL` (0x07) or the two-byte
//!   ST `ESC \`. Required so terminal hyperlinks (`ESC ]8;;url ESC \\`) and
//!   similar OSC payloads do not contribute to the visible width.
//! - **Two-byte escapes** (`ESC` followed by a single intermediate/final
//!   byte): `ESC N`/`ESC O` (single shifts), `ESC ( c`/`ESC ) c` (charset
//!   selection — the trailing `c` is consumed below), and bare `ESC <final>`.

use std::str::Chars;

/// Iterator that yields the visible (non-escape) characters of `s` after
/// consuming ANSI CSI/OSC/two-byte-escape sequences. Encapsulates the entire
/// ANSI grammar so width measurement and string stripping share one parser.
struct AnsiVisibleChars<'a> {
    chars: Chars<'a>,
}

impl<'a> Iterator for AnsiVisibleChars<'a> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        loop {
            let ch = self.chars.next()?;
            if ch != '\x1b' {
                return Some(ch);
            }
            match self.chars.next() {
                Some('[') => self.consume_csi(),
                Some(']' | 'P' | 'X' | '^' | '_') => self.consume_string_terminated(),
                Some('(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' | ' ') => {
                    // Two-byte escape with one intermediate, then a final char.
                    self.chars.next();
                }
                Some(_) | None => {}
            }
        }
    }
}

impl<'a> AnsiVisibleChars<'a> {
    fn consume_csi(&mut self) {
        for c in self.chars.by_ref() {
            if matches!(c, '\x40'..='\x7E') {
                break;
            }
        }
    }

    fn consume_string_terminated(&mut self) {
        while let Some(c) = self.chars.next() {
            if c == '\x07' {
                break;
            }
            if c == '\x1b' {
                if self.chars.clone().next() == Some('\\') {
                    self.chars.next();
                }
                break;
            }
        }
    }
}

fn ansi_visible_chars(s: &str) -> AnsiVisibleChars<'_> {
    AnsiVisibleChars { chars: s.chars() }
}

/// Visible terminal width of `s` after stripping ANSI escapes, computed
/// without allocating an intermediate `String`.
///
/// PERF-3 / TASK-0746: equivalent to `display_width(&strip_ansi(s))` but
/// scans the same ANSI grammar inline and accumulates per-character widths
/// (`UnicodeWidthChar`). The boxed-layout step renderer calls this per row,
/// so removing the intermediate `String` allocation pays off on every step
/// of every run. Hot-path callers should prefer this over the
/// `display_width(&strip_ansi(...))` pair.
#[must_use]
pub fn visible_width(s: &str) -> usize {
    use unicode_width::UnicodeWidthChar;
    ansi_visible_chars(s)
        .map(|c| c.width().unwrap_or(0))
        .fold(0usize, |acc, w| acc.saturating_add(w))
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    out.extend(ansi_visible_chars(s));
    out
}
