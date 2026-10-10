//! Extension trait and registries: `CommandRegistry`, `DataRegistry`, Context.

// This crate holds no hand-written `unsafe` and must stay that way. `forbid`
// rather than `deny` so a scoped `#[allow(unsafe_code)]` cannot reintroduce
// it. The `factory:` arms of `impl_extension!` expand — via
// `linkme::distributed_slice` — to `#[link_section]` registry statics, unsafe
// tokens the compiler counts into the invoking crate, so they must never be
// expanded inside this crate (its tests live in `tests/`, a separate crate).
// The macro emits `#[allow(unsafe_code)]` onto the generated static for
// invoking crates that only deny (see `macros.rs`).
// Crate-root attribute rather than a `[lints.rust]` table because Cargo
// cannot merge a per-crate lints table with `workspace = true` inheritance.
#![forbid(unsafe_code)]

mod context;
mod data;
mod db_handle;
mod deadline;
mod error;
mod extension;
mod macros;

pub use context::Context;
pub use data::{DataField, DataProvider, DataProviderSchema, DataRegistry};
pub use deadline::{Deadline, DEFAULT_PROVIDER_BUDGET};
pub use error::{DataProviderError, SharedError};
pub use extension::{
    sort_compiled_extensions, CommandRegistry, Extension, ExtensionFactory, ExtensionInfo,
    ExtensionType, EXTENSION_REGISTRY,
};

#[cfg(feature = "sqlite")]
pub use db_handle::SqliteHandle;
