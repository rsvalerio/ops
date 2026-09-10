//! ANSI escape stripping, visible-width measurement and width-bounded
//! truncation.
//!
//! ARCH-1 / TASK-0881: split out of `style.rs` so this concern (read-only
//! ANSI grammar handling) is reusable without dragging in the rendering
//! crate's TTY/`NO_COLOR` gating logic.
//!
//! DUP-1 / TASK-0978: the ANSI grammar lives in a single iterator
//! [`AnsiPieces`]; [`visible_width`], [`strip_ansi`],
//! [`strip_ansi_preserving_raw`] and [`truncate_to_width`] all consume it so
//! any future grammar fix lands in one place.
//!
//! DUP-3 / TASK-2148: this is the workspace's *only* ANSI grammar. The
//! private ~150-line copy `ops-cargo-update` used to carry — which handled
//! only `ESC [`/`ESC ]`/nF and had already missed the C1 families — was
//! removed in favour of [`strip_ansi_preserving_raw`], the policy that
//! parser needs over the same iterator.
//!
//! The grammar covers:
//!
//! - **CSI** (`ESC [ … final`): any final byte in `0x40..=0x7E`, not just `m`.
//!   Covers SGR (`m`), cursor moves (`H`/`A`/etc.), and other CSI commands.
//! - **OSC / DCS / SOS / PM / APC** (`ESC ]`/`ESC P`/`ESC X`/`ESC ^`/`ESC _`):
//!   string-introducer sequences terminated by `BEL` (0x07) or the two-byte
//!   ST `ESC \`. Required so terminal hyperlinks (`ESC ]8;;url ESC \\`) and
//!   similar OSC payloads do not contribute to the visible width.
//! - **nF-class escapes** (`ESC` + one or more intermediates in
//!   `0x21..=0x2F`, then a final byte in `0x30..=0x7E`): `ESC ( B` charset
//!   select, `ESC # 8` DECALN, … `0x20` (space) is excluded from the
//!   intermediate range even though ECMA-48 allows it: `ESC` + space is
//!   exactly the shape a stray `ESC` in captured text takes, and consuming
//!   it would swallow the following visible word (the reconciliation kept
//!   cargo-update's PATTERN-1 / TASK-1790 defensive choice).
//! - **Two-byte escapes** (`ESC` + a final byte in `0x30..=0x7E`):
//!   `ESC c` RIS, `ESC 7`, `ESC =`, …
//! - **8-bit C1 introducers** (SEC-11 / TASK-1967): `U+009B` (CSI),
//!   `U+009D` (OSC), `U+0090` (DCS), `U+0098` (SOS), `U+009E` (PM) and
//!   `U+009F` (APC) are the single-code-point equivalents of the two-byte
//!   `ESC` forms above; a terminal in 8-bit mode acts on them identically.
//!   They are consumed with the same payload rules, and `U+009C` (ST)
//!   terminates a string sequence just like `ESC \`.
//! - **Unterminated escapes** (PATTERN-1 / TASK-1028, reconciled from
//!   cargo-update): a sequence cut off by end of input, a scan that hits its
//!   cap ([`CSI_SCAN_CAP`] / [`STRING_SCAN_CAP`]), or a stray introducer
//!   that begins nothing recognised yields its consumed bytes as a
//!   [`Raw`](AnsiPiece::Raw) piece instead of draining the iterator to
//!   end-of-string. Policy decides what happens to those bytes:
//!   [`strip_ansi`] drops them, [`strip_ansi_preserving_raw`] keeps them
//!   verbatim so a downstream validator can reject the line.
//!
//! What the iterator does *not* decide, because the two policies disagree:
//! bare control characters and tab. They are yielded verbatim as
//! [`Char`](AnsiPiece::Char) pieces and each policy function applies its own
//! rule:
//!
//! - **Bare control characters** (SEC-11 / TASK-1967): every remaining C0
//!   code point (`\r`, `\n`, `\x08`, `\x07`, …), `DEL` (`\x7f`) and every
//!   non-introducer C1 code point is dropped by [`strip_ansi`] and
//!   [`truncate_to_width`]. They measure as zero columns but the terminal
//!   *acts* on them — a bare `\r` returns to column 0 and overwrites a box
//!   frame — so a "stripped" string must not still carry them.
//!   [`strip_ansi_preserving_raw`] keeps them: the parser it serves has its
//!   own field validator that rejects any control-carrying field, and
//!   dropping them here would mask a malformed line from that check.
//! - **Tab** (CL-3 / TASK-2019): the one control character that used to be
//!   passed through. It measures as zero columns
//!   (`UnicodeWidthChar::width('\t')` is `None`) while a terminal advances
//!   the cursor to the next 8-column stop, so a tab in captured stderr made
//!   every boxed frame under-pad and the closing bar land short.
//!   [`strip_ansi`] and [`truncate_to_width`] rewrite it to a single space
//!   ([`TAB_REPLACEMENT`]), where the painted string is produced, so
//!   measurement and painting agree. A fixed tab stop cannot be honoured
//!   instead: these helpers see a fragment, not its column offset inside the
//!   frame. [`strip_ansi_preserving_raw`] keeps it verbatim.

use std::borrow::Cow;
use std::str::Chars;

use unicode_width::UnicodeWidthChar;

/// One unit of the ANSI grammar: a complete escape sequence, the raw bytes
/// of an escape that never terminated, or a single character that is part of
/// no sequence.
enum AnsiPiece<'a> {
    /// A complete escape sequence, borrowed from the source string.
    Escape(&'a str),
    /// The bytes consumed while trying to complete an escape that never
    /// terminated: a sequence truncated by end of input, a scan that hit its
    /// cap, or a stray introducer beginning nothing recognised. Starts with
    /// the introducer (`ESC` or a C1 code point).
    Raw(&'a str),
    /// One character that belongs to no escape sequence, verbatim —
    /// including control characters and tab, whose fate each policy
    /// function decides.
    Char(char),
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

/// True for the code points that can begin an escape sequence: `ESC` and
/// the six 8-bit C1 introducers the grammar recognises.
const fn is_escape_introducer(c: char) -> bool {
    matches!(
        c,
        '\x1b' | '\u{90}' | '\u{98}' | '\u{9b}' | '\u{9d}' | '\u{9e}' | '\u{9f}'
    )
}

/// CL-3 / TASK-2019: what a tab is rewritten to by the display policies.
/// One space, not an 8-column stop — see the module docs.
pub const TAB_REPLACEMENT: char = ' ';

/// True for characters the display policies rewrite or remove, i.e. every
/// character for which the output cannot simply borrow the input.
const fn is_rewritten(c: char) -> bool {
    c == '\t' || is_droppable_control(c)
}

/// PATTERN-1 / TASK-1028 (reconciled from cargo-update, DUP-3 / TASK-2148):
/// bound each CSI/nF scan so a truncated input (`...\x1b[3` with no final
/// byte before EOF) or a runaway parameter run cannot drain the iterator to
/// end-of-string and silently swallow trailing visible text. Real CSI
/// sequences are short (~10 bytes); 64 is generous.
const CSI_SCAN_CAP: usize = 64;

/// String-sequence bodies carry URLs (cargo's OSC-8 hyperlinks), so they get
/// a larger — still bounded — budget.
const STRING_SCAN_CAP: usize = 1024;

impl<'a> Iterator for AnsiPieces<'a> {
    type Item = AnsiPiece<'a>;

    fn next(&mut self) -> Option<AnsiPiece<'a>> {
        let rest = self.chars.as_str();
        let ch = self.chars.next()?;
        match ch {
            '\x1b' => Some(self.consume_escape(rest)),
            // 8-bit C1 introducers: same payload rules as their `ESC`
            // two-byte equivalents.
            '\u{9b}' => Some(self.consume_csi(rest)),
            '\u{90}' | '\u{98}' | '\u{9d}' | '\u{9e}' | '\u{9f}' => {
                Some(self.consume_string_terminated(rest))
            }
            c => Some(AnsiPiece::Char(c)),
        }
    }
}

impl<'a> AnsiPieces<'a> {
    /// The prefix of `rest` consumed since `rest` was captured.
    fn consumed_from(&self, rest: &'a str) -> &'a str {
        let taken = rest.len().saturating_sub(self.chars.as_str().len());
        rest.get(..taken).unwrap_or(rest)
    }

    /// The sequence introduced by an `ESC` already taken from the iterator.
    /// Peeks rather than takes so an `ESC` that begins nothing recognised
    /// leaves the following character for the main loop to re-emit.
    fn consume_escape(&mut self, rest: &'a str) -> AnsiPiece<'a> {
        match self.chars.clone().next() {
            Some('[') => {
                self.chars.next();
                self.consume_csi(rest)
            }
            Some(']' | 'P' | 'X' | '^' | '_') => {
                self.chars.next();
                self.consume_string_terminated(rest)
            }
            // nF-class: one or more intermediates, then a final byte. Space
            // (0x20) is excluded from the intermediate range — see the
            // module docs.
            Some(c) if ('\u{21}'..='\u{2f}').contains(&c) => {
                self.chars.next();
                self.consume_nf(rest)
            }
            // Two-byte escape: `ESC c` RIS, `ESC 7`, `ESC =`, ...
            Some(c) if ('\u{30}'..='\u{7e}').contains(&c) => {
                self.chars.next();
                AnsiPiece::Escape(self.consumed_from(rest))
            }
            // A bare `ESC` introducing nothing recognised (including end of
            // input, a space, or a control character): the introducer alone
            // becomes a Raw piece.
            _ => AnsiPiece::Raw(self.consumed_from(rest)),
        }
    }

    /// Consume a CSI body: parameter/intermediate bytes followed by a final
    /// byte in `0x40..=0x7E`. The lead-in (`ESC [` or `U+009B`) is already
    /// consumed.
    fn consume_csi(&mut self, rest: &'a str) -> AnsiPiece<'a> {
        for _ in 0..CSI_SCAN_CAP {
            match self.chars.next() {
                Some(c) if ('\u{40}'..='\u{7e}').contains(&c) => {
                    return AnsiPiece::Escape(self.consumed_from(rest));
                }
                Some(_) => {}
                None => break,
            }
        }
        AnsiPiece::Raw(self.consumed_from(rest))
    }

    /// Consume an OSC/DCS/SOS/PM/APC body, terminated by `BEL` (0x07), the
    /// 8-bit ST (`U+009C`), or the two-byte ST (`ESC \`). A further `ESC`
    /// that does not open an ST ends the sequence — a new escape always
    /// begins a new sequence. The lead-in is already consumed.
    fn consume_string_terminated(&mut self, rest: &'a str) -> AnsiPiece<'a> {
        for _ in 0..STRING_SCAN_CAP {
            match self.chars.next() {
                Some('\x07' | '\u{9c}') => {
                    return AnsiPiece::Escape(self.consumed_from(rest));
                }
                Some('\x1b') => {
                    if self.chars.clone().next() == Some('\\') {
                        self.chars.next();
                    }
                    return AnsiPiece::Escape(self.consumed_from(rest));
                }
                Some(_) => {}
                None => break,
            }
        }
        AnsiPiece::Raw(self.consumed_from(rest))
    }

    /// Consume an nF-class body: the first intermediate is already consumed;
    /// further intermediates in `0x21..=0x2F` may follow before the final
    /// byte in `0x30..=0x7E`. A byte outside both ranges means this was
    /// never an escape — everything consumed becomes a Raw piece, including
    /// that byte.
    fn consume_nf(&mut self, rest: &'a str) -> AnsiPiece<'a> {
        for _ in 0..CSI_SCAN_CAP {
            match self.chars.next() {
                Some(c) if ('\u{30}'..='\u{7e}').contains(&c) => {
                    return AnsiPiece::Escape(self.consumed_from(rest));
                }
                Some(c) if ('\u{21}'..='\u{2f}').contains(&c) => {}
                Some(_) | None => break,
            }
        }
        AnsiPiece::Raw(self.consumed_from(rest))
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
            // Tab is measured as its replacement's one column so the boxed
            // frame's right pad stays exact (CL-3 / TASK-2019).
            AnsiPiece::Char('\t') => Some(1),
            AnsiPiece::Char(c) if is_droppable_control(c) => None,
            AnsiPiece::Char(c) => Some(c.width().unwrap_or(0)),
            AnsiPiece::Escape(_) | AnsiPiece::Raw(_) => None,
        })
        .fold(0usize, usize::saturating_add)
}

/// Remove every ANSI escape sequence and every control character from `s`,
/// rewriting tab to [`TAB_REPLACEMENT`].
///
/// SEC-11 / TASK-1967, CL-3 / TASK-2019: the result is guaranteed to contain
/// no C0 code point, no `DEL`, and no C1 code point — so callers may treat it
/// as safe to print *and* safe to measure with a width helper.
#[must_use]
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    out.extend(ansi_pieces(s).filter_map(|p| match p {
        AnsiPiece::Char('\t') => Some(TAB_REPLACEMENT),
        AnsiPiece::Char(c) if is_droppable_control(c) => None,
        AnsiPiece::Char(c) => Some(c),
        AnsiPiece::Escape(_) | AnsiPiece::Raw(_) => None,
    }));
    out
}

/// Remove complete ANSI escape sequences from `s`, preserving everything
/// else verbatim — the bytes of truncated, runaway or stray escapes, and
/// every control character.
///
/// DUP-3 / TASK-2148: the policy `ops-cargo-update` needs for untrusted
/// subprocess output (PATTERN-1 / TASK-1028). Dropping truncated-escape
/// bytes would silently swallow trailing visible text, and dropping control
/// characters would mask a malformed line from the downstream field
/// validator that rejects any field still carrying one. Callers that want
/// display-safe output should use [`strip_ansi`] instead.
#[must_use]
pub fn strip_ansi_preserving_raw(s: &str) -> Cow<'_, str> {
    // PERF-3 / TASK-0970: fast-path the typical case (no escape introducer
    // in the line — terminals without colour, redirected CI output) by
    // returning a borrow; only allocate when a sequence must be removed.
    if !s.chars().any(is_escape_introducer) {
        return Cow::Borrowed(s);
    }
    let mut out = String::with_capacity(s.len());
    for piece in ansi_pieces(s) {
        match piece {
            AnsiPiece::Escape(_) => {}
            AnsiPiece::Raw(raw) => out.push_str(raw),
            AnsiPiece::Char(c) => out.push(c),
        }
    }
    Cow::Owned(out)
}

/// The single-column marker appended when [`truncate_to_width`] drops
/// content.
pub const ELLIPSIS: char = '…';

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
/// - Control characters are dropped and tab is rewritten to
///   [`TAB_REPLACEMENT`], exactly as [`strip_ansi`] treats them.
/// - When content is dropped, the last visible column is spent on
///   [`ELLIPSIS`] so the reader can see the line was cut. `max_cols == 0`
///   therefore yields an empty visible string.
/// - Returns `Cow::Borrowed` when `s` already fits and carries nothing that
///   needs removing, so the common case does not allocate.
#[must_use]
pub fn truncate_to_width(s: &str, max_cols: usize) -> Cow<'_, str> {
    let fits = visible_width(s) <= max_cols;
    if fits && !s.chars().any(is_rewritten) {
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
        let c = match piece {
            // Raw pieces are escape-shaped (they begin with the introducer)
            // and cost no columns; preserving them keeps the truncated
            // bytes observable, the same reasoning as
            // strip_ansi_preserving_raw.
            AnsiPiece::Escape(seq) | AnsiPiece::Raw(seq) => {
                had_escape = true;
                out.push_str(seq);
                continue;
            }
            AnsiPiece::Char('\t') => TAB_REPLACEMENT,
            AnsiPiece::Char(c) if is_droppable_control(c) => continue,
            AnsiPiece::Char(c) => c,
        };
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
    if truncated && had_escape && !out.ends_with(RESET) {
        out.push_str(RESET);
    }
    Cow::Owned(out)
}
