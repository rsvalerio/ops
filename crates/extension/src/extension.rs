//! Extension trait, type flags, info, and registries.

use crate::data::{DataField, DataProviderSchema, DataRegistry};
use indexmap::IndexMap;
use ops_core::config::{CommandId, CommandSpec, Config};
pub use ops_core::stack::Stack;
use std::path::Path;

/// Factory function that creates an extension instance given config and workspace root.
/// Registered automatically by `impl_extension!` when the `factory:` arm is provided.
pub type ExtensionFactory = fn(&Config, &Path) -> Option<(&'static str, Box<dyn Extension>)>;

/// Distributed slice collecting all extension factories at link time.
/// Extensions contribute to this slice via `impl_extension!` with a `factory:` arm.
#[linkme::distributed_slice]
pub static EXTENSION_REGISTRY: [ExtensionFactory];

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct ExtensionType: u8 {
        const DATASOURCE = 0b01;
        const COMMAND    = 0b10;
    }
}

impl ExtensionType {
    pub fn is_datasource(self) -> bool {
        self.contains(Self::DATASOURCE)
    }

    pub fn is_command(self) -> bool {
        self.contains(Self::COMMAND)
    }
}

/// Metadata describing an extension.
///
/// API-9 / TASK-0349: marked `#[non_exhaustive]` so that adding a field
/// here is not a SemVer break for downstream extensions. External callers
/// should construct via [`ExtensionInfo::new`] (and adjust fields via
/// direct field access — the fields stay `pub` for ergonomic struct
/// updates inside this crate and for read access from outside).
#[non_exhaustive]
pub struct ExtensionInfo {
    pub name: &'static str,
    pub shortname: &'static str,
    pub description: &'static str,
    pub types: ExtensionType,
    pub command_names: &'static [&'static str],
    pub data_provider_name: Option<&'static str>,
    pub stack: Option<Stack>,
}

impl ExtensionInfo {
    /// Build a minimal `ExtensionInfo` from `name`. All other fields default
    /// to empty/None and may be set via direct field access. Required for
    /// downstream extensions because the struct is `#[non_exhaustive]` and
    /// cannot be constructed via struct-literal syntax.
    #[must_use]
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            shortname: name,
            description: "",
            types: ExtensionType::empty(),
            command_names: &[],
            data_provider_name: None,
            stack: None,
        }
    }
}

/// Registry of command ID → CommandSpec (from config + extensions).
///
/// An `IndexMap` preserving insertion order, mapping command names to their
/// specifications. Commands are registered by:
/// 1. Config file (`[commands.*]` sections)
/// 2. Extensions via [`Extension::register_commands`]
///
/// Config-defined commands take precedence over extension commands when
/// merged into the `CommandRunner`.
pub type CommandRegistry = IndexMap<CommandId, CommandSpec>;

/// Extension: registers commands and/or data providers.
///
/// Extensions are the primary mechanism for adding functionality to ops.
/// They can register:
/// - **Commands**: New named commands available via `cargo ops <name>`
/// - **Data providers**: Named data sources queryable by other extensions
///
/// # Lifecycle
///
/// 1. Extensions are instantiated by `builtin_extensions()` based on config
/// 2. `register_commands()` is called to add commands to the registry
/// 3. `register_data_providers()` is called to add data providers
/// 4. The registries are attached to the `CommandRunner`
///
/// # Example
///
/// ```text
/// struct MyExtension;
///
/// impl Extension for MyExtension {
///     fn name(&self) -> &'static str { "my-ext" }
///
///     fn register_commands(&self, registry: &mut CommandRegistry) {
///         registry.insert("my-cmd".into(), CommandSpec::Exec(...));
///     }
///
///     fn register_data_providers(&self, registry: &mut DataRegistry) {
///         registry.register("my-data", Box::new(MyDataProvider));
///     }
/// }
/// ```
pub trait Extension: Send + Sync {
    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str {
        ""
    }

    fn shortname(&self) -> &'static str {
        self.name()
    }

    fn types(&self) -> ExtensionType {
        ExtensionType::empty()
    }

    fn command_names(&self) -> &'static [&'static str] {
        &[]
    }

    fn data_provider_name(&self) -> Option<&'static str> {
        None
    }

    fn stack(&self) -> Option<Stack> {
        None
    }

    fn info(&self) -> ExtensionInfo {
        ExtensionInfo {
            name: self.name(),
            shortname: self.shortname(),
            description: self.description(),
            types: self.types(),
            command_names: self.command_names(),
            data_provider_name: self.data_provider_name(),
            stack: self.stack(),
        }
    }

    fn register_commands(&self, registry: &mut CommandRegistry);

    fn register_data_providers(&self, _registry: &mut DataRegistry) {}
}

// Suppress unused import warnings — these are used by the macros in macros.rs
// which reference them via `$crate::` paths, but the compiler doesn't see
// that usage from this module's perspective.
const _: () = {
    fn _assert_imports_used() {
        let _ = std::any::type_name::<DataField>();
        let _ = std::any::type_name::<DataProviderSchema>();
    }
};
