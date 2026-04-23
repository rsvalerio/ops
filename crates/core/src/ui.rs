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

use std::io::Write;

fn emit(level: &str, message: &str) {
    let mut err = std::io::stderr().lock();
    let _ = writeln!(err, "ops: {level}: {message}");
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
