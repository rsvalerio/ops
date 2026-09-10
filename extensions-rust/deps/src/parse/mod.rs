//! Parsing logic for `cargo upgrade` and `cargo deny` output.
//!
//! One submodule per tool. Each owns its own constants, types, and helpers,
//! so a format change in one tool does not churn the other's state machine.

mod deny;
mod upgrade;

pub use deny::{interpret_deny_result, parse_deny_output, run_cargo_deny};
pub use upgrade::{categorize_upgrades, interpret_upgrade_output, run_cargo_upgrade_dry_run};

// `parse_upgrade_table` is deliberately absent from this list. It discards the
// parse diagnostics and so cannot report cargo-edit format drift; only
// `interpret_upgrade_output` applies the guards, which makes it the published
// counterpart of `interpret_deny_result`. The table-shape tests reach the
// unguarded slicing through `upgrade`'s own `#[cfg(test)]` wrapper.

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
        // Walk back from MAX to the nearest char boundary: `get` yields
        // `None` while `end` sits inside a multi-byte codepoint, so the head
        // can never split one. Index 0 is always a boundary, which bounds
        // the walk.
        let mut end = MAX;
        // Index 0 is always a boundary, so the loop stops at or above 0 and
        // `saturating_sub(1)` equals `-= 1` exactly.
        let head = loop {
            if let Some(head) = s.get(..end) {
                break head;
            }
            end = end.saturating_sub(1);
        };
        format!("{head}…")
    }
}
