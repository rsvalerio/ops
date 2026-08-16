//! Hierarchical configuration parsing and command resolution.
//!
//! Resolution order: internal default → global config → local `.ops.toml` → env vars.
//!
//! This module is a re-export hub; the types live in focused submodules:
//!
//! - [`root`] — the root [`Config`] type and its validation.
//! - [`sections`] — the `[extensions]`, `[about]`, `[data]`, `[output]` sections.
//! - [`commands`] / [`command_id`] — command specs and the [`CommandId`] newtype.
//! - [`overlay`] / [`merge`] — the partial-config mirror types and their merge.
//! - [`loader`] — the file/env resolution order described above.
//! - [`init`] — `ops init` template rendering.
//! - [`edit`] — in-place `.ops.toml` editing.
//! - [`theme_types`] / [`tools`] — the `[themes]` and `[tools]` payload types.

pub(crate) mod command_id;
pub(crate) mod commands;
mod edit;
mod init;
mod loader;
pub(crate) mod merge;
pub(crate) mod overlay;
pub(crate) mod root;
pub(crate) mod sections;
pub mod theme_types;
pub mod tools;

pub use command_id::CommandId;
pub use commands::{CommandSpec, CompositeCommandSpec, ExecCommandSpec};
pub use edit::{
    atomic_write, command_names, edit_ops_toml, ensure_table, insert_command, read_ops_toml,
    write_ops_toml,
};
pub use init::{default_ops_toml, init_template, InitSections};
pub use overlay::{
    AboutConfigOverlay, ConfigOverlay, DataConfigOverlay, ExtensionConfigOverlay,
    OutputConfigOverlay,
};
pub use root::{Config, MAX_COMPOSITE_DEPTH};
pub use sections::{AboutConfig, DataConfig, ExtensionConfig, OutputConfig};

#[cfg(test)]
pub(crate) use loader::resolve_global_config_path;
pub use loader::{
    load_config, load_config_at, load_config_or_default, load_config_or_default_at,
    read_config_file,
};
#[cfg(any(test, feature = "test-support"))]
pub use loader::{
    load_config_call_count, reset_global_config_path_cache, reset_load_config_call_count,
    GlobalConfigPathResetToken,
};
pub use merge::merge_config;

#[cfg(test)]
mod tests;
