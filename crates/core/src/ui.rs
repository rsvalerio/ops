//! Thin user-facing reporter for CLI diagnostics.
//!
//! Distinguishes three output channels:
//! - `tracing::{warn,info,debug}` — structured logs, filtered by `OPS_LOG_LEVEL`.
//! - `ops_core::ui::{note,warn,error}` — always-on user-facing messages on
//!   stderr with a consistent `ops: ...` prefix.
//! - Command output — always stdout.
//!
//! These helpers swallow broken-pipe errors (there is no recovery channel for
//! a failed stderr write), but keep the message format uniform so downstream
//! tooling can grep for "ops: error:" / "ops: warning:".

use std::fmt::Write as _;
use std::io::Write;

/// SEC-21 (TASK-0981): sanitise a single line for stderr emission.
///
/// # Character classes neutralised
///
/// Every character in these classes is replaced by a printable escape, so it
/// reaches the terminal as text rather than as a control sequence:
///
/// | Class | Range | Rendered as |
/// |---|---|---|
/// | C0 controls (including ESC `U+001B`) | `U+0000..=U+001F` | `\xNN` |
/// | DEL | `U+007F` | `\x7f` |
/// | C1 controls (including CSI `U+009B`, OSC `U+009D`) | `U+0080..=U+009F` | `\xNN` |
/// | Bidi overrides / embeddings | `U+202A..=U+202E` | `\u{NNNN}` |
/// | Bidi isolates | `U+2066..=U+2069` | `\u{NNNN}` |
///
/// TAB (`\t`) is passed through verbatim; it is the one control character
/// operators expect to survive in a diagnostic. Newlines are the
/// responsibility of the caller — they are split before reaching this helper
/// so each physical line gets its own `ops: <level>:` prefix.
///
/// SEC-21 / TASK-1843 records two deliberate decisions:
///
/// 1. **C1 is escaped even though not every emulator acts on it.** `U+009B`
///    is the single-character form of `ESC [` and `U+009D` of `ESC ]`;
///    whether a given terminal honours C1 delivered as UTF-8 varies (xterm
///    gates it behind a resource, some VTE-family terminals historically did
///    not). Escaping ESC while passing CSI through would leave the shorter
///    spelling of the same primitive open, and the range costs one arm.
/// 2. **Bidi controls are neutralised, not passed through.** They are the
///    Trojan-Source vector: they reorder rendered text without changing the
///    bytes, so a `--dry-run` audit preview could display a command line that
///    is not the one that will run. Bidi *marks* (`U+200E`/`U+200F`) and
///    other invisible formatting characters are out of scope — they cannot
///    reorder a run of text — so this is an escape of the reordering
///    controls, not general Unicode confusable filtering.
///
/// SEC-21 / TASK-1184: also exposed for the `ops --dry-run` audit channel,
/// which prints (env-expanded) program / args / env values / cwd verbatim
/// to stdout. An adversarial `.ops.toml` value (or `${VAR}` expansion of
/// one) containing ANSI clear-screen / cursor-move sequences can otherwise
/// repaint the operator's terminal during a preview, defeating the whole
/// purpose of dry-run.
pub fn sanitise_line(line: &str, out: &mut String) {
    for ch in line.chars() {
        match ch {
            '\t' => out.push('\t'),
            // C0 (ESC included, at U+001B), DEL, and C1. `\xNN` keeps the
            // existing rendering for the first two.
            c if u32::from(c) < 0x20 || c == '\u{7f}' || ('\u{80}'..='\u{9f}').contains(&c) => {
                let _ = write!(out, "\\x{:02x}", u32::from(c));
            }
            c if is_bidi_control(c) => {
                let _ = write!(out, "\\u{{{:04x}}}", u32::from(c));
            }
            c => out.push(c),
        }
    }
}

/// SEC-21 / TASK-1843: the Unicode bidirectional *reordering* controls —
/// the embeddings/overrides (`U+202A..=U+202E`) and the isolates
/// (`U+2066..=U+2069`). These are the Trojan-Source characters; they are
/// escaped by [`sanitise_line`] rather than stripped so an operator can see
/// that the input contained them.
const fn is_bidi_control(c: char) -> bool {
    matches!(c, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

fn emit(level: &str, message: &str) {
    emit_to(level, message, &mut std::io::stderr().lock());
}

/// Writer-generic core of [`emit`]: renders `message` through the SEC-21 line-
/// split + sanitise pipeline into `w`. Production callers pass a locked stderr
/// handle; tests pass a `Vec<u8>` so they can assert on the exact bytes the
/// production pipeline produces (DUP-1 TASK-1031).
pub(crate) fn emit_to<W: Write>(level: &str, message: &str, w: &mut W) {
    // SEC-21 (TASK-0981): split on `\n` so a multi-line anyhow chain renders
    // as continuation lines indented under the prefix, and an attacker-
    // injected `\n` cannot forge a top-level `ops: <level>:` line. Each
    // physical line is then sanitised to neutralise ANSI / control bytes.
    //
    // PERF-3 / TASK-1422: render the full output into a single buffer and
    // emit it with one `write_all`. Stderr is unbuffered when piped (the
    // typical CI / capture path), so a writeln-per-line loop issued N
    // separate syscalls and risked interleaving with parallel writers.
    let mut buf =
        String::with_capacity(message.len().saturating_add(level.len()).saturating_add(8));
    let mut first = true;
    for line in message.split('\n') {
        let prefix = if first { "" } else { "  " };
        let _ = write!(buf, "ops: {level}: {prefix}");
        sanitise_line(line, &mut buf);
        buf.push('\n');
        first = false;
    }
    let _ = w.write_all(buf.as_bytes());
}

/// Print an informational note, e.g. `ops: note: ...`.
pub fn note(message: impl AsRef<str>) {
    emit("note", message.as_ref());
}

/// Print a warning, e.g. `ops: warning: ...`.
pub fn warn(message: impl AsRef<str>) {
    emit("warning", message.as_ref());
}

/// Print an error, e.g. `ops: error: ...`.
pub fn error(message: impl AsRef<str>) {
    emit("error", message.as_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Render a message through the same pipeline `emit` uses, but into a
    /// `Vec<u8>` so we can assert on the exact output without touching stderr.
    /// Routes through production `emit_to` (DUP-1 TASK-1031) so SEC-21
    /// regressions catch drift in the real pipeline, not a parallel copy.
    fn render(level: &str, message: &str) -> String {
        let mut out: Vec<u8> = Vec::new();
        emit_to(level, message, &mut out);
        String::from_utf8(out).expect("emit_to writes UTF-8")
    }

    #[test]
    fn plain_message_unchanged() {
        assert_eq!(render("error", "boom"), "ops: error: boom\n");
    }

    /// SEC-21 AC#3: an injected newline must not produce a second physical
    /// line beginning with `ops:`.
    #[test]
    fn injected_newline_does_not_forge_top_level_line() {
        let out = render("error", "real\nops: warning: forged");
        let mut lines = out.lines();
        assert_eq!(lines.next(), Some("ops: error: real"));
        let second = lines.next().expect("continuation line");
        assert!(
            !second.starts_with("ops: warning:") && !second.starts_with("ops: error:  ops:"),
            "continuation must not start a forged ops: line, got {second:?}"
        );
        assert!(second.starts_with("ops: error:   "));
    }

    /// SEC-21 / TASK-1843 AC#4: `U+009B` is CSI — the single-character form
    /// of `ESC [` — and `U+009D` is OSC. Escaping ESC while passing these
    /// through left the shorter spelling of the same primitive open.
    #[test]
    fn c1_control_characters_are_escaped() {
        let mut out = String::new();
        sanitise_line("a\u{9b}31mb\u{9d}c\u{80}", &mut out);
        assert_eq!(out, "a\\x9b31mb\\x9dc\\x80");
        assert!(!out.contains('\u{9b}'), "CSI must not survive: {out:?}");
    }

    /// SEC-21 / TASK-1843 AC#4: bidi overrides and isolates reorder rendered
    /// text without changing the bytes, so a dry-run preview could display a
    /// command line that is not the one that will run.
    #[test]
    fn bidi_controls_are_escaped() {
        let mut out = String::new();
        sanitise_line("rm \u{202e}txt.exe\u{202c} now\u{2066}x\u{2069}", &mut out);
        assert_eq!(out, "rm \\u{202e}txt.exe\\u{202c} now\\u{2066}x\\u{2069}");
        assert!(
            !out.chars().any(super::is_bidi_control),
            "no bidi control may survive: {out:?}"
        );
    }

    /// The neighbouring code points must stay untouched — the escape is a
    /// bounded range, not a blanket filter on non-ASCII text.
    #[test]
    fn characters_adjacent_to_the_escaped_ranges_pass_through() {
        let mut out = String::new();
        sanitise_line(
            "\u{7e}\u{a0}\u{2029}\u{2065}\u{206a}caf\u{e9} 名前",
            &mut out,
        );
        assert_eq!(out, "\u{7e}\u{a0}\u{2029}\u{2065}\u{206a}caf\u{e9} 名前");
    }

    /// SEC-21 AC#1: ANSI ESC and other control bytes are escaped, not passed
    /// through to a TTY.
    #[test]
    fn ansi_and_control_bytes_are_escaped() {
        let out = render("error", "x\u{1b}[31mred\u{07}\u{0c}y");
        assert!(!out.contains('\u{1b}'));
        assert!(!out.contains('\u{07}'));
        assert!(!out.contains('\u{0c}'));
        assert!(out.contains("\\x1b"));
        assert!(out.contains("\\x07"));
        assert!(out.contains("\\x0c"));
    }

    /// SEC-21 AC#2: legitimate multi-line anyhow chains stay readable as
    /// indented continuation lines under the prefix.
    #[test]
    fn multiline_chain_renders_as_indented_continuations() {
        let out = render("error", "outer\n  caused by: inner");
        let mut lines = out.lines();
        assert_eq!(lines.next(), Some("ops: error: outer"));
        assert_eq!(lines.next(), Some("ops: error:     caused by: inner"));
    }

    #[test]
    fn tab_is_preserved() {
        let out = render("note", "a\tb");
        assert!(out.contains("a\tb"));
    }

    /// ERR-7 (TASK-1370): the program-root error printer in
    /// `crates/cli/src/main.rs` renders an `anyhow::Error` chain via
    /// `format!("{e:#}")` and passes the assembled string to
    /// [`error`]. Pin that the assembly-then-sanitise order escapes ESC
    /// bytes that originated *inside* an interpolated value of a nested
    /// cause — the chain-joiner `: ` produced by anyhow's alternate
    /// Display must not exempt inner-cause Display strings from the
    /// SEC-21 sweep.
    #[test]
    fn anyhow_chain_alternate_display_routed_through_emit_sanitises_inner_cause() {
        let hostile_path = "evil\u{1b}[2J\u{1b}[31m.txt";
        let inner = anyhow::anyhow!("loading {hostile_path}");
        let err = inner.context("init failed");
        let assembled = format!("{err:#}");
        let out = render("error", &assembled);
        assert!(
            !out.contains('\u{1b}'),
            "ESC must be escaped end-to-end: {out:?}"
        );
        assert!(
            out.contains("\\x1b"),
            "ESC must be rendered as \\x1b: {out:?}"
        );
        assert!(
            out.contains("init failed"),
            "outer context preserved: {out:?}"
        );
        assert!(out.contains("loading"), "inner message preserved: {out:?}");
    }
}
