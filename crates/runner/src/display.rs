//! Progress display: step-line rendering, progress bars, and CLI event handling.
//!
//! The module is split for cohesion:
//! - [`error_detail`] — error block rendering
//! - [`render_config`] — render config + constructor options
//! - [`style`] — progress-bar style construction

mod error_detail;
mod render_config;
mod style;
#[cfg(test)]
mod tests;

pub use error_detail::ErrorDetailRenderer;
pub use render_config::{DisplayOptions, RenderConfig};

use crate::command::RunnerEvent;
use ops_core::config::CommandId;
use ops_core::output::{StepLine, StepStatus};
use ops_theme::{self as theme, BoxSnapshot};

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use style::{build_running_style, pending_style, SPINNER_TICK_INTERVAL_MS};

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
/// The non-test module body is already past the original 500-line guideline
/// (see `wc -l` on this file). The extraction below is a live candidate, not
/// a hypothetical one — pick it up when the next non-trivial change lands here
/// rather than piling onto the current surface:
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
    header_bar: Option<ProgressBar>,
    completed_steps: usize,
    failed_steps: usize,
    total_steps: usize,
    plan_command_ids: Vec<String>,
    run_started_at: Option<Instant>,
    tap_file: Option<File>,
}

impl ProgressDisplay {
    fn is_stderr_tty() -> bool {
        std::io::stderr().is_terminal()
    }

    fn open_tap_file(path: PathBuf) -> Option<File> {
        match File::create(&path) {
            Ok(f) => Some(f),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "failed to open tap file");
                None
            }
        }
    }

    pub fn new(opts: DisplayOptions<'_>) -> anyhow::Result<Self> {
        Self::new_with_tty_check(opts, Self::is_stderr_tty)
    }

    fn new_with_tty_check<F>(opts: DisplayOptions<'_>, is_tty_fn: F) -> anyhow::Result<Self>
    where
        F: FnOnce() -> bool,
    {
        let DisplayOptions {
            output,
            display_map,
            custom_themes,
            tap,
        } = opts;
        let is_tty = is_tty_fn();
        let multi = MultiProgress::with_draw_target(if is_tty {
            ProgressDrawTarget::stderr()
        } else {
            ProgressDrawTarget::hidden()
        });
        let resolved_theme = theme::resolve_theme(&output.theme, custom_themes)?;
        let running_style = build_running_style(resolved_theme.as_ref(), &output.theme)?;
        let tap_file = tap.and_then(Self::open_tap_file);

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
            header_bar: None,
            completed_steps: 0,
            failed_steps: 0,
            total_steps: 0,
            plan_command_ids: Vec::new(),
            run_started_at: None,
            tap_file,
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
        bar.set_style(pending_style());
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
        self.steps = command_ids
            .iter()
            .map(|id| self.resolve_step_display(id))
            .collect();

        self.total_steps = command_ids.len();
        self.plan_command_ids = command_ids.iter().map(|id| id.to_string()).collect();
        self.completed_steps = 0;
        self.failed_steps = 0;
        self.run_started_at = Some(Instant::now());

        let is_boxed = self
            .render
            .theme
            .box_top_border(
                BoxSnapshot::new(0, self.total_steps, 0.0, true, self.render.columns)
                    .with_command_ids(&self.plan_command_ids),
            )
            .is_some();

        if !is_boxed {
            let header_lines = self.render.theme.render_plan_header(&self.plan_command_ids);
            for line in &header_lines {
                self.emit_line(line);
            }
        }

        // For boxed layout, emit the top border as a live-updating header bar first,
        // so subsequent step bars appear below it.
        if is_boxed {
            self.create_header_bar();
        }

        self.create_pending_bars();

        self.create_footer();
    }

    fn create_header_bar(&mut self) {
        let msg = self.render_header_message();
        let pb = self.multi.add(
            ProgressBar::new(0)
                .with_style(pending_style())
                .with_message(msg.clone()),
        );
        pb.tick();
        self.write_non_tty(&msg);
        self.header_bar = Some(pb);
    }

    /// Build a [`BoxSnapshot`] describing the plan's current live state.
    /// Centralized so header/footer live-borders share one source of truth
    /// for `completed_steps`, `failed_steps`, and `elapsed_secs`.
    fn live_box_snapshot(&self) -> BoxSnapshot<'_> {
        let elapsed = self
            .run_started_at
            .map(|t| t.elapsed().as_secs_f64())
            .unwrap_or(0.0);
        let success_so_far = self.failed_steps == 0;
        BoxSnapshot::new(
            self.completed_steps,
            self.total_steps,
            elapsed,
            success_so_far,
            self.render.columns,
        )
        .with_command_ids(&self.plan_command_ids)
    }

    fn render_header_message(&self) -> String {
        self.render
            .theme
            .box_top_border(self.live_box_snapshot())
            .unwrap_or_default()
    }

    /// Compute the vertical-progress cell glyph for a step in `status`.
    /// `█` = done (succeeded/failed/skipped), `▓` = running, `░` = pending.
    ///
    /// Driven by the step's own status rather than row vs. completed-count, so
    /// parallel plans where steps finish out of order still show the right
    /// glyph per row.
    fn progress_cell(status: StepStatus) -> &'static str {
        match status {
            StepStatus::Pending => "░",
            StepStatus::Running => "▓",
            StepStatus::Succeeded | StepStatus::Failed | StepStatus::Skipped => "█",
        }
    }

    /// Render a step with the theme, reserving columns for the box frame and
    /// wrapping the result for boxed layouts.
    fn render_and_wrap_step(&self, step: &StepLine) -> String {
        let reserve = self.render.theme.step_column_reserve();
        let effective = self.render.columns.saturating_sub(reserve);
        let inner = self.render.theme.render(step, effective);
        let cell = Self::progress_cell(step.status);
        self.render
            .theme
            .wrap_step_line(&inner, cell, self.render.columns)
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
                self.render_and_wrap_step(&step)
            })
            .collect();

        self.bars.clear();
        for line in &pending_lines {
            let pb = self.multi.add(
                ProgressBar::new_spinner()
                    .with_style(pending_style())
                    .with_message(line.clone()),
            );
            pb.tick();
            self.write_non_tty(line);
            self.bars.push(pb);
        }
    }

    fn create_footer(&mut self) {
        let is_boxed = self.header_bar.is_some();
        if is_boxed {
            // Boxed layout: footer bar carries a live bottom border that updates
            // with completion count and elapsed as the plan runs; RunFinished
            // locks it to the final `Done/Failed N/N in …` line.
            let footer_pb = self.multi.add(
                ProgressBar::new(0)
                    .with_style(pending_style())
                    .with_message(self.render_footer_message()),
            );
            footer_pb.tick();
            self.footer_bar = Some(footer_pb);
            return;
        }

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
        sep_pb.set_style(pending_style());
        sep_pb.finish_with_message(separator_message);
        self.footer_separator = Some(sep_pb);

        let footer_msg = self.render_footer_message();
        let footer_pb = self.multi.add(
            ProgressBar::new(0)
                .with_style(pending_style())
                .with_message(footer_msg),
        );
        footer_pb.tick();
        self.footer_bar = Some(footer_pb);
    }

    fn render_footer_message(&self) -> String {
        // Boxed layout: render a live bottom border so the frame stays closed
        // while the plan is still running (mirrors the live top border).
        if self.header_bar.is_some() {
            if let Some(bottom) = self
                .render
                .theme
                .box_bottom_border(self.live_box_snapshot())
            {
                return bottom;
            }
        }
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
        // Running rows: the running_template owns the full line (left chrome,
        // spinner, elapsed, right chrome). `running_template_overhead` already
        // reserves width for every fixed part, so we pass the full terminal
        // `columns` — subtracting `step_column_reserve` would double-count
        // chrome that the template itself emits and make the row under-fill.
        let line = self.render.theme.render(&step, self.render.columns);
        self.bars[i].set_style(self.running_style.clone());
        self.bars[i].set_message(line);
        self.bars[i].enable_steady_tick(Duration::from_millis(SPINNER_TICK_INTERVAL_MS));
        self.refresh_header_bar();
        self.refresh_footer_bar();
    }

    fn refresh_header_bar(&self) {
        if let Some(ref hb) = self.header_bar {
            hb.set_message(self.render_header_message());
        }
    }

    fn refresh_footer_bar(&self) {
        // Only refreshes for boxed layout — flat themes use the Done N/M… text
        // set per step in finish_step.
        if self.header_bar.is_some() {
            if let Some(ref fb) = self.footer_bar {
                fb.set_message(self.render_footer_message());
            }
        }
    }

    fn tap_line(&mut self, line: &str) {
        // ERR-1: previously dropped the writeln Result silently; a broken tap
        // fd (disk full, NFS drop, closed underneath) would swallow every
        // subsequent line without the user seeing any diagnostic. On first
        // failure log once at debug level and drop the handle so we stop
        // trying — subsequent lines then no-op rather than spamming debug.
        if let Some(ref mut f) = self.tap_file {
            if let Err(e) = writeln!(f, "{}", line) {
                tracing::debug!(error = %e, "tap file write failed; disabling further tap writes");
                self.tap_file = None;
            }
        }
    }

    fn on_step_output(&mut self, id: &str, line: String, stderr: bool) {
        if stderr {
            self.step_stderr
                .entry(id.to_string())
                .or_default()
                .push(line.clone());
        }
        self.tap_line(&line);
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
        // Count the step as complete before rendering so its row's progress
        // cell shows the "done" glyph (█) instead of the "current" glyph (▓).
        self.completed_steps += 1;
        if matches!(status, StepStatus::Failed) {
            self.failed_steps += 1;
        }
        let line = self.render_and_wrap_step(&step);
        self.finish_bar(&self.bars[i], &line);
        if let Some(ref fb) = self.footer_bar {
            let msg = self.render_footer_message();
            fb.set_message(msg);
        }
        self.refresh_header_bar();

        Some(i)
    }

    fn on_step_finished(&mut self, id: &str, duration_secs: f64, display_cmd: Option<&str>) {
        self.finish_step(id, StepStatus::Succeeded, duration_secs, display_cmd);
    }

    fn on_step_skipped(&mut self, id: &str, display_cmd: Option<&str>) {
        self.finish_step(id, StepStatus::Skipped, 0.0, display_cmd);
    }

    fn render_error_details_tty(&mut self, bar_index: usize, detail_lines: &[String]) {
        let mut anchor = self.bars[bar_index].clone();
        for detail_line in detail_lines {
            let pb = self.multi.insert_after(&anchor, ProgressBar::new(0));
            pb.set_style(pending_style());
            pb.finish_with_message(detail_line.clone());
            anchor = pb;
        }
    }

    fn render_error_details_non_tty(detail_lines: &[String]) {
        for detail_line in detail_lines {
            write_stderr(Some(detail_line));
        }
    }

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
            self.render_error_details_tty(i, &detail_lines);
        } else {
            Self::render_error_details_non_tty(&detail_lines);
        }
    }

    fn on_run_finished(&mut self, duration_secs: f64, success: bool) {
        // Boxed layout: finalize header bar to "Done" state and emit bottom border.
        if let Some(bottom) = self.render.theme.box_bottom_border(BoxSnapshot::new(
            self.completed_steps,
            self.total_steps,
            duration_secs,
            success,
            self.render.columns,
        )) {
            if let Some(ref hb) = self.header_bar {
                hb.finish_with_message(self.render_header_message());
            }
            if let Some(ref fb) = self.footer_bar {
                fb.finish_with_message(bottom.clone());
                self.write_non_tty(&bottom);
            } else {
                let pb = self.multi.add(ProgressBar::new(0));
                self.finish_bar(&pb, &bottom);
            }
            return;
        }

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
            let body = format!(
                "{} {}/{} in {}",
                label, self.completed_steps, self.total_steps, elapsed
            );
            let colored = theme::apply_style(&body, self.render.theme.summary_color());
            format!(
                "{}{}{}",
                self.render.theme.left_pad_str(),
                self.render.theme.summary_prefix(),
                colored
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
            pb.set_style(pending_style());
            pb.finish_with_message(separator_message);
        } else if separator.is_empty() {
            write_stderr(None);
        } else if let Err(e) = write!(io::stderr(), "{}", separator) {
            tracing::debug!(error = %e, "stderr write failed");
        }
    }
}
