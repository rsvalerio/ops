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
///
/// # Panics
///
/// Never in practice: the template is the string literal `{msg}`, which
/// `indicatif` always accepts (docs/clippy.md layer 3).
#[allow(clippy::expect_used)]
pub fn pending_style() -> ProgressStyle {
    ProgressStyle::with_template("{msg}").expect("static pending template")
}

pub fn build_running_style(
    resolved_theme: &theme::ConfigurableTheme,
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
            let _ = write!(w, "{}", theme::format_running_elapsed(state.elapsed().as_secs_f64()));
        });
    Ok(style)
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;
    use indicatif::{ProgressBar, ProgressDrawTarget, TermLike};
    use ops_core::output::{StepLine, StepStatus};
    use ops_theme::ThemeConfig;
    use std::io;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    const COLUMNS: u16 = 72;

    /// A terminal that records every string indicatif draws to it.
    #[derive(Debug, Clone, Default)]
    struct RecordingTerm(Arc<Mutex<Vec<String>>>);

    impl RecordingTerm {
        fn record(&self, s: &str) {
            self.0.lock().expect("recording lock").push(s.to_string());
        }
    }

    impl TermLike for RecordingTerm {
        fn width(&self) -> u16 {
            COLUMNS
        }
        fn move_cursor_up(&self, _: usize) -> io::Result<()> {
            Ok(())
        }
        fn move_cursor_down(&self, _: usize) -> io::Result<()> {
            Ok(())
        }
        fn move_cursor_right(&self, _: usize) -> io::Result<()> {
            Ok(())
        }
        fn move_cursor_left(&self, _: usize) -> io::Result<()> {
            Ok(())
        }
        fn write_line(&self, s: &str) -> io::Result<()> {
            self.record(s);
            Ok(())
        }
        fn write_str(&self, s: &str) -> io::Result<()> {
            self.record(s);
            Ok(())
        }
        fn clear_line(&self) -> io::Result<()> {
            Ok(())
        }
        fn flush(&self) -> io::Result<()> {
            Ok(())
        }
    }

    /// The themes users actually get: the ones embedded in `.default.ops.toml`.
    fn builtin_themes() -> IndexMap<String, ThemeConfig> {
        let config: ops_core::config::Config =
            toml::from_str(ops_core::config::default_ops_toml()).expect("default config parses");
        config.themes
    }

    /// Draw one running row exactly as the runner does — the theme renders
    /// the message, the running style expands the template around it — and
    /// return what indicatif wrote, ANSI stripped.
    fn drawn_running_row(
        theme: &theme::ConfigurableTheme,
        name: &str,
        label: &str,
        elapsed: Duration,
    ) -> String {
        let term = RecordingTerm::default();
        let bar = ProgressBar::with_draw_target(
            None,
            ProgressDrawTarget::term_like(Box::new(term.clone())),
        )
        .with_elapsed(elapsed);
        bar.set_style(build_running_style(theme, name).expect("running style builds"));
        let step = StepLine::new(StepStatus::Running, label.to_string(), None);
        bar.set_message(theme.render(&step, COLUMNS));
        bar.tick();
        let drawn = term.0.lock().expect("recording lock").clone();
        drawn
            .iter()
            .rev()
            .map(|s| theme::strip_ansi(s))
            .find(|s| !s.trim().is_empty())
            .unwrap_or_else(|| panic!("theme '{name}' drew no running row"))
    }

    /// A running row must fit the column budget it was rendered for. The
    /// `running_template_overhead` of a built-in theme once assumed a
    /// three-column `{elapsed}` while `format_duration` prints at least five
    /// (`0.00s`), so compact and classic ran 2 columns past a 72-column
    /// terminal and flux 4, wrapping every running row; and past an hour
    /// `format_duration` grows to 8 columns, overflowing even studio's
    /// six-column field until running times got their own formatter.
    #[test]
    fn builtin_running_rows_fit_their_column_budget() {
        let long_label = "cargo test --workspace --all-features -- ".repeat(4);
        let elapsed = [
            Duration::ZERO,
            Duration::from_millis(9_990),
            Duration::from_millis(59_990),
            Duration::from_secs(59 * 60 + 59),
            // Past an hour `format_duration` would print `1h0m10s` and
            // `1h10m10s` (7 and 8 columns); the running formatter keeps six.
            Duration::from_secs(3_610),
            Duration::from_secs(4_210),
            Duration::from_mins(99 * 60 + 59),
            Duration::from_hours(1_000),
        ];
        for (name, config) in builtin_themes() {
            let theme = theme::ConfigurableTheme::new(config);
            for label in ["cargo build", long_label.as_str()] {
                for duration in elapsed {
                    let row = drawn_running_row(&theme, &name, label, duration);
                    let width = theme::visible_width(&row);
                    assert_eq!(
                        width,
                        usize::from(COLUMNS),
                        "theme '{name}' running row at {duration:?} is {width} columns, \
                         not the {COLUMNS}-column budget: {row:?}"
                    );
                }
            }
        }
    }
}
