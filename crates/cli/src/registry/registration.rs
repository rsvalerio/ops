//! Audit-tracked extension registration for commands and data providers.
//!
//! Split out from the parent `registry` module per ARCH-1 / TASK-0842 so
//! the symmetric command / data-provider registration paths and their
//! shared [`Owner`] machinery live in one place separately from the
//! discovery/filtering surface in [`super::discovery`].
//!
//! # Collision-resolution policy is asymmetric (CL-5 / TASK-0904)
//!
//! The two registries deliberately resolve duplicates in opposite
//! directions:
//!
//! - **Commands → last-write-wins** ([`register_extension_commands`]).
//!   Mirrors `IndexMap::insert` semantics and the long-standing observed
//!   behaviour of `extensions.enabled` ordering. Operators relying on a
//!   later override (e.g. a stack-specific extension shadowing a generic
//!   one) need this to not silently fail.
//! - **Data providers → first-write-wins**
//!   ([`register_extension_data_providers`]). The security-trusted
//!   default: a data provider is a source of *content* fed back into
//!   commands and rendered into operator-facing artefacts, so a later
//!   extension cannot quietly take over a name that an earlier
//!   extension already claimed.
//!
//! Both paths warn loudly via `tracing::warn!` on every collision class
//! so the chosen winner is always observable. The names of the public
//! functions encode the policy: callers reading `register_*_commands`
//! and `register_*_data_providers` should also read the corresponding
//! function rustdoc to pick the correct semantics.

use super::discovery::{as_ext_refs, builtin_extensions, builtin_extensions_from};
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

/// Snapshot the keys already present in the target registry into an owner
/// map so the first extension's contributions classify existing keys as
/// `PreExisting` rather than producing false-positive collisions.
///
/// **This snapshot is taken once at register-time and is not re-checked.**
/// Subsequent extensions classify their contributions purely via the
/// in-loop `owners.insert` (or `Entry`) collision check inside
/// [`register_extension_commands`] / [`register_extension_data_providers`]:
/// extension N+1 sees N's contributions through that map (because each
/// `Owner::Extension(...)` entry is inserted as it is registered), *not*
/// through this seed. The seed only carries the registry's pre-existing
/// keys (e.g. config-defined commands) into the same classification path.
pub(super) fn snapshot_initial_owners<I, K>(keys: I) -> std::collections::HashMap<K, Owner>
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

/// DUP-2 / TASK-1297: collision-resolution policy on duplicate keys. The
/// command path is last-write-wins; the data-provider path is
/// first-write-wins. See module doc for the security rationale. The
/// enum is private to this module — the public `register_extension_*`
/// shells pick the policy.
#[derive(Clone, Copy)]
enum InsertPolicy {
    LastWriteWins,
    FirstWriteWins,
}

/// DUP-2 / TASK-1297: per-policy warn-message strings used by the shared
/// audit pipeline. Hoisted into a single value rather than four scattered
/// `tracing::warn!` call sites so a future audit-policy change touches one
/// table, not two functions × four messages.
struct AuditPolicy {
    policy: InsertPolicy,
    kind_field: &'static str,
    in_ext_duplicate: &'static str,
    cross_ext_collision: &'static str,
    pre_existing_collision: &'static str,
}

const COMMAND_AUDIT: AuditPolicy = AuditPolicy {
    policy: InsertPolicy::LastWriteWins,
    kind_field: "command",
    in_ext_duplicate:
        "extension registered the same command id more than once; the later registration shadows the earlier within this extension",
    cross_ext_collision:
        "duplicate command registration; the later extension shadows the earlier one",
    pre_existing_collision:
        "extension command shadows an entry already present in the registry (e.g. a config-defined command)",
};

const DATA_PROVIDER_AUDIT: AuditPolicy = AuditPolicy {
    policy: InsertPolicy::FirstWriteWins,
    kind_field: "provider",
    in_ext_duplicate:
        "extension registered the same data provider name more than once; first-write-wins keeps the earlier registration within this extension and the later ones are dropped",
    cross_ext_collision:
        "duplicate data provider registration; first-write-wins keeps the earlier extension's provider and the later one is dropped",
    pre_existing_collision:
        "extension data provider would shadow an entry already present in the registry; first-write-wins keeps the existing one",
};

/// DUP-2 / TASK-1297 + FN-1 / TASK-1288: classify one (key, ext_name)
/// against the running `owners` map and emit the matching warn under the
/// chosen `policy`. Returns `true` if the caller should actually install
/// the entry into the shared registry. The four collision cases that
/// previously lived as inline match arms in two functions now live here
/// once, parameterised by `InsertPolicy` and the per-policy message table.
fn classify_and_warn_collision(
    owners: &mut std::collections::HashMap<String, Owner>,
    key: &str,
    ext_name: &'static str,
    audit: &AuditPolicy,
) -> bool {
    // PERF-3 / TASK-1349: lookup before allocating the owned key. The
    // common path is a fresh key per extension; only on the genuine-miss
    // branch do we materialise a `String` for `HashMap::insert`. This
    // avoids `key.to_string()` on every Occupied hit (the warning paths)
    // which dominated registration when an extension registers many ids.
    if let Some(prev) = owners.get_mut(key) {
        match prev {
            Owner::Extension(prev_name) if *prev_name != ext_name => {
                tracing::warn!(
                    target: "ops::registry",
                    kind = %audit.kind_field,
                    key = %key,
                    first = %prev_name,
                    second = %ext_name,
                    "{}",
                    audit.cross_ext_collision,
                );
                match audit.policy {
                    InsertPolicy::LastWriteWins => {
                        *prev = Owner::Extension(ext_name);
                        true
                    }
                    InsertPolicy::FirstWriteWins => false,
                }
            }
            Owner::PreExisting => {
                tracing::warn!(
                    target: "ops::registry",
                    kind = %audit.kind_field,
                    key = %key,
                    second = %ext_name,
                    "{}",
                    audit.pre_existing_collision,
                );
                match audit.policy {
                    InsertPolicy::LastWriteWins => {
                        *prev = Owner::Extension(ext_name);
                        true
                    }
                    InsertPolicy::FirstWriteWins => false,
                }
            }
            Owner::Extension(_) => {
                // PATTERN-1 / TASK-1350: same-extension duplicates are
                // drained upstream by `take_duplicate_inserts` before this
                // loop ever sees them, so each id in `local` is unique per
                // extension. If the invariant ever breaks, the debug build
                // surfaces it loudly; release builds preserve the previous
                // policy-dependent return for safety.
                debug_assert!(
                    false,
                    "in-extension duplicates must be drained upstream via take_duplicate_inserts (kind={}, key={}, extension={})",
                    audit.kind_field, key, ext_name,
                );
                matches!(audit.policy, InsertPolicy::LastWriteWins)
            }
        }
    } else {
        owners.insert(key.to_string(), Owner::Extension(ext_name));
        true
    }
}

/// DUP-3 / TASK-1371: shared in-extension duplicate-warn loop. Both the
/// command and data-provider registration paths drain the per-extension
/// `take_duplicate_inserts` audit; collapsing the warn template into one
/// helper keeps the structured-log shape (target, kind, key, extension)
/// and message phrasing in lockstep with the cross-extension audit.
fn warn_in_extension_duplicates<I, T>(duplicates: I, ext_name: &'static str, audit: &AuditPolicy)
where
    I: IntoIterator<Item = T>,
    T: std::fmt::Display,
{
    for dup in duplicates {
        tracing::warn!(
            target: "ops::registry",
            kind = %audit.kind_field,
            key = %dup,
            extension = ext_name,
            "{}",
            audit.in_ext_duplicate,
        );
    }
}

/// Collect all commands from registered extensions into a registry —
/// **last-write-wins** on duplicate command ids (CL-5 / TASK-0904).
///
/// SEC-31 / TASK-0402 (symmetric audit story with TASK-0350 for
/// `DataRegistry` — but with opposite resolution policy, see module doc):
/// extensions register into a shared `CommandRegistry` via `IndexMap::insert`.
/// Collisions surface as `tracing::warn!` through the shared
/// [`classify_and_warn_collision`] pipeline (DUP-2 / TASK-1297). The thin
/// shell here picks the [`InsertPolicy::LastWriteWins`] policy and message
/// table; structural duplication with [`register_extension_data_providers`]
/// is removed.
pub fn register_extension_commands(extensions: &[&dyn Extension], registry: &mut CommandRegistry) {
    let mut owners = snapshot_initial_owners(registry.keys().map(|k| k.to_string()));
    for ext in extensions {
        debug!(extension = ext.name(), action = "commands", "registering");
        let mut local = CommandRegistry::new();
        ext.register_commands(&mut local);
        warn_in_extension_duplicates(local.take_duplicate_inserts(), ext.name(), &COMMAND_AUDIT);
        for (id, spec) in local {
            if classify_and_warn_collision(&mut owners, id.as_ref(), ext.name(), &COMMAND_AUDIT) {
                registry.insert(id, spec);
            }
        }
    }
}

/// Collect all data providers from registered extensions —
/// **first-write-wins** on duplicate provider names (CL-5 / TASK-0904,
/// opposite to [`register_extension_commands`]; see module doc for the
/// rationale). Thin shell over the shared
/// [`classify_and_warn_collision`] pipeline; the only difference from the
/// command path is the chosen [`InsertPolicy`] and the message table.
pub fn register_extension_data_providers(
    extensions: &[&dyn Extension],
    registry: &mut DataRegistry,
) {
    let mut owners =
        snapshot_initial_owners(registry.provider_names().into_iter().map(str::to_string));
    for ext in extensions {
        debug!(
            extension = ext.name(),
            action = "data_providers",
            "registering"
        );
        let mut local = DataRegistry::new();
        ext.register_data_providers(&mut local);
        warn_in_extension_duplicates(
            local.take_duplicate_inserts(),
            ext.name(),
            &DATA_PROVIDER_AUDIT,
        );
        for (name, provider) in local {
            if classify_and_warn_collision(&mut owners, &name, ext.name(), &DATA_PROVIDER_AUDIT) {
                registry.register(name, provider);
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

/// PERF-1 / TASK-1380: same as [`build_data_registry`] but consumes a
/// pre-collected `compiled` vector so the show path can build the
/// schema-lookup registry without re-walking `EXTENSION_REGISTRY`.
pub fn build_data_registry_from(
    compiled: Vec<(&'static str, Box<dyn Extension>)>,
    config: &Config,
    workspace_root: &Path,
) -> anyhow::Result<DataRegistry> {
    let exts = builtin_extensions_from(compiled, config, workspace_root)?;
    let mut registry = DataRegistry::new();
    register_extension_data_providers(&as_ext_refs(&exts), &mut registry);
    Ok(registry)
}
