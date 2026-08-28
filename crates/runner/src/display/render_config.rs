//! Render-time configuration and constructor arguments for `ProgressDisplay`.

use indexmap::IndexMap;
use ops_core::config;
use ops_theme::{ConfigurableTheme, ThemeConfig};
use std::collections::HashMap;
use std::path::PathBuf;

/// Typed stderr tail policy — replaces the old `usize::MAX` sentinel.
///
/// TASK-0762: the display layer decides unbounded vs capped; the config
/// field stores the user's value verbatim and is never mutated post-load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StderrTail {
    /// Show all captured stderr lines (verbose mode).
    ///
    /// PERF-3 / TASK-1925: `--verbose` **intentionally removes the ring's
    /// eviction bound** — `cap()` returns `usize::MAX`, so `record_stderr`
    /// never evicts and the deque grows one entry per stderr line for every
    /// step until the plan ends. That is the point of the mode (the whole
    /// stream is meant to be renderable), and it is bounded in practice by
    /// what the runner captured in the first place: `OPS_OUTPUT_BYTE_CAP`
    /// per stream per step. Since TASK-1925 each retained line owns only its
    /// own bytes, so the growth is the lines themselves rather than one
    /// pinned multi-megabyte capture buffer per step.
    Unbounded,
    /// Show at most N tail lines.
    Limited(usize),
}

impl StderrTail {
    /// Return the ring-buffer cap. `Unbounded` returns `usize::MAX` so the
    /// existing `record_stderr` cap logic works unchanged.
    #[must_use]
    pub const fn cap(self) -> usize {
        match self {
            Self::Unbounded => usize::MAX,
            Self::Limited(n) => n,
        }
    }

    /// Return the max tail lines to extract for error detail rendering.
    #[must_use]
    pub const fn max_lines(self) -> usize {
        self.cap()
    }
}

/// Render configuration extracted from `OutputConfig`.
#[non_exhaustive]
pub struct RenderConfig {
    pub theme: ConfigurableTheme,
    pub columns: u16,
    pub is_tty: bool,
    pub show_error_detail: bool,
    pub stderr_tail: StderrTail,
}

impl RenderConfig {
    #[must_use]
    pub const fn new(
        theme: ConfigurableTheme,
        columns: u16,
        is_tty: bool,
        show_error_detail: bool,
        stderr_tail: StderrTail,
    ) -> Self {
        Self {
            theme,
            columns,
            is_tty,
            show_error_detail,
            stderr_tail,
        }
    }
}

/// Named constructor arguments for `ProgressDisplay::new`.
#[non_exhaustive]
pub struct DisplayOptions<'a> {
    pub output: &'a config::OutputConfig,
    pub display_map: HashMap<String, String>,
    pub custom_themes: &'a IndexMap<String, ThemeConfig>,
    pub tap: Option<PathBuf>,
    /// When true, stderr tail is unbounded regardless of config setting.
    pub verbose: bool,
}

impl<'a> DisplayOptions<'a> {
    #[must_use]
    pub const fn new(
        output: &'a config::OutputConfig,
        display_map: HashMap<String, String>,
        custom_themes: &'a IndexMap<String, ThemeConfig>,
        tap: Option<PathBuf>,
        verbose: bool,
    ) -> Self {
        Self {
            output,
            display_map,
            custom_themes,
            tap,
            verbose,
        }
    }
}
