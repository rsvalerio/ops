//! Overlay structs used during hierarchical config merging.
//!
//! Extracted from `config/mod.rs` (ARCH-1 / TASK-0343). Each `*Overlay`
//! mirrors one field of [`super::Config`] with all leaves wrapped in
//! `Option`, so that an overlay only overwrites values explicitly set in
//! the higher-priority source.

use std::path::PathBuf;

use indexmap::IndexMap;
use serde::Deserialize;

use super::commands::CommandSpec;
use super::theme_types::ThemeConfig;
use super::tools::ToolSpec;

/// Overlay configuration with optional fields — only explicitly-set values
/// overwrite the base config during merging.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ConfigOverlay {
    #[serde(default)]
    pub output: Option<OutputConfigOverlay>,
    #[serde(default)]
    pub commands: Option<IndexMap<String, CommandSpec>>,
    #[serde(default)]
    pub data: Option<DataConfigOverlay>,
    #[serde(default)]
    pub themes: Option<IndexMap<String, ThemeConfig>>,
    #[serde(default)]
    pub extensions: Option<ExtensionConfigOverlay>,
    #[serde(default)]
    pub about: Option<AboutConfigOverlay>,
    #[serde(default)]
    pub stack: Option<String>,
    #[serde(default)]
    pub tools: Option<IndexMap<String, ToolSpec>>,
}

/// Generate a single-field overlay struct (DUP-3 collapse).
///
/// `ExtensionConfigOverlay`, `AboutConfigOverlay`, and `DataConfigOverlay`
/// all followed the same shape: one `Option<T>` field plus
/// `serde(deny_unknown_fields)`. Adding another single-field overlay used to
/// mean copy-pasting the entire struct + derives + doc comment; the macro
/// keeps the surface identical across all three so drift can't creep in.
macro_rules! single_field_overlay {
    ($( #[$meta:meta] )* $name:ident, $field:ident : $ty:ty) => {
        $( #[$meta] )*
        #[derive(Debug, Clone, Default, Deserialize)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            pub $field: Option<$ty>,
        }
    };
}

single_field_overlay!(
    /// Overlay for extension settings.
    ExtensionConfigOverlay, enabled: Vec<String>
);

single_field_overlay!(
    /// Overlay for about settings.
    AboutConfigOverlay, fields: Vec<String>
);

single_field_overlay!(
    /// Overlay for data settings.
    DataConfigOverlay, path: PathBuf
);

/// Overlay for output settings — each field is optional so partial configs
/// don't overwrite intentional base values with defaults.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputConfigOverlay {
    pub theme: Option<String>,
    pub columns: Option<u16>,
    pub show_error_detail: Option<bool>,
    pub stderr_tail_lines: Option<usize>,
    pub category_order: Option<Vec<String>>,
}
