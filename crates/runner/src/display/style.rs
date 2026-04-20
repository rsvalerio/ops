//! Progress-bar style construction.

use anyhow::Context;
use indicatif::{ProgressState, ProgressStyle};
use ops_theme as theme;
use std::fmt::Write as FmtWrite;

/// Spinner tick interval in milliseconds.
///
/// 80ms = ~12.5 FPS, smooth without excessive wakeups.
pub const SPINNER_TICK_INTERVAL_MS: u64 = 80;

/// Build a fresh pending style. Trivial template avoids storing a field just to clone it.
pub fn pending_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg}").expect("static pending template")
}

pub fn build_running_style(
    resolved_theme: &dyn theme::StepLineTheme,
    theme_name: &str,
) -> anyhow::Result<ProgressStyle> {
    let left_pad_str = " ".repeat(resolved_theme.left_pad());
    let padded = format!("{}{}", left_pad_str, resolved_theme.running_template());
    let style = ProgressStyle::with_template(&padded)
        .with_context(|| {
            format!(
                "invalid running_template for theme '{}': {}",
                theme_name,
                resolved_theme.running_template()
            )
        })?
        .tick_chars(resolved_theme.tick_chars())
        .with_key("elapsed", |state: &ProgressState, w: &mut dyn FmtWrite| {
            let _ = write!(
                w,
                "{}",
                theme::format_duration(state.elapsed().as_secs_f64())
            );
        });
    Ok(style)
}
