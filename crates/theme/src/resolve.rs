//! Theme resolution: name → [`ConfigurableTheme`] lookup and listing.

use indexmap::IndexMap;

use crate::configurable::ConfigurableTheme;
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

/// Resolve a theme name to a concrete [`ConfigurableTheme`].
///
/// Looks up the theme in the provided IndexMap (includes built-in themes from default config).
pub fn resolve_theme(
    name: &str,
    themes: &IndexMap<String, ThemeConfig>,
) -> Result<ConfigurableTheme, ThemeError> {
    themes
        .get(name)
        .map(|tc| ConfigurableTheme::new(tc.clone()))
        .ok_or_else(|| ThemeError::NotFound(name.to_string()))
}

/// Owning sibling of [`resolve_theme`]: take the named entry out of `themes`
/// via `IndexMap::swap_remove`, avoiding the per-call `ThemeConfig::clone`.
///
/// OWN-4 / TASK-0836: `ThemeConfig` is ~13 `String` fields plus an
/// `ErrorBlockChars` (5 strings), so the unconditional clone in
/// [`resolve_theme`] is non-trivial on every CLI run. Use this variant when
/// the caller owns the theme map and does not need to look the entry up
/// again — `swap_remove` removes the value from the map and hands ownership
/// to the constructor.
pub fn resolve_theme_owned(
    name: &str,
    themes: &mut IndexMap<String, ThemeConfig>,
) -> Result<ConfigurableTheme, ThemeError> {
    themes
        .swap_remove(name)
        .map(ConfigurableTheme::new)
        .ok_or_else(|| ThemeError::NotFound(name.to_string()))
}

/// List all available theme names.
/// Note: Used by tests and available for programmatic access.
pub fn list_theme_names(themes: &IndexMap<String, ThemeConfig>) -> Vec<String> {
    themes.keys().cloned().collect()
}
