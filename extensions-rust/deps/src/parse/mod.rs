//! Parsing logic for `cargo upgrade`, `cargo deny` and `cargo machete` output.
//!
//! One submodule per tool. Each owns its own constants, types, and helpers,
//! so a format change in one tool does not churn the other's state machine.

mod deny;
mod machete;
mod upgrade;

// Only the spawning entry points leave this module; each tool's guarded
// `interpret_*_output` is reached through them. The unguarded slicers
// (`parse_upgrade_table`, `parse_deny_output`) discard their parse
// diagnostics and so cannot report format drift; they are `#[cfg(test)]`-only
// and the tests reach them through their own submodule.
pub use deny::run_cargo_deny;
pub use machete::run_cargo_machete;
pub use upgrade::{categorize_upgrades, run_cargo_upgrade_dry_run};

#[cfg(test)]
pub use deny::MISSING_SEVERITY_SENTINEL;

/// Truncate a log line for tracing — operators get enough context to
/// diagnose schema drift without flooding logs with multi-KB cargo-deny
/// diagnostics.
pub fn truncate_for_log(s: &str) -> String {
    const MAX: usize = 200;
    if s.len() <= MAX {
        s.to_string()
    } else {
        // `floor_char_boundary` backs off to the nearest char boundary, so
        // the head never splits a multi-byte codepoint.
        let head = s.get(..s.floor_char_boundary(MAX)).unwrap_or_default();
        format!("{head}…")
    }
}
