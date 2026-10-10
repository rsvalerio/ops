//! Integration tests over the crate's public API.
//!
//! Tests are split by concern into submodules. Shared
//! imports, the [`render_line`] helper, and the [`MINIMAL_THEME_TOML`]
//! fixture live here so each submodule can pick them up via `use super::*;`.
//!
//! Only the colour-gate internals (`color_enabled_for`) and the crate-visible
//! `render_error_block_gated` need private access; their tests stay inline in
//! `src/tests/`.

use indexmap::IndexMap;
use ops_core::output::{ErrorDetail, StepLine, StepStatus};
use ops_core::test_utils::EnvGuard;
use ops_theme::*;
use serial_test::serial;

mod boxed_layout;
mod deserialize;
mod edge_case_width;
mod format_duration;
mod left_pad;
mod overhead_diagnostic;
mod render_basics;
mod render_report;
mod render_summary;
mod resolve;
mod unicode;

/// Minimal valid `ThemeConfig` TOML with all required fields.
/// Tests that need to tweak one field can append/override after this base.
const MINIMAL_THEME_TOML: &str = r#"
icon_pending = "○"
icon_running = ""
icon_succeeded = "●"
icon_failed = "✗"
icon_skipped = "—"
separator_char = '.'
step_indent = "  "
running_template = "  {spinner:.cyan}{msg}"
tick_chars = "⠁⠂⠄ "
running_template_overhead = 7
summary_prefix = "→ "
summary_separator = ""
left_pad = 0
"#;

fn render_line(
    theme: &ConfigurableTheme,
    status: StepStatus,
    label: &str,
    elapsed: Option<f64>,
) -> String {
    let step = StepLine::new(status, label.to_string(), elapsed);
    theme.render(&step, 80)
}
