//! Progress display: step-line rendering, progress bars, and CLI event handling.
//!
//! The module is split for cohesion:
//! - [`error_detail`] — error block rendering
//! - [`render_config`] — render config + constructor options
//! - [`style`] — progress-bar style construction
//! - [`progress_state`] — per-plan step bookkeeping (bars, steps, captured stderr)

mod error_detail;
mod finalize;
mod progress_state;
mod render_config;
mod style;
mod tap;
#[cfg(test)]
mod tests;

pub use error_detail::ErrorDetailRenderer;
pub use render_config::{DisplayOptions, RenderConfig, StderrTail};

use crate::command::RunnerEvent;
use ops_core::config::CommandId;
use ops_core::output::{StepLine, StepStatus};
use ops_theme::{self as theme, BoxSnapshot};

use indicatif::{MultiProgress, ProgressBar, ProgressDrawTarget, ProgressStyle};
use progress_state::ProgressState;
use std::io::{self, IsTerminal, Write};
use std::marker::PhantomData;
use std::time::{Duration, Instant};
use tap::TapWriter;

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
pub(super) fn write_stderr(line: Option<&str>) {
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
/// Per-plan step bookkeeping lives in [`ProgressState`] (ARCH-1 / TASK-0332);
/// tap-file lifecycle lives in [`tap::TapWriter`] (ARCH-1 / TASK-0581);
/// error rendering lives in [`ErrorDetailRenderer`]. This struct retains the
/// rendering config, the `MultiProgress` handle, the header/footer bars, and
/// the run-scoped counters — i.e. only what depends on `RenderConfig` + the
/// indicatif lifecycle.
pub struct ProgressDisplay {
    pub(super) render: RenderConfig,
    pub(super) multi: MultiProgress,
    pub(crate) state: ProgressState,
    // PERF-3: `running_style` is cloned per progress bar because `indicatif::ProgressBar::with_style`
    // takes ownership. Acceptable for typical step counts (<20 commands).
    running_style: ProgressStyle,
    footer_separator: Option<ProgressBar>,
    pub(super) footer_bar: Option<ProgressBar>,
    pub(super) header_bar: Option<ProgressBar>,
    /// CL-3 / TASK-0771: counts every step that reached a terminal state —
    /// succeeded, failed, or skipped. The legacy name "completed" is
    /// retained for backwards compatibility with `BoxSnapshot.completed`,
    /// but the semantics are *terminal-step count*, not success count.
    /// `failed_steps` and `skipped_steps` carry the breakdown.
    pub(crate) completed_steps: usize,
    pub(super) failed_steps: usize,
    pub(super) skipped_steps: usize,
    pub(super) total_steps: usize,
    run_started_at: Option<Instant>,
    /// ARCH-1 (TASK-0581): tap-file lifecycle (open handle, original path
    /// for re-opening on failure, captured truncation kind) extracted into
    /// `tap::TapWriter`. `None` here means no tap was requested at all.
    pub(super) tap: Option<TapWriter>,
    /// CL-3 / TASK-0656 + TRAIT-9 / TASK-0907: structurally enforce the
    /// sync-IO invariant on [`Self::handle_event`].
    ///
    /// **Why this is a free-standing marker, not tied to a field:** every
    /// real field on `ProgressDisplay` is `Send + Sync` today (the tap
    /// uses `std::fs::File`, the indicatif handles are `Send`, etc.).
    /// The `!Send` constraint exists *only* to keep `handle_event` —
    /// which performs blocking sync stderr / tap writes — off
    /// multi-thread tokio worker threads. There is therefore no field to
    /// "tie" the marker to; if someone routes the tap through an async
    /// writer in the future, both the writer change and this marker
    /// removal need to land together.
    ///
    /// The `static_assert_not_send` block in [`tests`] pins the
    /// invariant: removing this marker fails compilation of the test
    /// module so the regression cannot land silently.
    _not_send: PhantomData<*const ()>,
}

impl ProgressDisplay {
    fn is_stderr_tty() -> bool {
        std::io::stderr().is_terminal()
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
            verbose,
        } = opts;
        let is_tty = is_tty_fn();
        let multi = MultiProgress::with_draw_target(if is_tty {
            ProgressDrawTarget::stderr()
        } else {
            ProgressDrawTarget::hidden()
        });
        let resolved_theme = theme::resolve_theme(&output.theme, custom_themes)?;
        let running_style = build_running_style(&resolved_theme, &output.theme)?;
        let tap = tap.map(TapWriter::new);
        // TASK-0762: verbose → unbounded; otherwise use the user's config value.
        let stderr_tail = if verbose {
            StderrTail::Unbounded
        } else {
            StderrTail::Limited(output.stderr_tail_lines)
        };

        Ok(Self {
            render: RenderConfig {
                theme: resolved_theme,
                columns: output.columns,
                is_tty,
                show_error_detail: output.show_error_detail,
                stderr_tail,
            },
            multi,
            state: ProgressState::new(display_map),
            running_style,
            footer_separator: None,
            footer_bar: None,
            header_bar: None,
            completed_steps: 0,
            failed_steps: 0,
            skipped_steps: 0,
            total_steps: 0,
            run_started_at: None,
            tap,
            _not_send: PhantomData,
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
    pub(super) fn write_non_tty(&self, line: &str) {
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

    /// PERF-3 / TASK-0776: take an owned `String` instead of `&str` so the
    /// indicatif `finish_with_message` move avoids an extra `to_string`
    /// allocation. Every caller already constructs `line` as a `String`,
    /// and the non-TTY mirror borrows back via `&line` before the move.
    pub(super) fn finish_bar(&self, bar: &ProgressBar, line: String) {
        bar.set_style(pending_style());
        self.write_non_tty(&line);
        bar.finish_with_message(line);
    }

    /// Dispatch a RunnerEvent to the appropriate handler method.
    ///
    /// CONC-5 / TASK-0331 — async-safety invariant: this method performs
    /// blocking I/O (synchronous `write(2)` on the tap file and stderr) and
    /// must only be driven from a synchronous event-pump loop, never polled
    /// inside a tokio task. CL-3 / TASK-0656 makes the constraint structural:
    /// [`ProgressDisplay`] holds a `PhantomData<*const ()>` marker, so it is
    /// `!Send + !Sync`. Any future that borrows `&mut self` across an
    /// `.await` is therefore non-`Send` and cannot be passed to
    /// `tokio::spawn` on the multi-thread scheduler — the build fails
    /// instead of silently regressing CONC-5.
    ///
    /// ```compile_fail
    /// fn assert_send<T: Send>() {}
    /// assert_send::<ops_runner::display::ProgressDisplay>();
    /// ```
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
                // CONC-7 / TASK-0457: surface dropped lines only at DEBUG so a
                // green run isn't visually polluted by a noisy step. The
                // tracing event still records the count for postmortem; the
                // user-facing stderr/tap line is gated on the tracing level
                // so `OPS_LOG_LEVEL=debug` (or `RUST_LOG`) re-enables it.
                let line = format!("[ops] {id}: {dropped_count} output line(s) dropped under load");
                tracing::debug!(target: "ops::runner", "{line}");
                if tracing::enabled!(target: "ops::runner", tracing::Level::DEBUG) {
                    self.emit_line(&line);
                    self.tap_line_for(&line, Some(id.as_str()));
                }
            }
        }
    }

    fn on_plan_started(&mut self, command_ids: &[CommandId]) {
        self.state.reset_for_plan(command_ids);

        self.total_steps = command_ids.len();
        self.completed_steps = 0;
        self.failed_steps = 0;
        self.skipped_steps = 0;
        self.run_started_at = Some(Instant::now());

        let is_boxed = self
            .render
            .theme
            .box_top_border(BoxSnapshot {
                completed: 0,
                failed: 0,
                skipped: 0,
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
            failed: self.failed_steps,
            skipped: self.skipped_steps,
            total: self.total_steps,
            elapsed_secs: elapsed,
            success: success_so_far,
            columns: self.render.columns,
            command_ids: &self.state.plan_command_ids,
        }
    }

    pub(super) fn render_header_message(&self) -> String {
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
    pub(super) fn render_and_wrap_step(&self, step: &StepLine) -> String {
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
        if let Some(ref mut tap) = self.tap {
            tap.write_line(line, step_id);
        }
    }

    fn on_step_output(&mut self, id: &str, line: crate::command::OutputLine, stderr: bool) {
        if stderr {
            self.state
                .record_stderr(id, line.clone(), self.render.stderr_tail.cap());
        }
        self.tap_line_for(line.as_str(), Some(id));
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
        } else if matches!(status, StepStatus::Skipped) {
            self.skipped_steps += 1;
        }
        let line = self.render_and_wrap_step(&step);
        self.finish_bar(&self.state.bars[i], line);
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

        let stderr_tail = self
            .state
            .step_stderr
            .get_mut(id)
            .map(|buf| {
                ErrorDetailRenderer::extract_stderr_tail(
                    buf.make_contiguous(),
                    self.render.stderr_tail.max_lines(),
                )
            })
            .unwrap_or_default();
        let renderer = ErrorDetailRenderer::new(&self.render.theme, self.render.columns);
        let detail_lines = renderer.render(message, &stderr_tail);

        if self.render.is_tty {
            self.render_error_details_tty(i, &detail_lines);
        } else {
            Self::render_error_details_non_tty(&detail_lines);
        }
    }

    fn on_run_finished(&mut self, duration_secs: f64, success: bool) {
        // FN-1 (TASK-0582) + ARCH-1 (TASK-0581): each finishing concern lives
        // in its own helper in the `finalize` submodule. This body is a flat
        // dispatcher.
        self.finalize_orphan_bars();
        self.report_tap_truncation();
        if self.finalize_boxed_layout(duration_secs, success) {
            return;
        }
        self.finalize_flat_layout(duration_secs, success);
    }
}
