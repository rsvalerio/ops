//! Theme types and step-line rendering.
//!
//! [`ThemeConfig`] is the serializable theme definition (TOML-compatible),
//! defined in `ops-core` and re-exported here for convenience.
//! [`ConfigurableTheme`] wraps a `ThemeConfig` and implements [`StepLineTheme`]
//! for rendering step lines and error details.

mod configurable;
mod render;
mod resolve;
mod step_line_theme;
pub mod style;

pub use configurable::ConfigurableTheme;
pub use ops_core::config::theme_types;
pub use ops_core::config::theme_types::{ErrorBlockChars, PlanHeaderStyle, ThemeConfig};
pub use render::render_error_block;
pub use resolve::{list_theme_names, resolve_theme, ThemeError};
pub use step_line_theme::{format_duration, BoxSnapshot, StepLineTheme};
pub use style::{apply_style, strip_ansi};

#[cfg(test)]
mod tests;
