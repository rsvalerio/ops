//! Theme resolution: name → [`StepLineTheme`] lookup and listing.

use indexmap::IndexMap;

use crate::configurable::ConfigurableTheme;
use crate::step_line_theme::StepLineTheme;
use crate::ThemeConfig;

/// Error type for theme resolution failures.
///
/// Marked `#[non_exhaustive]`: this crate is extension-facing, and future
/// variants (e.g. `NotConfigured`, `InvalidField`) should be additive rather
/// than SemVer-breaking.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
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
        .map(|tc| Box::new(ConfigurableTheme::new(tc.clone())) as Box<dyn StepLineTheme>)
        .ok_or_else(|| ThemeError::NotFound(name.to_string()))
}

/// List all available theme names.
/// Note: Used by tests and available for programmatic access.
pub fn list_theme_names(themes: &IndexMap<String, ThemeConfig>) -> Vec<String> {
    themes.keys().cloned().collect()
}
