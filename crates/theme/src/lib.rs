//! Theme types and step-line rendering.
//!
//! [`ThemeConfig`] is the serializable theme definition (TOML-compatible),
//! defined in `ops-core` and re-exported here for convenience.
//! [`ConfigurableTheme`] wraps a `ThemeConfig` and implements [`StepLineTheme`]
//! for rendering step lines and error details.

mod configurable;
mod render;
mod step_line_theme;

pub use configurable::ConfigurableTheme;
pub use ops_core::config::theme_types;
pub use ops_core::config::theme_types::{ErrorBlockChars, PlanHeaderStyle, ThemeConfig};
pub use render::render_error_block;
pub use step_line_theme::{format_duration, StepLineTheme};

use indexmap::IndexMap;

/// Error type for theme resolution failures.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ThemeError {
    #[error("Theme not found: {0}")]
    NotFound(String),
}

/// Resolve a theme name to a concrete [`StepLineTheme`] implementation.
///
/// Looks up the theme in the provided IndexMap (includes built-in themes from default config).
pub fn resolve_theme(
    name: &str,
    themes: &IndexMap<String, ThemeConfig>,
) -> Result<Box<dyn StepLineTheme>, ThemeError> {
    themes
        .get(name)
        .map(|tc| Box::new(ConfigurableTheme(tc.clone())) as Box<dyn StepLineTheme>)
        .ok_or_else(|| ThemeError::NotFound(name.to_string()))
}

/// List all available theme names.
/// Note: Used by tests and available for programmatic access.
pub fn list_theme_names(themes: &IndexMap<String, ThemeConfig>) -> Vec<String> {
    themes.keys().cloned().collect()
}

#[cfg(test)]
mod tests;
