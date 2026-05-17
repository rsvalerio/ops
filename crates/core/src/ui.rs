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
/// Escapes ESC (`\x1b`) and any non-`\t` control character (`< 0x20`, `\x7f`)
/// using `\xNN` so an attacker-controlled error message cannot smuggle ANSI
/// escapes or terminal control codes into operator-facing output. Newlines are
/// the responsibility of the caller — they are split before reaching this
/// helper so each physical line gets its own `ops: <level>:` prefix.
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
            c if (c as u32) < 0x20 || c == '\x7f' || c == '\u{1b}' => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
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
    let mut buf = String::with_capacity(message.len() + level.len() + 8);
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
