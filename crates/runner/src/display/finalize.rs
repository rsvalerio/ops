//! Run-finished finalization helpers for `ProgressDisplay`.
//!
//! ARCH-1 (TASK-0581) + FN-1 (TASK-0582): the `RunFinished` dispatch
//! previously dispatched four independent finishing concerns inline:
//! orphan-bar finalization, tap-truncation reporting, boxed-layout border
//! emission, and flat-layout summary fallback. Each owns its own state
//! machine and failure mode, so each lives in its own helper here.

use super::style::pending_style;
use super::{write_stderr, ProgressDisplay};
use indicatif::ProgressBar;
use ops_core::output::{StepLine, StepStatus};
use ops_theme::{self as theme, BoxSnapshot};
use std::io::{self, Write};

impl ProgressDisplay {
    /// Finalize any step bars still in the "running" state. Without this, a
    /// bar whose task was aborted mid-flight (e.g. `fail_fast` cancellation)
    /// never receives a `StepFinished/Failed/Skipped` event, so its row gets
    /// dropped from the multi-progress draw on the next redraw — leaving a
    /// hole in the boxed frame and a visible row count that disagrees with
    /// `Done N/M`.
    ///
    /// Each finalized orphan also bumps `completed_steps` (ERR-1 / TASK-0333):
    /// without that, the footer shows `Done 1/3` while three rows are visibly
    /// finished — defeating the very disagreement this routine was added to
    /// fix.
    pub(super) fn finalize_orphan_bars(&mut self) {
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
            self.skipped_steps += 1;
            let line = self.render_and_wrap_step(&step);
            self.finish_bar(&self.state.bars[i], line);
        }
    }

    /// ERR-2 / TASK-0458 + ARCH-1 / TASK-0581: surface tap-file truncation
    /// if we hit a write error mid-run. Emitted exactly once per run to
    /// both stderr (for the user) and the tap file itself (for downstream
    /// test harnesses that scan the tap), so a partial tap is never
    /// silently treated as "no failures". The tap re-open is best-effort.
    pub(super) fn report_tap_truncation(&mut self) {
        let Some(ref mut tap) = self.tap else {
            return;
        };
        let Some((step_id, kind)) = tap.take_truncation() else {
            return;
        };
        let line = format!("[ops] tap file truncated after step {step_id} due to: {kind}");
        tracing::warn!(target: "ops::tap", "{}", line);
        write_stderr(Some(&line));
        tap.append_marker(&line);
    }

    /// Finalize the boxed layout: locks the live header to "Done" and emits
    /// the bottom border. Returns `true` when the active theme renders a
    /// boxed layout and no further finalization is needed.
    pub(super) fn finalize_boxed_layout(&mut self, duration_secs: f64, success: bool) -> bool {
        let Some(bottom) = self.render.theme.box_bottom_border(BoxSnapshot {
            completed: self.completed_steps,
            failed: self.failed_steps,
            skipped: self.skipped_steps,
            total: self.total_steps,
            elapsed_secs: duration_secs,
            success,
            columns: self.render.columns,
            command_ids: &self.state.plan_command_ids,
        }) else {
            return false;
        };
        if let Some(ref hb) = self.header_bar {
            hb.finish_with_message(self.render_header_message());
        }
        if let Some(ref fb) = self.footer_bar {
            fb.finish_with_message(bottom.clone());
            self.write_non_tty(&bottom);
        } else {
            let pb = self.multi.add(ProgressBar::new(0));
            self.finish_bar(&pb, bottom);
        }
        true
    }

    /// Finalize the flat (non-boxed) layout: emit the summary line into the
    /// existing footer bar, or as a fallback create a separator + summary
    /// pair when no plan was ever started.
    pub(super) fn finalize_flat_layout(&mut self, duration_secs: f64, success: bool) {
        let summary = self.format_summary(duration_secs, success);

        if let Some(ref fb) = self.footer_bar {
            fb.finish_with_message(summary.clone());
            self.write_non_tty(&summary);
            return;
        }

        self.render_fallback_separator();
        let summary_pb = self.multi.add(ProgressBar::new(0));
        self.finish_bar(&summary_pb, summary);
    }

    fn format_summary(&self, duration_secs: f64, success: bool) -> String {
        if self.total_steps > 0 {
            let elapsed = theme::format_duration(duration_secs);
            // CL-3 / TASK-0771: on failure, surface the succeeded/skipped/failed
            // breakdown rather than a single "Failed N/M" — that label
            // conflated terminal-step count with success count.
            let body = if success {
                format!(
                    "Done {}/{} in {}",
                    self.completed_steps, self.total_steps, elapsed
                )
            } else {
                let succeeded = self
                    .completed_steps
                    .saturating_sub(self.failed_steps)
                    .saturating_sub(self.skipped_steps);
                format!(
                    "{} succeeded, {} skipped, {} failed of {} in {}",
                    succeeded, self.skipped_steps, self.failed_steps, self.total_steps, elapsed
                )
            };
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
