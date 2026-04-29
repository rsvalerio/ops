//! Render-time configuration and constructor arguments for `ProgressDisplay`.

use indexmap::IndexMap;
use ops_core::config;
use ops_theme::{self as theme, ThemeConfig};
use std::collections::HashMap;
use std::path::PathBuf;

/// Render configuration extracted from `OutputConfig`.
#[non_exhaustive]
pub struct RenderConfig {
    pub theme: Box<dyn theme::StepLineTheme>,
    pub columns: u16,
    pub is_tty: bool,
    pub show_error_detail: bool,
    pub stderr_tail_lines: usize,
}

impl RenderConfig {
    pub fn new(
        theme: Box<dyn theme::StepLineTheme>,
        columns: u16,
        is_tty: bool,
        show_error_detail: bool,
        stderr_tail_lines: usize,
    ) -> Self {
        Self {
            theme,
            columns,
            is_tty,
            show_error_detail,
            stderr_tail_lines,
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
}

impl<'a> DisplayOptions<'a> {
    pub fn new(
        output: &'a config::OutputConfig,
        display_map: HashMap<String, String>,
        custom_themes: &'a IndexMap<String, ThemeConfig>,
        tap: Option<PathBuf>,
    ) -> Self {
        Self {
            output,
            display_map,
            custom_themes,
            tap,
        }
    }
}
