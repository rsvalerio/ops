//! Theme types and step-line rendering.
//!
//! [`ThemeConfig`] is the serializable theme definition (TOML-compatible),
//! defined in `ops-core` and re-exported here for convenience.
//! [`ConfigurableTheme`] wraps a `ThemeConfig` and renders step lines and
//! error details.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

mod configurable;
mod render;
mod resolve;
mod step_line_theme;
pub mod style;

pub use configurable::ConfigurableTheme;
pub use ops_core::config::theme_types;
pub use ops_core::config::theme_types::{ErrorBlockChars, PlanHeaderStyle, ThemeConfig};
pub use render::render_error_block;
pub use resolve::{list_theme_names, resolve_theme, resolve_theme_owned, ThemeError};
pub use step_line_theme::{format_duration, BoxSnapshot, SlotLine, StepPrefixParts};
pub use style::{apply_style, strip_ansi, visible_width};

#[cfg(test)]
mod tests;
