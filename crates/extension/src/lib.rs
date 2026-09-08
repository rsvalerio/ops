//! Extension trait and registries: `CommandRegistry`, `DataRegistry`, Context.

// UNSAFE-12 / TASK-2104: this crate holds no hand-written `unsafe` and must
// stay that way. Production builds carry the full `forbid` (unliftable, so a
// well-meaning scoped `#[allow(unsafe_code)]` cannot reintroduce unsafe).
// Test builds downgrade to `deny` because the `factory:` arms of
// `impl_extension!` (used only by this crate's own test suite) expand, via
// `linkme::distributed_slice`, to `#[link_section]` registry statics —
// unsafe tokens the compiler counts into this crate, which `forbid` cannot
// excuse at all. The `#[allow(unsafe_code)]` covering exactly that
// expansion is emitted by the macro itself onto the generated static (see
// `macros.rs`; rustc ignores `allow` applied to a macro invocation).
// Crate-root attribute rather than a `[lints.rust]` table because ARCH-11
// centralizes lint levels in `[workspace.lints]` and Cargo cannot merge a
// per-crate lints table with `workspace = true` inheritance. Verified:
// `cargo check -p ops-extension [--features duckdb]` clean under the
// forbid, `cargo clippy -p ops-extension --all-targets` clean, and any
// hand-written unsafe block fails the build.
#![cfg_attr(not(test), forbid(unsafe_code))]
#![cfg_attr(test, deny(unsafe_code))]

mod data;
mod error;
mod extension;
mod macros;

pub use data::{
    Context, DataField, DataProvider, DataProviderSchema, DataRegistry, Deadline,
    DEFAULT_PROVIDER_BUDGET,
};
pub use error::{DataProviderError, SharedError};
pub use extension::{
    sort_compiled_extensions, CommandRegistry, Extension, ExtensionFactory, ExtensionInfo,
    ExtensionType, Stack, EXTENSION_REGISTRY,
};

#[cfg(feature = "duckdb")]
pub use data::DuckDbHandle;

/// Duplicate-registration policy for the two registries this crate owns.
///
/// CL-3 / TASK-1872: this is the single place the policy split is explained.
/// [`DataRegistry::register`] and [`CommandRegistry::insert`] link here rather
/// than each carrying its own copy of the reasoning, which had drifted into
/// two divergent essays.
///
/// | Registry | Policy | Return value | Why |
/// |---|---|---|---|
/// | [`DataRegistry`] | **first-write-wins** | `Some(rejected)` on collision | Providers are security-trusted built-ins (`identity`, `metadata`). SEC-31 / TASK-0350: a later extension must not be able to shadow one by registering the same name. |
/// | [`CommandRegistry`] | **last-write-wins** | `Some(previous)` on collision | Shadowing is the feature. Config-defined `[commands.*]` are merged after extension commands specifically so a user can override them. |
///
/// Both record the colliding key on a per-instance audit trail that the CLI
/// wiring layer drains via `take_duplicate_inserts` and reports as one
/// `tracing::warn!` per entry. The audit trail is the *aggregated* signal; the
/// return value is the per-call one. `DataRegistry::register` is additionally
/// `#[must_use]` because a dropped provider surfaces much later as an
/// unrelated `NotFound`, whereas a shadowed command at least still runs
/// something.
pub mod registry_duplicate_policy {}

#[cfg(test)]
mod tests;
