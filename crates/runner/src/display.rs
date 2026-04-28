//! Progress display: step-line rendering, progress bars, and CLI event handling.
//!
//! The module is split for cohesion:
//! - [`error_detail`] — error block rendering
//! - [`render_config`] — render config + constructor options
//! - [`style`] — progress-bar style construction
//! - [`progress_state`] — per-plan step bookkeeping (bars, steps, captured stderr)

mod error_detail;
mod progress_state;
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
use progress_state::ProgressState;
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
/// # Architecture (ARCH-1 / TASK-0332)
///
/// Per-plan step bookkeeping (`bars`, `steps`, `step_stderr`, `display_map`,
/// `plan_command_ids`) lives in [`ProgressState`]; this struct retains the
/// rendering config, the `MultiProgress` handle, the running plan's
/// header/footer bars, the run-scoped counters, and the tap file. This split
/// keeps the per-step row state cohesive and shrinks `ProgressDisplay` to
/// the surface that depends on `RenderConfig` + the indicatif lifecycle.
///
/// `ErrorDetailRenderer` is extracted into its own submodule for the same
/// reason — error rendering depends only on theme + columns, not on
/// progress state.
pub struct ProgressDisplay {
    render: RenderConfig,
    multi: MultiProgress,
    pub(crate) state: ProgressState,
    // PERF-3: `running_style` is cloned per progress bar because `indicatif::ProgressBar::with_style`
    // takes ownership. Acceptable for typical step counts (<20 commands).
    // `pending_style` was removed — it's trivially reconstructed via `pending_style()`.
    running_style: ProgressStyle,
    footer_separator: Option<ProgressBar>,
    footer_bar: Option<ProgressBar>,
    header_bar: Option<ProgressBar>,
    pub(crate) completed_steps: usize,
    failed_steps: usize,
    total_steps: usize,
    run_started_at: Option<Instant>,
    tap_file: Option<File>,
    /// ERR-2 / TASK-0458: retained so that on a write error we can attempt
    /// to reopen the tap and append a final "truncated" marker line in
    /// addition to the stderr warning.
    tap_path: Option<PathBuf>,
    /// ERR-2 / TASK-0458: when `tap_file` is dropped due to a write error,
    /// remember the error kind and the step id that triggered it so
    /// RunFinished can emit a single user-visible warning ("tap file
    /// truncated after step X due to <kind>") instead of leaving the tap
    /// silently truncated.
    tap_truncation: Option<(String, String)>,
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
        let tap_path = tap.clone();
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
            state: ProgressState::new(display_map),
            running_style,
            footer_separator: None,
            footer_bar: None,
            header_bar: None,
            completed_steps: 0,
            failed_steps: 0,
            total_steps: 0,
            run_started_at: None,
            tap_file,
            tap_path,
            tap_truncation: None,
        })
    }

    /// Returns a reference to the render config for testing.
    #[cfg(test)]
    pub fn render_config(&self) -> &RenderConfig {
        &self.render
    }

    fn step_index(&self, id: &str) -> Option<usize> {
        self.state.step_index(id)
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
    /// # Async safety invariant (CONC-5 / TASK-0331)
    ///
    /// `handle_event` performs **blocking** I/O on two paths:
    ///
    /// 1. `tap_line` calls `writeln!` on a `std::fs::File`. Under a chatty
    ///    command (e.g. `cargo build` emitting thousands of stderr lines),
    ///    each line incurs a synchronous `write(2)`. On NFS or a slow
    ///    fsync-heavy filesystem this can stall.
    /// 2. `emit_line` and `write_stderr` write to stderr (also sync).
    ///
    /// **This method must never be polled from inside a tokio async task.**
    /// Today it is consumed exclusively from the dedicated event-pump loop in
    /// the CLI (synchronous draining of `mpsc::Receiver<RunnerEvent>`), so
    /// the blocking writes do not stall the runtime. If a future refactor
    /// moves the consumer into `tokio::spawn`, switch the tap file to a
    /// buffered/async writer fed via an mpsc channel and a dedicated writer
    /// task before doing so — otherwise a single noisy command will starve
    /// every other task on the same worker.
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
            RunnerEvent::StepOutputDropped { id, dropped_count } => {
                // CONC-7 / TASK-0457: render to stderr and the tap so the
                // operator sees that some output lines were lost under
                // backpressure instead of inferring it from a step
                // failure with no visible cause.
                let line = format!("[ops] {id}: {dropped_count} output line(s) dropped under load");
                tracing::warn!(target: "ops::runner", "{line}");
                self.emit_line(&line);
                self.tap_line_for(&line, Some(id.as_str()));
            }
        }
    }

    fn on_plan_started(&mut self, command_ids: &[CommandId]) {
        self.state.reset_for_plan(command_ids);

        self.total_steps = command_ids.len();
        self.completed_steps = 0;
        self.failed_steps = 0;
        self.run_started_at = Some(Instant::now());

        let is_boxed = self
            .render
            .theme
            .box_top_border(BoxSnapshot {
                completed: 0,
                total: self.total_steps,
                elapsed_secs: 0.0,
                success: true,
                columns: self.render.columns,
                command_ids: &self.state.plan_command_ids,
            })
            .is_some();

        if !is_boxed {
            let header_lines = self
                .render
                .theme
                .render_plan_header(&self.state.plan_command_ids);
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
        BoxSnapshot {
            completed: self.completed_steps,
            total: self.total_steps,
            elapsed_secs: elapsed,
            success: success_so_far,
            columns: self.render.columns,
            command_ids: &self.state.plan_command_ids,
        }
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
            // API-9: StepStatus is #[non_exhaustive]; future variants
            // render as a "done" glyph rather than break the build.
            _ => "█",
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
            .state
            .steps
            .iter()
            .map(|(_, display)| {
                let step = StepLine::new(StepStatus::Pending, display.clone(), None);
                self.render_and_wrap_step(&step)
            })
            .collect();

        self.state.bars.clear();
        for line in &pending_lines {
            let pb = self.multi.add(
                ProgressBar::new_spinner()
                    .with_style(pending_style())
                    .with_message(line.clone()),
            );
            pb.tick();
            self.write_non_tty(line);
            self.state.bars.push(pb);
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
        let step = StepLine::new(StepStatus::Running, self.state.steps[i].1.clone(), None);
        // Running rows: the running_template owns the full line (left chrome,
        // spinner, elapsed, right chrome). `running_template_overhead` already
        // reserves width for every fixed part, so we pass the full terminal
        // `columns` — subtracting `step_column_reserve` would double-count
        // chrome that the template itself emits and make the row under-fill.
        let line = self.render.theme.render(&step, self.render.columns);
        self.state.bars[i].set_style(self.running_style.clone());
        self.state.bars[i].set_message(line);
        self.state.bars[i].enable_steady_tick(Duration::from_millis(SPINNER_TICK_INTERVAL_MS));
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

    fn tap_line_for(&mut self, line: &str, step_id: Option<&str>) {
        // ERR-1: previously dropped the writeln Result silently; a broken tap
        // fd (disk full, NFS drop, closed underneath) would swallow every
        // subsequent line without the user seeing any diagnostic.
        // ERR-2 / TASK-0458: on first failure capture the kind + step id so
        // RunFinished can emit a user-visible "tap truncated" line. We do
        // not retry: the inner File is `std::fs::File` whose write
        // interface does not surface EAGAIN distinctly, and a retry-once
        // strategy is documented as optional in the task. Subsequent lines
        // no-op rather than spamming.
        if let Some(ref mut f) = self.tap_file {
            if let Err(e) = writeln!(f, "{}", line) {
                tracing::debug!(error = %e, "tap file write failed; disabling further tap writes");
                self.tap_truncation = Some((
                    step_id.unwrap_or("<unknown>").to_string(),
                    e.kind().to_string(),
                ));
                self.tap_file = None;
            }
        }
    }

    fn on_step_output(&mut self, id: &str, line: String, stderr: bool) {
        if stderr {
            self.state.record_stderr(id, line.clone());
        }
        self.tap_line_for(&line, Some(id));
    }

    fn finish_step(
        &mut self,
        id: &str,
        status: StepStatus,
        duration_secs: f64,
        display_cmd: Option<&str>,
    ) -> Option<usize> {
        let i = self.step_index(id)?;
        self.state.bars[i].disable_steady_tick();
        let display = display_cmd.unwrap_or(self.state.steps[i].1.as_str());
        let step = StepLine::new(status, display.to_string(), Some(duration_secs));
        // Count the step as complete before rendering so its row's progress
        // cell shows the "done" glyph (█) instead of the "current" glyph (▓).
        self.completed_steps += 1;
        if matches!(status, StepStatus::Failed) {
            self.failed_steps += 1;
        }
        let line = self.render_and_wrap_step(&step);
        self.finish_bar(&self.state.bars[i], &line);
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
        // READ-5 / TASK-0337: read the bar's own elapsed timer before
        // `finish_step` disables its steady tick. Hard-coding 0.0 hid how
        // much CPU actually went to a cancelled task.
        let elapsed = self
            .step_index(id)
            .map(|i| self.state.bars[i].elapsed().as_secs_f64())
            .unwrap_or(0.0);
        self.finish_step(id, StepStatus::Skipped, elapsed, display_cmd);
    }

    fn render_error_details_tty(&mut self, bar_index: usize, detail_lines: &[String]) {
        let mut anchor = self.state.bars[bar_index].clone();
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
            self.state
                .step_stderr
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

    /// Finalize any step bars still in the "running" state. Without this, a bar
    /// whose task was aborted mid-flight (e.g. `fail_fast` cancellation) never
    /// receives a `StepFinished/Failed/Skipped` event, so its row gets dropped
    /// from the multi-progress draw on the next redraw — leaving a hole in the
    /// boxed frame and a visible row count that disagrees with `Done N/M`.
    ///
    /// Each finalized orphan also bumps `completed_steps` (ERR-1 / TASK-0333):
    /// without that, the footer shows `Done 1/3` while three rows are visibly
    /// finished — defeating the very disagreement this routine was added to
    /// fix.
    fn finalize_orphan_bars(&mut self) {
        for i in 0..self.state.bars.len() {
            if self.state.bars[i].is_finished() {
                continue;
            }
            self.state.bars[i].disable_steady_tick();
            let elapsed = self.state.bars[i].elapsed().as_secs_f64();
            let step = StepLine::new(
                StepStatus::Skipped,
                self.state.steps[i].1.clone(),
                Some(elapsed),
            );
            self.completed_steps += 1;
            let line = self.render_and_wrap_step(&step);
            self.finish_bar(&self.state.bars[i], &line);
        }
    }

    fn on_run_finished(&mut self, duration_secs: f64, success: bool) {
        self.finalize_orphan_bars();
        // ERR-2 / TASK-0458: surface tap-file truncation if we hit a write
        // error mid-run. Emitted exactly once per run to both stderr (for
        // the user) and the tap file itself (for downstream test harnesses
        // that scan the tap), so a partial tap is never silently treated
        // as "no failures".
        if let Some((step_id, kind)) = self.tap_truncation.take() {
            let line = format!("[ops] tap file truncated after step {step_id} due to: {kind}");
            tracing::warn!(target: "ops::tap", "{}", line);
            write_stderr(Some(&line));
            // Best-effort: try to append the marker as the last tap line
            // so a downstream parser that only inspects the file (no
            // stderr capture) still sees the truncation. If this open
            // also fails, the stderr warning above is still visible.
            if let Some(path) = self.tap_path.as_ref() {
                if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(path) {
                    let _ = writeln!(f, "{line}");
                }
            }
        }
        // Boxed layout: finalize header bar to "Done" state and emit bottom border.
        if let Some(bottom) = self.render.theme.box_bottom_border(BoxSnapshot {
            completed: self.completed_steps,
            total: self.total_steps,
            elapsed_secs: duration_secs,
            success,
            columns: self.render.columns,
            command_ids: &self.state.plan_command_ids,
        }) {
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
            let pb = if let Some(last_bar) = self.state.bars.last() {
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
