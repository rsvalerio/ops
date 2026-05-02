//! Render-time configuration and constructor arguments for `ProgressDisplay`.

use indexmap::IndexMap;
use ops_core::config;
use ops_theme::{self as theme, ThemeConfig};
use std::collections::HashMap;
use std::path::PathBuf;

/// Typed stderr tail policy — replaces the old `usize::MAX` sentinel.
/// TASK-0762: the display layer decides unbounded vs capped; the config
/// field stores the user's value verbatim and is never mutated post-load.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StderrTail {
    /// Show all captured stderr lines (verbose mode).
    Unbounded,
    /// Show at most N tail lines.
    Limited(usize),
}

impl StderrTail {
    /// Return the ring-buffer cap. `Unbounded` returns `usize::MAX` so the
    /// existing `record_stderr` cap logic works unchanged.
    pub fn cap(self) -> usize {
        match self {
            StderrTail::Unbounded => usize::MAX,
            StderrTail::Limited(n) => n,
        }
    }

    /// Return the max tail lines to extract for error detail rendering.
    pub fn max_lines(self) -> usize {
        self.cap()
    }
}

/// Render configuration extracted from `OutputConfig`.
#[non_exhaustive]
pub struct RenderConfig {
    pub theme: Box<dyn theme::StepLineTheme>,
    pub columns: u16,
    pub is_tty: bool,
    pub show_error_detail: bool,
    pub stderr_tail: StderrTail,
}

impl RenderConfig {
    pub fn new(
        theme: Box<dyn theme::StepLineTheme>,
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
    pub fn new(
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
