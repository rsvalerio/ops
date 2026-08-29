//! ANSI escape stripping, visible-width measurement and width-bounded
//! truncation.
//!
//! ARCH-1 / TASK-0881: split out of `style.rs` so this concern (read-only
//! ANSI grammar handling) is reusable without dragging in the rendering
//! crate's TTY/`NO_COLOR` gating logic.
//!
//! DUP-1 / TASK-0978: the ANSI grammar lives in a single iterator
//! [`AnsiPieces`]; [`visible_width`], [`strip_ansi`] and
//! [`truncate_to_width`] all consume it so any future grammar fix lands in
//! one place.
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
//! - **8-bit C1 introducers** (SEC-11 / TASK-1967): `U+009B` (CSI),
//!   `U+009D` (OSC), `U+0090` (DCS), `U+0098` (SOS), `U+009E` (PM) and
//!   `U+009F` (APC) are the single-code-point equivalents of the two-byte
//!   `ESC` forms above; a terminal in 8-bit mode acts on them identically.
//!   They are consumed with the same payload rules, and `U+009C` (ST)
//!   terminates a string sequence just like `ESC \`.
//! - **Bare control characters** (SEC-11 / TASK-1967): every remaining C0
//!   code point except tab (`\r`, `\n`, `\x08`, `\x07`, …), `DEL` (`\x7f`)
//!   and every non-introducer C1 code point is dropped. They measure as zero
//!   columns but the terminal *acts* on them — a bare `\r` returns to column
//!   0 and overwrites a box frame — so a "stripped" string must not still
//!   carry them.

use std::borrow::Cow;
use std::str::Chars;

use unicode_width::UnicodeWidthChar;

/// One unit of the ANSI grammar: either an escape sequence (zero visible
/// columns, but meaningful to the terminal) or a single visible character.
///
/// Control characters that are neither are dropped by the iterator — see the
/// module docs.
enum AnsiPiece<'a> {
    /// A complete escape sequence, borrowed from the source string.
    Escape(&'a str),
    /// A visible character.
    Visible(char),
}

/// Iterator over the [`AnsiPiece`]s of a string. Encapsulates the entire
/// ANSI grammar so width measurement, stripping and truncation share one
/// parser.
struct AnsiPieces<'a> {
    chars: Chars<'a>,
}

/// True for control characters that carry no visible width and must never
/// survive into rendered output: C0 except tab, `DEL`, and the C1 block.
///
/// The C1 *introducers* are matched by the parser before this predicate is
/// reached; this catches the remainder (e.g. `U+0085` NEL, `U+0084` IND).
const fn is_droppable_control(c: char) -> bool {
    matches!(c, '\u{0}'..='\u{8}' | '\u{a}'..='\u{1f}' | '\u{7f}'..='\u{9f}')
}

impl<'a> Iterator for AnsiPieces<'a> {
    type Item = AnsiPiece<'a>;

    fn next(&mut self) -> Option<AnsiPiece<'a>> {
        loop {
            let rest = self.chars.as_str();
            let ch = self.chars.next()?;
            match ch {
                '\x1b' => {
                    match self.chars.next() {
                        Some('[') => self.consume_csi(),
                        Some(']' | 'P' | 'X' | '^' | '_') => self.consume_string_terminated(),
                        Some('(' | ')' | '*' | '+' | '-' | '.' | '/' | '#' | ' ') => {
                            // Two-byte escape with one intermediate, then a final char.
                            self.chars.next();
                        }
                        Some(_) | None => {}
                    }
                    return Some(AnsiPiece::Escape(self.consumed_from(rest)));
                }
                // 8-bit C1 introducers: same payload rules as their `ESC`
                // two-byte equivalents.
                '\u{9b}' => {
                    self.consume_csi();
                    return Some(AnsiPiece::Escape(self.consumed_from(rest)));
                }
                '\u{90}' | '\u{98}' | '\u{9d}' | '\u{9e}' | '\u{9f}' => {
                    self.consume_string_terminated();
                    return Some(AnsiPiece::Escape(self.consumed_from(rest)));
                }
                c if is_droppable_control(c) => {}
                c => return Some(AnsiPiece::Visible(c)),
            }
        }
    }
}

impl<'a> AnsiPieces<'a> {
    /// The prefix of `rest` consumed since `rest` was captured.
    fn consumed_from(&self, rest: &'a str) -> &'a str {
        let taken = rest.len().saturating_sub(self.chars.as_str().len());
        rest.get(..taken).unwrap_or(rest)
    }

    fn consume_csi(&mut self) {
        for c in self.chars.by_ref() {
            if matches!(c, '\x40'..='\x7E') {
                break;
            }
        }
    }

    fn consume_string_terminated(&mut self) {
        while let Some(c) = self.chars.next() {
            // BEL and the 8-bit ST each terminate in one code point.
            if c == '\x07' || c == '\u{9c}' {
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

fn ansi_pieces(s: &str) -> AnsiPieces<'_> {
    AnsiPieces { chars: s.chars() }
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
    ansi_pieces(s)
        .filter_map(|p| match p {
            AnsiPiece::Visible(c) => Some(c.width().unwrap_or(0)),
            AnsiPiece::Escape(_) => None,
        })
        .fold(0usize, usize::saturating_add)
}

/// Remove every ANSI escape sequence and every non-tab control character
/// from `s`.
///
/// SEC-11 / TASK-1967: the result is guaranteed to contain no C0 code point
/// other than tab, no `DEL`, and no C1 code point — so callers may treat it
/// as safe to print *and* safe to measure with a width helper.
#[must_use]
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    out.extend(ansi_pieces(s).filter_map(|p| match p {
        AnsiPiece::Visible(c) => Some(c),
        AnsiPiece::Escape(_) => None,
    }));
    out
}

/// The single-column marker appended when [`truncate_to_width`] drops
/// content.
pub const ELLIPSIS: char = '\u{2026}';

/// SGR reset appended after a cut that happened inside a styled region.
const RESET: &str = "\x1b[0m";

/// Truncate `s` so its visible width is at most `max_cols`.
///
/// CL-3 / TASK-1969: this is the layout pipeline's documented truncation
/// policy.
///
/// - Escape sequences are preserved (they cost no columns, and dropping them
///   mid-string would change the styling of what survives); a `\x1b[0m` reset
///   is appended whenever a truncated string carried an escape, so the cut
///   cannot leave the terminal in a styled state.
/// - Control characters are dropped, exactly as [`strip_ansi`] drops them.
/// - When content is dropped, the last visible column is spent on
///   [`ELLIPSIS`] so the reader can see the line was cut. `max_cols == 0`
///   therefore yields an empty visible string.
/// - Returns `Cow::Borrowed` when `s` already fits and carries nothing that
///   needs removing, so the common case does not allocate.
#[must_use]
pub fn truncate_to_width(s: &str, max_cols: usize) -> Cow<'_, str> {
    let fits = visible_width(s) <= max_cols;
    if fits && !s.chars().any(is_droppable_control) {
        return Cow::Borrowed(s);
    }
    // When content must be dropped, reserve the last column for the ellipsis
    // marker. A string that already fits (and is only being stripped of
    // control characters) keeps the full budget.
    let body_cols = if fits {
        max_cols
    } else {
        max_cols.saturating_sub(1)
    };
    let mut out = String::with_capacity(s.len());
    let mut used = 0usize;
    let mut had_escape = false;
    let mut truncated = false;
    for piece in ansi_pieces(s) {
        match piece {
            AnsiPiece::Escape(seq) => {
                had_escape = true;
                out.push_str(seq);
            }
            AnsiPiece::Visible(c) => {
                if truncated {
                    continue;
                }
                let w = c.width().unwrap_or(0);
                if used.saturating_add(w) > body_cols {
                    truncated = true;
                    // Mark the cut in place so the ellipsis inherits the
                    // styling of the text it replaces, and any trailing
                    // reset in the source still lands after it.
                    if max_cols > 0 {
                        out.push(ELLIPSIS);
                    }
                    continue;
                }
                used = used.saturating_add(w);
                out.push(c);
            }
        }
    }
    if truncated && had_escape && !out.ends_with(RESET) {
        out.push_str(RESET);
    }
    Cow::Owned(out)
}
