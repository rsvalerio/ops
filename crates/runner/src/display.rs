//! Progress display: step-line rendering, progress bars, and CLI event handling.

use crate::command::RunnerEvent;
use ops_core::config::{self, CommandId};
use ops_core::output::{tail_lines, ErrorDetail, StepLine, StepStatus};
use ops_theme::{self as theme, ThemeConfig};

use anyhow::Context;
use indexmap::IndexMap;
use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressState, ProgressStyle};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;
use std::io::{self, IsTerminal, Write};
use std::time::Duration;

/// Spinner tick interval in milliseconds.
///
/// This value balances smooth animation with minimal CPU overhead.
/// 80ms = ~12.5 FPS, which appears smooth without excessive wakeups.
const SPINNER_TICK_INTERVAL_MS: u64 = 80;

/// Default number of stderr tail lines to show in error details.
#[cfg(test)]
const DEFAULT_STDERR_TAIL_LINES: usize = 5;

/// Write to stderr, logging at debug level on failure.
///
/// IO errors to stderr are typically terminal issues (broken pipe, no TTY)
/// that the application cannot recover from. We log at debug level and continue,
/// as crashing on stderr write failures would be unexpected behavior.
#[inline]
fn write_stderr(line: Option<&str>) {
    let result = match line {
        Some(text) => writeln!(io::stderr(), "{}", text),
        None => writeln!(io::stderr()),
    };
    if let Err(e) = result {
        tracing::debug!(error = %e, "stderr write failed");
    }
}

/// Render configuration extracted from OutputConfig.
pub struct RenderConfig {
    pub theme: Box<dyn theme::StepLineTheme>,
    pub columns: u16,
    pub is_tty: bool,
    pub show_error_detail: bool,
    pub stderr_tail_lines: usize,
}

/// Renders error detail blocks for failed steps.
///
/// This struct encapsulates error detail rendering to support future growth.
/// Currently kept minimal as the display logic is manageable.
pub struct ErrorDetailRenderer<'a> {
    theme: &'a dyn theme::StepLineTheme,
    columns: u16,
}

impl<'a> ErrorDetailRenderer<'a> {
    pub fn new(theme: &'a dyn theme::StepLineTheme, columns: u16) -> Self {
        Self { theme, columns }
    }

    pub fn render(&self, message: &str, stderr_tail: &[String]) -> Vec<String> {
        let detail = ErrorDetail {
            message: message.to_string(),
            stderr_tail: stderr_tail.to_vec(),
        };
        self.theme.render_error_detail(&detail, self.columns)
    }

    pub fn extract_stderr_tail(stderr_lines: &[String], max_lines: usize) -> Vec<String> {
        tail_lines(stderr_lines, max_lines).to_vec()
    }
}

/// Encapsulates progress bar state and rendering for the CLI event loop.
///
/// # Architecture (CQ-008)
///
/// This struct handles several distinct concerns:
/// - **Rendering config**: Theme, columns, TTY detection, error detail settings
/// - **Progress state**: MultiProgress, bars, step metadata, stderr capture
/// - **Event routing**: Converting `RunnerEvent` to visual output
/// - **Error display**: Rendering failed step details via `ErrorDetailRenderer`
///
/// While this could be split into smaller structs (ProgressState, EventDispatcher,
/// ErrorRenderer), the current design is kept as a single struct because:
///
/// 1. All concerns share the same render configuration
/// 2. State (bars, stderr) is tightly coupled to event handling
/// 3. The public API is simple: `handle_event(RunnerEvent)`
/// 4. Test coverage is comprehensive
///
/// The `ErrorDetailRenderer` has already been extracted for error formatting,
/// demonstrating the pattern for future extractions if needed.
///
/// ## Future Refactoring
///
/// If this struct grows beyond ~50 methods or 500 lines of non-test code,
/// consider extracting:
/// - `ProgressState`: bars, steps, step_stderr, display_map
/// - `EventRouter`: handle_event dispatcher + on_* methods
pub struct ProgressDisplay {
    render: RenderConfig,
    multi: MultiProgress,
    bars: Vec<ProgressBar>,
    steps: Vec<(String, String)>,
    step_stderr: HashMap<String, Vec<String>>,
    // PERF-3: `running_style` is cloned per progress bar because `indicatif::ProgressBar::with_style`
    // takes ownership. Acceptable for typical step counts (<20 commands).
    // `pending_style` was removed — it's trivially reconstructed via `pending_style()`.
    running_style: ProgressStyle,
    display_map: HashMap<String, String>,
    footer_separator: Option<ProgressBar>,
    footer_bar: Option<ProgressBar>,
    completed_steps: usize,
    total_steps: usize,
}

impl ProgressDisplay {
    /// Build a fresh pending style. Trivial template avoids storing a field just to clone it.
    fn pending_style() -> ProgressStyle {
        ProgressStyle::with_template("{msg}").expect("static pending template")
    }

    fn is_stderr_tty() -> bool {
        std::io::stderr().is_terminal()
    }

    pub fn new(
        output: &config::OutputConfig,
        display_map: HashMap<String, String>,
        custom_themes: &IndexMap<String, ThemeConfig>,
    ) -> anyhow::Result<Self> {
        Self::new_with_tty_check(output, display_map, custom_themes, Self::is_stderr_tty)
    }

    fn new_with_tty_check<F>(
        output: &config::OutputConfig,
        display_map: HashMap<String, String>,
        custom_themes: &IndexMap<String, ThemeConfig>,
        is_tty_fn: F,
    ) -> anyhow::Result<Self>
    where
        F: FnOnce() -> bool,
    {
        let is_tty = is_tty_fn();
        let multi = MultiProgress::with_draw_target(if is_tty {
            ProgressDrawTarget::stderr()
        } else {
            ProgressDrawTarget::hidden()
        });
        let resolved_theme = theme::resolve_theme(&output.theme, custom_themes)?;
        let left_pad_str = " ".repeat(resolved_theme.left_pad());
        let padded_running_template =
            format!("{}{}", left_pad_str, resolved_theme.running_template());
        let running_style = ProgressStyle::with_template(&padded_running_template)
            .with_context(|| {
                format!(
                    "invalid running_template for theme '{}': {}",
                    output.theme,
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

        Ok(Self {
            render: RenderConfig {
                theme: resolved_theme,
                columns: output.columns,
                is_tty,
                show_error_detail: output.show_error_detail,
                stderr_tail_lines: output.stderr_tail_lines,
            },
            multi,
            bars: Vec::new(),
            steps: Vec::new(),
            step_stderr: HashMap::new(),
            running_style,
            display_map,
            footer_separator: None,
            footer_bar: None,
            completed_steps: 0,
            total_steps: 0,
        })
    }

    /// Returns a reference to the render config for testing.
    #[cfg(test)]
    pub fn render_config(&self) -> &RenderConfig {
        &self.render
    }

    fn step_index(&self, id: &str) -> Option<usize> {
        self.steps.iter().position(|(sid, _)| sid == id)
    }

    /// DUP-008: Helper to write line to stderr when not in TTY mode.
    #[inline]
    fn write_non_tty(&self, line: &str) {
        if !self.render.is_tty {
            write_stderr(Some(line));
        }
    }

    fn emit_line(&self, line: &str) {
        if self.render.is_tty {
            if let Err(e) = self.multi.println(line) {
                tracing::debug!(error = %e, "MultiProgress println failed");
            }
        } else if line.is_empty() {
            write_stderr(None);
        } else {
            write_stderr(Some(line));
        }
    }

    fn finish_bar(&self, bar: &ProgressBar, line: &str) {
        bar.set_style(Self::pending_style());
        bar.finish_with_message(line.to_string());
        self.write_non_tty(line);
    }

    /// Dispatch a RunnerEvent to the appropriate handler method.
    ///
    /// CQ-009: This method is a dispatcher that routes events to specialized
    /// handlers. At 25 lines with 8 match arms, it's at the threshold of what
    /// would benefit from refactoring. The current design is kept because:
    ///
    /// 1. Each match arm is a single method call -- clear and readable
    /// 2. The match is exhaustive, ensuring all events are handled
    /// 3. Extracting to a visitor pattern would add indirection without benefit
    ///
    /// If more event types are added, consider:
    /// - Grouping related events (StepStarted/StepOutput/StepFinished) into sub-handlers
    /// - Using a `fn handle_X(&mut self, ...)` pattern for each event type
    pub fn handle_event(&mut self, event: RunnerEvent) {
        match event {
            RunnerEvent::PlanStarted { command_ids } => self.on_plan_started(&command_ids),
            RunnerEvent::StepStarted { id, .. } => self.on_step_started(&id),
            RunnerEvent::StepOutput { id, line, stderr } => self.on_step_output(&id, line, stderr),
            RunnerEvent::StepFinished {
                id,
                duration_secs,
                display_cmd,
            } => self.on_step_finished(&id, duration_secs, display_cmd.as_deref()),
            RunnerEvent::StepSkipped { id, display_cmd } => {
                self.on_step_skipped(&id, display_cmd.as_deref())
            }
            RunnerEvent::StepFailed {
                id,
                duration_secs,
                message,
                display_cmd,
            } => self.on_step_failed(&id, duration_secs, &message, display_cmd.as_deref()),
            RunnerEvent::RunFinished {
                duration_secs,
                success,
            } => self.on_run_finished(duration_secs, success),
        }
    }

    fn resolve_step_display(&self, id: &CommandId) -> (String, String) {
        let id_str = id.to_string();
        let display = self
            .display_map
            .get(id.as_str())
            .cloned()
            .unwrap_or_else(|| {
                tracing::trace!(id = %id, "display_map fallback: using id as display");
                id_str.clone()
            });
        (id_str, display)
    }

    fn on_plan_started(&mut self, command_ids: &[CommandId]) {
        let ids_as_strings: Vec<String> = command_ids.iter().map(|id| id.to_string()).collect();
        self.steps = command_ids
            .iter()
            .map(|id| self.resolve_step_display(id))
            .collect();

        let header_lines = self
            .render
            .theme
            .render_plan_header(&ids_as_strings, self.render.columns);
        for line in &header_lines {
            self.emit_line(line);
        }

        self.create_pending_bars();

        self.total_steps = command_ids.len();
        self.completed_steps = 0;

        self.create_footer();
    }

    fn create_pending_bars(&mut self) {
        let pending_lines: Vec<String> = self
            .steps
            .iter()
            .map(|(_, display)| {
                let step = StepLine {
                    status: StepStatus::Pending,
                    label: display.clone(),
                    elapsed: None,
                };
                self.render.theme.render(&step, self.render.columns)
            })
            .collect();

        self.bars.clear();
        for line in &pending_lines {
            let pb = self.multi.add(
                ProgressBar::new_spinner()
                    .with_style(Self::pending_style())
                    .with_message(line.clone()),
            );
            pb.tick();
            self.write_non_tty(line);
            self.bars.push(pb);
        }
    }

    fn create_footer(&mut self) {
        let separator = self
            .render
            .theme
            .render_summary_separator(self.render.columns);
        let separator_message = if separator.is_empty() {
            " ".to_string()
        } else {
            separator
        };

        let sep_pb = self.multi.add(ProgressBar::new(0));
        sep_pb.set_style(Self::pending_style());
        sep_pb.finish_with_message(separator_message);
        self.footer_separator = Some(sep_pb);

        let footer_msg = self.render_footer_message();
        let footer_pb = self.multi.add(
            ProgressBar::new(0)
                .with_style(Self::pending_style())
                .with_message(footer_msg),
        );
        footer_pb.tick();
        self.footer_bar = Some(footer_pb);
    }

    fn render_footer_message(&self) -> String {
        format!(
            "{}{}Done {}/{}…",
            self.render.theme.left_pad_str(),
            self.render.theme.summary_prefix(),
            self.completed_steps,
            self.total_steps
        )
    }

    fn on_step_started(&mut self, id: &str) {
        let Some(i) = self.step_index(id) else {
            return;
        };
        let step = StepLine {
            status: StepStatus::Running,
            label: self.steps[i].1.clone(),
            elapsed: None,
        };
        let line = self.render.theme.render(&step, self.render.columns);
        self.bars[i].set_style(self.running_style.clone());
        self.bars[i].set_message(line);
        self.bars[i].enable_steady_tick(Duration::from_millis(SPINNER_TICK_INTERVAL_MS));
    }

    fn on_step_output(&mut self, id: &str, line: String, stderr: bool) {
        if stderr {
            self.step_stderr
                .entry(id.to_string())
                .or_default()
                .push(line);
        }
    }

    fn finish_step(
        &mut self,
        id: &str,
        status: StepStatus,
        duration_secs: f64,
        display_cmd: Option<&str>,
    ) -> Option<usize> {
        let i = self.step_index(id)?;
        self.bars[i].disable_steady_tick();
        let display = display_cmd.unwrap_or(self.steps[i].1.as_str());
        let step = StepLine {
            status,
            label: display.to_string(),
            elapsed: Some(duration_secs),
        };
        let line = self.render.theme.render(&step, self.render.columns);
        self.finish_bar(&self.bars[i], &line);

        self.completed_steps += 1;
        if let Some(ref fb) = self.footer_bar {
            let msg = self.render_footer_message();
            fb.set_message(msg);
        }

        Some(i)
    }

    fn on_step_finished(&mut self, id: &str, duration_secs: f64, display_cmd: Option<&str>) {
        self.finish_step(id, StepStatus::Succeeded, duration_secs, display_cmd);
    }

    fn on_step_skipped(&mut self, id: &str, display_cmd: Option<&str>) {
        self.finish_step(id, StepStatus::Skipped, 0.0, display_cmd);
    }

    /// Handle a step failure event: render failure line and optional error details.
    ///
    /// CQ-010: This method has 4 levels of nesting due to the combination of:
    /// - TTY vs non-TTY output paths
    /// - Error detail display toggle
    /// - Multi-line error detail rendering
    ///
    /// The `ErrorDetailRenderer` has been extracted to handle the formatting logic,
    /// but the control flow remains here because:
    ///
    /// 1. TTY path needs access to `multi` and `bars` for inline insertion
    /// 2. Non-TTY path needs direct stderr access
    /// 3. Both paths share the same error detail extraction
    ///
    /// Future refactoring could extract `render_error_details_tty()` and
    /// `render_error_details_non_tty()` helper methods if this grows.
    fn on_step_failed(
        &mut self,
        id: &str,
        duration_secs: f64,
        message: &str,
        display_cmd: Option<&str>,
    ) {
        let Some(i) = self.finish_step(id, StepStatus::Failed, duration_secs, display_cmd) else {
            return;
        };

        if !self.render.show_error_detail {
            return;
        }

        let stderr_tail = ErrorDetailRenderer::extract_stderr_tail(
            self.step_stderr
                .get(id)
                .map(|v| v.as_slice())
                .unwrap_or(&[]),
            self.render.stderr_tail_lines,
        );
        let renderer = ErrorDetailRenderer::new(self.render.theme.as_ref(), self.render.columns);
        let detail_lines = renderer.render(message, &stderr_tail);

        if self.render.is_tty {
            let mut anchor = self.bars[i].clone();
            for detail_line in &detail_lines {
                let pb = self.multi.insert_after(&anchor, ProgressBar::new(0));
                pb.set_style(Self::pending_style());
                pb.finish_with_message(detail_line.clone());
                anchor = pb;
            }
        } else {
            for detail_line in &detail_lines {
                write_stderr(Some(detail_line));
            }
        }
    }

    fn on_run_finished(&mut self, duration_secs: f64, success: bool) {
        let summary = self.format_summary(duration_secs, success);

        // If we have a footer bar from on_plan_started, finalize it in place.
        if let Some(ref fb) = self.footer_bar {
            fb.finish_with_message(summary.clone());
            self.write_non_tty(&summary);
            return;
        }

        // Fallback: no footer (e.g. no plan was started), create bars inline.
        self.render_fallback_separator();

        let summary_pb = self.multi.add(ProgressBar::new(0));
        self.finish_bar(&summary_pb, &summary);
    }

    fn format_summary(&self, duration_secs: f64, success: bool) -> String {
        if self.total_steps > 0 {
            let label = if success { "Done" } else { "Failed" };
            let elapsed = theme::format_duration(duration_secs);
            format!(
                "{}{}{} {}/{} in {}",
                self.render.theme.left_pad_str(),
                self.render.theme.summary_prefix(),
                label,
                self.completed_steps,
                self.total_steps,
                elapsed
            )
        } else {
            self.render.theme.render_summary(success, duration_secs)
        }
    }

    fn render_fallback_separator(&self) {
        let separator = self
            .render
            .theme
            .render_summary_separator(self.render.columns);

        if self.render.is_tty {
            let separator_message = if separator.is_empty() {
                " ".to_string()
            } else {
                separator
            };
            let pb = if let Some(last_bar) = self.bars.last() {
                self.multi.insert_after(last_bar, ProgressBar::new(0))
            } else {
                self.multi.add(ProgressBar::new(0))
            };
            pb.set_style(Self::pending_style());
            pb.finish_with_message(separator_message);
        } else if separator.is_empty() {
            write_stderr(None);
        } else if let Err(e) = write!(io::stderr(), "{}", separator) {
            tracing::debug!(error = %e, "stderr write failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::RunnerEvent;
    use ops_core::output::{StepLine, StepStatus};
    use ops_theme::ThemeConfig;

    /// Renders step lines with status icons and elapsed time.
    pub struct StepRenderer<'a> {
        theme: &'a dyn theme::StepLineTheme,
        columns: u16,
    }

    impl<'a> StepRenderer<'a> {
        pub fn new(theme: &'a dyn theme::StepLineTheme, columns: u16) -> Self {
            Self { theme, columns }
        }

        pub fn render(&self, status: StepStatus, label: &str, elapsed: Option<f64>) -> String {
            let step = StepLine {
                status,
                label: label.to_string(),
                elapsed,
            };
            self.theme.render(&step, self.columns)
        }
    }

    fn test_themes() -> IndexMap<String, ThemeConfig> {
        let mut themes = IndexMap::new();
        themes.insert("classic".into(), ThemeConfig::classic());
        themes.insert("compact".into(), ThemeConfig::compact());
        themes
    }

    /// DUP-004: Reduce repeated ProgressDisplay test setup.
    fn test_display(entries: &[(&str, &str)]) -> ProgressDisplay {
        test_display_with_config(config::OutputConfig::default(), entries)
    }

    fn test_display_with_config(
        output: config::OutputConfig,
        entries: &[(&str, &str)],
    ) -> ProgressDisplay {
        let display_map: HashMap<String, String> = entries
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        let custom_themes = test_themes();
        ProgressDisplay::new(&output, display_map, &custom_themes).expect("test display construct")
    }

    #[test]
    fn progress_display_handles_full_lifecycle() {
        let mut display = test_display(&[("echo_hi", "echo hi")]);

        // Non-TTY: events go to stderr (we just verify no panics and state is correct)
        display.handle_event(RunnerEvent::PlanStarted {
            command_ids: vec!["echo_hi".into()],
        });
        assert_eq!(display.steps.len(), 1);
        assert_eq!(display.step_index("echo_hi"), Some(0));
        assert_eq!(display.step_index("unknown"), None);

        // TQ-004: Verify the rendered pending step line contains the label text
        let pending_msg = display.bars[0].message();
        assert!(
            pending_msg.contains("echo hi"),
            "pending step line should contain label, got: {pending_msg}"
        );

        display.handle_event(RunnerEvent::StepStarted {
            id: "echo_hi".into(),
            display_cmd: Some("echo hi".to_string()),
        });

        display.handle_event(RunnerEvent::StepOutput {
            id: "echo_hi".into(),
            line: "some error output".to_string(),
            stderr: true,
        });
        assert_eq!(display.step_stderr["echo_hi"].len(), 1);

        display.handle_event(RunnerEvent::StepFinished {
            id: "echo_hi".into(),
            duration_secs: 0.05,
            display_cmd: Some("echo hi".to_string()),
        });

        display.handle_event(RunnerEvent::RunFinished {
            duration_secs: 0.05,
            success: true,
        });
    }

    #[test]
    fn progress_display_handles_failure_with_error_detail() {
        let mut display = test_display_with_config(
            config::OutputConfig {
                show_error_detail: true,
                ..config::OutputConfig::default()
            },
            &[("fail_cmd", "false")],
        );

        display.handle_event(RunnerEvent::PlanStarted {
            command_ids: vec!["fail_cmd".into()],
        });
        display.handle_event(RunnerEvent::StepStarted {
            id: "fail_cmd".into(),
            display_cmd: Some("false".to_string()),
        });
        display.handle_event(RunnerEvent::StepOutput {
            id: "fail_cmd".into(),
            line: "error: something went wrong".to_string(),
            stderr: true,
        });
        display.handle_event(RunnerEvent::StepFailed {
            id: "fail_cmd".into(),
            duration_secs: 0.01,
            message: "exit status: 1".to_string(),
            display_cmd: Some("false".to_string()),
        });
        display.handle_event(RunnerEvent::RunFinished {
            duration_secs: 0.01,
            success: false,
        });
        // Verify stderr was captured
        assert_eq!(display.step_stderr["fail_cmd"].len(), 1);
    }

    #[test]
    fn progress_display_render_step() {
        let display = test_display(&[]);
        let renderer = StepRenderer::new(
            display.render_config().theme.as_ref(),
            display.render_config().columns,
        );
        let line = renderer.render(StepStatus::Succeeded, "cargo build", Some(1.23));
        assert!(line.contains("cargo build"));
        assert!(line.contains("1.23s"));
    }

    #[test]
    fn emit_line_non_tty_writes_to_stderr() {
        let output = config::OutputConfig {
            columns: 80,
            ..config::OutputConfig::default()
        };
        let custom_themes = test_themes();
        let display =
            ProgressDisplay::new_with_tty_check(&output, HashMap::new(), &custom_themes, || false)
                .expect("should construct");
        assert!(!display.render.is_tty);
        display.emit_line("test line");
    }

    #[test]
    fn emit_line_handles_empty_string() {
        // Verifies no-panic on empty input edge case.
        let display = test_display(&[]);
        display.emit_line("");
    }

    #[test]
    fn step_stderr_captures_output() {
        let mut display = test_display(&[("cmd", "test cmd")]);

        display.handle_event(RunnerEvent::PlanStarted {
            command_ids: vec!["cmd".into()],
        });
        display.handle_event(RunnerEvent::StepOutput {
            id: "cmd".into(),
            line: "stderr line 1".to_string(),
            stderr: true,
        });
        display.handle_event(RunnerEvent::StepOutput {
            id: "cmd".into(),
            line: "stdout line".to_string(),
            stderr: false,
        });
        display.handle_event(RunnerEvent::StepOutput {
            id: "cmd".into(),
            line: "stderr line 2".to_string(),
            stderr: true,
        });

        let captured = display
            .step_stderr
            .get("cmd")
            .expect("should capture stderr");
        assert_eq!(captured.len(), 2);
        assert_eq!(captured[0], "stderr line 1");
        assert_eq!(captured[1], "stderr line 2");
    }

    #[test]
    fn render_config_uses_output_settings() {
        let display = test_display_with_config(
            config::OutputConfig {
                columns: 100,
                show_error_detail: false,
                theme: "compact".into(),
                stderr_tail_lines: 10,
                category_order: Vec::new(),
            },
            &[],
        );
        assert_eq!(display.render.columns, 100);
        assert!(!display.render.show_error_detail);
        assert_eq!(display.render.stderr_tail_lines, 10);
    }

    #[test]
    fn progress_display_handles_step_skipped() {
        let mut display = test_display(&[("skip_cmd", "skipped command")]);

        display.handle_event(RunnerEvent::PlanStarted {
            command_ids: vec!["skip_cmd".into()],
        });
        display.handle_event(RunnerEvent::StepStarted {
            id: "skip_cmd".into(),
            display_cmd: Some("skipped command".to_string()),
        });
        display.handle_event(RunnerEvent::StepSkipped {
            id: "skip_cmd".into(),
            display_cmd: Some("skipped command".to_string()),
        });
        display.handle_event(RunnerEvent::RunFinished {
            duration_secs: 0.0,
            success: true,
        });

        assert!(display.bars.len() == 1);
    }

    mod edge_case_tests {
        use super::*;
        use crate::command::RunnerEvent;
        use ops_core::output::StepStatus;

        #[test]
        fn extract_stderr_tail_extracts_correct_count() {
            let lines: Vec<String> = (1..=10).map(|i| format!("line {}", i)).collect();
            let tail = ErrorDetailRenderer::extract_stderr_tail(&lines, DEFAULT_STDERR_TAIL_LINES);
            assert_eq!(tail.len(), DEFAULT_STDERR_TAIL_LINES);
            assert_eq!(tail[0], "line 6");
            assert_eq!(tail[4], "line 10");
        }

        #[test]
        fn extract_stderr_tail_handles_fewer_lines() {
            let lines: Vec<String> = vec!["a".into(), "b".into()];
            let tail = ErrorDetailRenderer::extract_stderr_tail(&lines, DEFAULT_STDERR_TAIL_LINES);
            assert_eq!(tail.len(), 2);
            assert_eq!(tail[0], "a");
            assert_eq!(tail[1], "b");
        }

        #[test]
        fn extract_stderr_tail_handles_empty() {
            let lines: Vec<String> = vec![];
            let tail = ErrorDetailRenderer::extract_stderr_tail(&lines, DEFAULT_STDERR_TAIL_LINES);
            assert!(tail.is_empty());
        }

        #[test]
        fn extract_stderr_tail_unlimited_returns_all() {
            let lines: Vec<String> = (1..=100).map(|i| format!("line {}", i)).collect();
            let tail = ErrorDetailRenderer::extract_stderr_tail(&lines, usize::MAX);
            assert_eq!(tail.len(), 100);
        }

        #[test]
        fn finish_step_returns_none_for_unknown_id() {
            let mut display = test_display(&[]);

            display.handle_event(RunnerEvent::PlanStarted {
                command_ids: vec!["known".into()],
            });

            let result = display.finish_step("unknown", StepStatus::Succeeded, 1.0, None);
            assert!(result.is_none(), "unknown step should return None");
        }

        #[test]
        fn write_stderr_handles_none_and_some() {
            // Verifies no-panic on both None and Some inputs.
            write_stderr(None);
            write_stderr(Some("test line"));
        }
    }

    /// TQ-018: Test rapid concurrent event sequences don't cause panics or race conditions.
    mod concurrent_event_tests {
        use super::*;

        #[test]
        fn handle_event_rapid_sequence_no_panic() {
            let mut display = test_display(&[("cmd1", "echo 1"), ("cmd2", "echo 2")]);

            // Simulate rapid event sequence as would occur in parallel execution
            let events = vec![
                RunnerEvent::PlanStarted {
                    command_ids: vec!["cmd1".into(), "cmd2".into()],
                },
                RunnerEvent::StepStarted {
                    id: "cmd1".into(),
                    display_cmd: Some("echo 1".into()),
                },
                RunnerEvent::StepStarted {
                    id: "cmd2".into(),
                    display_cmd: Some("echo 2".into()),
                },
                RunnerEvent::StepOutput {
                    id: "cmd1".into(),
                    line: "output1".into(),
                    stderr: false,
                },
                RunnerEvent::StepOutput {
                    id: "cmd2".into(),
                    line: "output2".into(),
                    stderr: true,
                },
                RunnerEvent::StepFinished {
                    id: "cmd1".into(),
                    duration_secs: 0.1,
                    display_cmd: Some("echo 1".into()),
                },
                RunnerEvent::StepFinished {
                    id: "cmd2".into(),
                    duration_secs: 0.15,
                    display_cmd: Some("echo 2".into()),
                },
                RunnerEvent::RunFinished {
                    duration_secs: 0.2,
                    success: true,
                },
            ];

            // Handle all events - should not panic
            for event in events {
                display.handle_event(event);
            }

            // Verify state is consistent
            assert_eq!(display.steps.len(), 2);
            assert_eq!(display.bars.len(), 2);
        }

        #[test]
        fn handle_event_interleaved_failure_sequence() {
            let mut display = test_display_with_config(
                config::OutputConfig {
                    show_error_detail: true,
                    ..config::OutputConfig::default()
                },
                &[("ok", "true"), ("fail", "false")],
            );

            // Simulate parallel execution with one failure
            let events = vec![
                RunnerEvent::PlanStarted {
                    command_ids: vec!["ok".into(), "fail".into()],
                },
                RunnerEvent::StepStarted {
                    id: "ok".into(),
                    display_cmd: Some("true".into()),
                },
                RunnerEvent::StepStarted {
                    id: "fail".into(),
                    display_cmd: Some("false".into()),
                },
                RunnerEvent::StepOutput {
                    id: "fail".into(),
                    line: "error message".into(),
                    stderr: true,
                },
                RunnerEvent::StepFinished {
                    id: "ok".into(),
                    duration_secs: 0.01,
                    display_cmd: Some("true".into()),
                },
                RunnerEvent::StepFailed {
                    id: "fail".into(),
                    duration_secs: 0.01,
                    message: "exit status 1".into(),
                    display_cmd: Some("false".into()),
                },
                RunnerEvent::RunFinished {
                    duration_secs: 0.02,
                    success: false,
                },
            ];

            for event in events {
                display.handle_event(event);
            }

            // Verify stderr was captured for failed command
            assert!(display.step_stderr.contains_key("fail"));
        }
    }

    /// TQ-005: Test ProgressDisplay error handling for invalid theme/template.
    mod error_path_tests {
        use super::*;

        #[test]
        fn progress_display_invalid_theme_returns_error() {
            let output = config::OutputConfig {
                theme: "nonexistent_theme".into(),
                ..config::OutputConfig::default()
            };
            let custom_themes = IndexMap::new();
            let result = ProgressDisplay::new(&output, HashMap::new(), &custom_themes);

            match result {
                Err(e) => {
                    let err = e.to_string();
                    assert!(
                        err.contains("Theme not found") || err.contains("nonexistent_theme"),
                        "error should mention theme issue: {err}"
                    );
                }
                Ok(_) => panic!("should fail for nonexistent theme"),
            }
        }

        #[test]
        fn progress_display_valid_theme_succeeds() {
            let output = config::OutputConfig {
                theme: "classic".into(),
                ..config::OutputConfig::default()
            };
            // test_display_with_config uses test_themes() which includes "classic"
            let _display = test_display_with_config(output, &[]);
        }
    }

    /// TQ-013: Test handle_event with unknown command IDs.
    mod unknown_command_tests {
        use super::*;

        #[test]
        fn handle_event_unknown_command_id_no_panic() {
            let mut display = test_display(&[]);

            display.handle_event(RunnerEvent::PlanStarted {
                command_ids: vec!["known_cmd".into()],
            });

            display.handle_event(RunnerEvent::StepStarted {
                id: "unknown_cmd".into(),
                display_cmd: Some("unknown command".to_string()),
            });

            display.handle_event(RunnerEvent::StepFinished {
                id: "unknown_cmd".into(),
                duration_secs: 0.1,
                display_cmd: Some("unknown command".to_string()),
            });

            display.handle_event(RunnerEvent::RunFinished {
                duration_secs: 0.1,
                success: true,
            });
        }

        #[test]
        fn handle_event_step_output_for_unknown_command_no_panic() {
            let mut display = test_display(&[]);

            display.handle_event(RunnerEvent::PlanStarted {
                command_ids: vec!["cmd1".into()],
            });

            display.handle_event(RunnerEvent::StepOutput {
                id: "non_existent_cmd".into(),
                line: "some output".to_string(),
                stderr: true,
            });

            assert!(
                display.step_stderr.contains_key("non_existent_cmd"),
                "output for unknown command should be stored under its ID"
            );
        }

        /// TQ-012: finish_step with unknown step ID returns None.
        #[test]
        fn finish_step_unknown_id_returns_none() {
            let mut display = test_display(&[]);

            display.handle_event(RunnerEvent::PlanStarted {
                command_ids: vec!["known".into()],
            });

            // finish_step is called internally via on_step_finished -- trigger it with unknown ID
            display.handle_event(RunnerEvent::StepFinished {
                id: "never_registered".into(),
                duration_secs: 1.0,
                display_cmd: None,
            });
            // No panic means finish_step correctly returned None and was handled
        }

        #[test]
        fn handle_event_step_failed_for_unknown_command_no_panic() {
            let mut display = test_display_with_config(
                config::OutputConfig {
                    show_error_detail: true,
                    ..config::OutputConfig::default()
                },
                &[],
            );

            display.handle_event(RunnerEvent::PlanStarted {
                command_ids: vec!["known".into()],
            });

            display.handle_event(RunnerEvent::StepFailed {
                id: "unknown_failed".into(),
                duration_secs: 0.1,
                message: "exit status 1".to_string(),
                display_cmd: Some("unknown failed cmd".to_string()),
            });

            display.handle_event(RunnerEvent::RunFinished {
                duration_secs: 0.1,
                success: false,
            });
        }
    }
}
