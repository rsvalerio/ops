//! Extension trait, type flags, info, and registries.

use crate::data::DataRegistry;
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
/// Wraps an [`IndexMap`] preserving insertion order. Commands are registered by:
/// 1. Config file (`[commands.*]` sections)
/// 2. Extensions via [`Extension::register_commands`]
///
/// Config-defined commands take precedence over extension commands when
/// merged into the `CommandRunner`.
///
/// ERR-2 (TASK-0579): unlike a bare `IndexMap`, [`CommandRegistry::insert`]
/// remembers any keys that get re-inserted during the lifetime of the
/// registry instance. The CLI wiring layer drains that list after each
/// extension's `register_commands` call so a single extension that registers
/// the same command id twice surfaces a `tracing::warn!` event instead of
/// silently dropping the first registration.
#[derive(Debug, Default)]
pub struct CommandRegistry {
    inner: IndexMap<CommandId, CommandSpec>,
    duplicate_inserts: Vec<CommandId>,
}

// TRAIT-4 (TASK-0653): `duplicate_inserts` is a per-instance audit trail
// drained once by `take_duplicate_inserts`. A blanket `derive(Clone)`
// would copy that history into the clone, so a downstream reader would
// see phantom warnings. Clone the data only and reset the audit trail.
impl Clone for CommandRegistry {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            duplicate_inserts: Vec::new(),
        }
    }
}

impl CommandRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a command, returning the previous value if any. ERR-2
    /// (TASK-0579): a re-insert for the same id is recorded so the CLI
    /// wiring layer can warn about within-extension self-shadowing instead
    /// of letting the silent overwrite swallow the first registration.
    ///
    /// CL-5 / TASK-0661: this registry is **last-write-wins** (matching
    /// `IndexMap::insert` semantics) so config-defined commands can shadow
    /// extension-provided commands when merged at the wiring layer.
    /// Contrast with [`crate::DataRegistry::register`] which is
    /// **first-write-wins**: data providers are security-trusted built-ins
    /// (identity, metadata) that must not be silently shadowed by a later
    /// extension. The asymmetry is intentional.
    pub fn insert(&mut self, id: CommandId, spec: CommandSpec) -> Option<CommandSpec> {
        // PATTERN-3 / TASK-0753: route through `Entry` so the duplicate-input
        // path consults the hash map exactly once. The previous shape did a
        // `contains_key`, then cloned the input id into the audit trail, then
        // called `insert` — three probes per registration. The audit clone
        // now uses the key already stored in the map, leaving the input `id`
        // untouched so the caller's allocation moves straight into the map
        // (or is dropped) without a heap copy on the duplicate path.
        match self.inner.entry(id) {
            indexmap::map::Entry::Occupied(mut occupied) => {
                self.duplicate_inserts.push(occupied.key().clone());
                Some(occupied.insert(spec))
            }
            indexmap::map::Entry::Vacant(vacant) => {
                vacant.insert(spec);
                None
            }
        }
    }

    /// Drain ids that were re-inserted into this registry since the last
    /// drain (ERR-2 / TASK-0579). Caller decides how to surface them — the
    /// CLI emits one `tracing::warn!` per duplicate; tests assert the list
    /// is non-empty for malformed extensions.
    pub fn take_duplicate_inserts(&mut self) -> Vec<CommandId> {
        std::mem::take(&mut self.duplicate_inserts)
    }
}

// ARCH-9 (TASK-0652): only `Deref` is exposed so the audit trail in
// `duplicate_inserts` cannot be bypassed by routing mutations through
// `IndexMap::insert` / `entry` / etc. The single mutating path is
// `CommandRegistry::insert`.
//
// ARCH-9 / TASK-0874: `Deref<Target = IndexMap<…>>` is intentional public
// API surface. Every read-only method on `IndexMap` (`get`, `iter`, `len`,
// `keys`, `values`, `contains_key`, `is_empty`, …) is part of the
// `CommandRegistry` contract and downstream extension authors are
// expected to rely on them. The trade-off: swapping the inner storage to
// a non-`IndexMap` map is a breaking change. We accept that — preserving
// insertion order is itself part of the contract (registration order
// drives `--list` ordering and the priority of late-registered overrides
// against the duplicate audit trail), so the implementation type is not
// expected to vary.
impl std::ops::Deref for CommandRegistry {
    type Target = IndexMap<CommandId, CommandSpec>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl IntoIterator for CommandRegistry {
    type Item = (CommandId, CommandSpec);
    type IntoIter = indexmap::map::IntoIter<CommandId, CommandSpec>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a CommandRegistry {
    type Item = (&'a CommandId, &'a CommandSpec);
    type IntoIter = indexmap::map::Iter<'a, CommandId, CommandSpec>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

impl FromIterator<(CommandId, CommandSpec)> for CommandRegistry {
    /// DUP-3 / TASK-1225: drain the audit trail emitted by [`Self::insert`]
    /// and surface duplicates via `tracing::warn!` here, since
    /// `collect()` / `from_iter()` consumers don't see the
    /// `duplicate_inserts` Vec the per-extension registration path drains
    /// explicitly. Without this, building a registry through
    /// `iter.collect()` silently lost the warning signal that ERR-2 /
    /// TASK-0579 hardened the `.insert()` path to preserve.
    fn from_iter<I: IntoIterator<Item = (CommandId, CommandSpec)>>(iter: I) -> Self {
        let mut reg = Self::new();
        for (id, spec) in iter {
            reg.insert(id, spec);
        }
        for dup in reg.take_duplicate_inserts() {
            tracing::warn!(
                command_id = %dup.as_str(),
                "CommandRegistry::from_iter: duplicate command id; later spec overrode earlier — \
                 collect/from_iter drained audit trail in place of explicit insert callers"
            );
        }
        reg
    }
}

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
