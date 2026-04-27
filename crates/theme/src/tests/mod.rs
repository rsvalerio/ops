//! Tests for theme types and rendering.
//!
//! Tests are split by concern into submodules — see TASK-0353. Shared
//! imports, the [`render_line`] helper, and the [`MINIMAL_THEME_TOML`]
//! fixture live here so each submodule can pick them up via `use super::*;`.

use super::*;
use indexmap::IndexMap;
use ops_core::output::{ErrorDetail, StepLine, StepStatus};
use ops_core::test_utils::EnvGuard;
use serial_test::serial;

mod boxed_layout;
mod deserialize;
mod edge_case_width;
mod error_block_color;
mod format_duration;
mod left_pad;
mod render_basics;
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
    theme: &dyn StepLineTheme,
    status: StepStatus,
    label: &str,
    elapsed: Option<f64>,
) -> String {
    let step = StepLine {
        status,
        label: label.to_string(),
        elapsed,
    };
    theme.render(&step, 80)
}
