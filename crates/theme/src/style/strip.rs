//! ANSI escape stripping and visible-width measurement.
//!
//! ARCH-1 / TASK-0881: split out of `style.rs` so this concern (read-only
//! ANSI grammar handling) is reusable without dragging in the rendering
//! crate's TTY/`NO_COLOR` gating logic.
//!
//! Both functions implement the same ANSI grammar:
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
    let mut width = 0usize;
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\x1b' {
            width = width.saturating_add(ch.width().unwrap_or(0));
            continue;
        }
        match chars.next() {
            Some('[') => {
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if matches!(c, '\x40'..='\x7E') {
                        break;
                    }
                }
            }
            Some(']' | 'P' | 'X' | '^' | '_') => {
                while let Some(c) = chars.next() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' {
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        }
                        break;
                    }
                }
            }
            Some('(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' | ' ') => {
                chars.next();
            }
            Some(_) | None => {}
        }
    }
    width
}

pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\x1b' {
            out.push(ch);
            continue;
        }
        match chars.next() {
            Some('[') => {
                while let Some(&c) = chars.peek() {
                    chars.next();
                    if matches!(c, '\x40'..='\x7E') {
                        break;
                    }
                }
            }
            Some(']' | 'P' | 'X' | '^' | '_') => {
                while let Some(c) = chars.next() {
                    if c == '\x07' {
                        break;
                    }
                    if c == '\x1b' {
                        if chars.peek() == Some(&'\\') {
                            chars.next();
                        }
                        break;
                    }
                }
            }
            Some('(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' | ' ') => {
                chars.next();
            }
            Some(_) | None => {}
        }
    }
    out
}
