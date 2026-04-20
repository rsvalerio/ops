//! Render-time configuration and constructor arguments for `ProgressDisplay`.

use indexmap::IndexMap;
use ops_core::config;
use ops_theme::{self as theme, ThemeConfig};
use std::collections::HashMap;
use std::path::PathBuf;

/// Render configuration extracted from `OutputConfig`.
pub struct RenderConfig {
    pub theme: Box<dyn theme::StepLineTheme>,
    pub columns: u16,
    pub is_tty: bool,
    pub show_error_detail: bool,
    pub stderr_tail_lines: usize,
}

/// Named constructor arguments for `ProgressDisplay::new`.
pub struct DisplayOptions<'a> {
    pub output: &'a config::OutputConfig,
    pub display_map: HashMap<String, String>,
    pub custom_themes: &'a IndexMap<String, ThemeConfig>,
    pub tap: Option<PathBuf>,
}
