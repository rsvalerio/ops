//! Audit-tracked extension registration for commands and data providers.
//!
//! Split out from the parent `registry` module per ARCH-1 / TASK-0842 so
//! the symmetric command / data-provider registration paths and their
//! shared [`Owner`] machinery live in one place separately from the
//! discovery/filtering surface in [`super::discovery`].

use super::discovery::{as_ext_refs, builtin_extensions};
use ops_core::config::Config;
use ops_extension::{CommandRegistry, DataRegistry, Extension};
use std::path::Path;
use tracing::debug;

/// DUP-1 / TASK-0876: shared owner-tracking type used by the symmetric
/// command and data-provider registration paths. Keeping the enum in one
/// place removes the asymmetry channel that originally produced
/// TASK-0756 (data providers had no audit, commands did) and makes any
/// future change to the audit policy a one-touch edit.
#[derive(Clone)]
pub(super) enum Owner {
    PreExisting,
    Extension(&'static str),
}

/// Seed an owner map from the keys already present in the target registry
/// so the first extension's contributions still see existing keys as
/// foreign-owned rather than producing false-positive collisions across
/// re-entries.
pub(super) fn seed_owners<I, K>(keys: I) -> std::collections::HashMap<K, Owner>
where
    I: IntoIterator<Item = K>,
    K: std::hash::Hash + Eq,
{
    let mut owners = std::collections::HashMap::new();
    for k in keys {
        owners.entry(k).or_insert(Owner::PreExisting);
    }
    owners
}

/// Collect all commands from registered extensions into a registry.
///
/// SEC-31 / TASK-0402 (symmetric with TASK-0350 for `DataRegistry`):
/// extensions register into a shared `CommandRegistry` via `IndexMap::insert`.
/// We snapshot the keys after each extension's contribution so a second
/// extension introducing a colliding command id is logged at
/// `tracing::warn!` instead of silently shadowing the first registration.
/// Insertion order is preserved (the late entry wins, matching the prior
/// observable behaviour) but the collision is now visible.
pub fn register_extension_commands(extensions: &[&dyn Extension], registry: &mut CommandRegistry) {
    // READ-5 / TASK-0716: typed [`Owner`] keeps the warn fields free of
    // internal sentinel strings (`<pre-existing>` no longer leaks).
    let mut owners = seed_owners(registry.keys().cloned());

    for ext in extensions {
        debug!(extension = ext.name(), action = "commands", "registering");
        // PERF-1 / TASK-0512: register into a per-extension scratch registry
        // so we can detect collisions in O(commands_this_ext) instead of
        // snapshotting every key in the shared registry on each iteration.
        let mut local = CommandRegistry::new();
        ext.register_commands(&mut local);
        // ERR-2 (TASK-0579): the per-extension scratch registry tracks
        // duplicate inserts so a single extension that registers the same
        // command id twice no longer silently drops the first version.
        for dup in local.take_duplicate_inserts() {
            tracing::warn!(
                command = %dup,
                extension = ext.name(),
                "extension registered the same command id more than once; the later registration shadows the earlier within this extension"
            );
        }
        for (id, spec) in local {
            let prev_owner = owners.insert(id.clone(), Owner::Extension(ext.name()));
            match prev_owner {
                Some(Owner::Extension(prev)) if prev != ext.name() => {
                    tracing::warn!(
                        command = %id,
                        first = %prev,
                        second = %ext.name(),
                        "duplicate command registration; the later extension shadows the earlier one"
                    );
                }
                Some(Owner::PreExisting) => {
                    tracing::warn!(
                        command = %id,
                        second = %ext.name(),
                        "extension command shadows an entry already present in the registry (e.g. a config-defined command)"
                    );
                }
                _ => {}
            }
            registry.insert(id, spec);
        }
    }
}

/// Collect all data providers from registered extensions.
///
/// CL-5 / TASK-0756: symmetric with [`register_extension_commands`]. Each
/// extension registers into a per-extension scratch [`DataRegistry`] so the
/// wiring layer can detect (a) in-extension duplicates via
/// [`DataRegistry::take_duplicate_inserts`] and (b) cross-extension or
/// pre-existing-owner collisions via the local `owners` map. Earlier the
/// data-provider path was a thin pass-through with no audit at all, so a
/// silent first-write-wins drop here was invisible to operators reading
/// `RUST_LOG=ops=debug` even though the symmetric command-registration path
/// already warned loudly on every collision class.
pub fn register_extension_data_providers(
    extensions: &[&dyn Extension],
    registry: &mut DataRegistry,
) {
    let mut owners = seed_owners(registry.provider_names().into_iter().map(str::to_string));

    for ext in extensions {
        debug!(
            extension = ext.name(),
            action = "data_providers",
            "registering"
        );
        let mut local = DataRegistry::new();
        ext.register_data_providers(&mut local);

        // ERR-2: a single extension that registers the same provider name
        // twice surfaces here via the audit trail rather than a silent drop.
        for dup in local.take_duplicate_inserts() {
            tracing::warn!(
                provider = %dup,
                extension = ext.name(),
                "extension registered the same data provider name more than once; first-write-wins keeps the earlier registration within this extension and the later ones are dropped"
            );
        }

        for (name, provider) in local {
            use std::collections::hash_map::Entry;
            match owners.entry(name.clone()) {
                Entry::Occupied(occ) => match occ.get() {
                    Owner::Extension(prev) if *prev != ext.name() => {
                        tracing::warn!(
                            provider = %name,
                            first = %prev,
                            second = %ext.name(),
                            "duplicate data provider registration; first-write-wins keeps the earlier extension's provider and the later one is dropped"
                        );
                    }
                    Owner::PreExisting => {
                        tracing::warn!(
                            provider = %name,
                            second = %ext.name(),
                            "extension data provider would shadow an entry already present in the registry; first-write-wins keeps the existing one"
                        );
                    }
                    Owner::Extension(_) => {
                        // Same extension — already surfaced via take_duplicate_inserts above.
                    }
                },
                Entry::Vacant(vac) => {
                    vac.insert(Owner::Extension(ext.name()));
                    registry.register(name, provider);
                }
            }
        }
    }
}

/// DUP-003: Build a DataRegistry from all enabled extensions in one call.
///
/// Reduces the 4-line boilerplate of builtin_extensions + ext_refs + new registry + register.
pub fn build_data_registry(config: &Config, workspace_root: &Path) -> anyhow::Result<DataRegistry> {
    let exts = builtin_extensions(config, workspace_root)?;
    let mut registry = DataRegistry::new();
    register_extension_data_providers(&as_ext_refs(&exts), &mut registry);
    Ok(registry)
}
