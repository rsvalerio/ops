//! Parsing logic for `cargo upgrade` and `cargo deny` output.
//!
//! ARCH-1 / TASK-1121: split the previous single-file parser into per-tool
//! submodules. Each submodule owns its own constants, types, and helpers so
//! a format change in one tool does not churn the other's state machine.

mod deny;
mod upgrade;

pub use deny::{interpret_deny_result, parse_deny_output, run_cargo_deny};
pub use upgrade::{categorize_upgrades, parse_upgrade_table, run_cargo_upgrade_dry_run};

#[cfg(test)]
pub(crate) use deny::MISSING_SEVERITY_SENTINEL;
#[cfg(test)]
pub(crate) use upgrade::interpret_upgrade_output;

/// Truncate a log line for tracing — operators get enough context to
/// diagnose schema drift without flooding logs with multi-KB cargo-deny
/// diagnostics.
pub(crate) fn truncate_for_log(s: &str) -> String {
    const MAX: usize = 200;
    if s.len() <= MAX {
        s.to_string()
    } else {
        let mut end = MAX;
        while !s.is_char_boundary(end) {
            end -= 1;
        }
        format!("{}…", &s[..end])
    }
}
