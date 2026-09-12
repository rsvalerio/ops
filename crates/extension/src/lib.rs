//! Extension trait and registries: `CommandRegistry`, `DataRegistry`, Context.

// UNSAFE-12 / TASK-2104: this crate holds no hand-written `unsafe` and must
// stay that way. The full `forbid` is unliftable, so a well-meaning scoped
// `#[allow(unsafe_code)]` cannot reintroduce unsafe. TEST-3 / TASK-2091 moved
// the test suite to `tests/`, so the `factory:` arms of `impl_extension!`
// that expand — via `linkme::distributed_slice` — to `#[link_section]`
// registry statics (unsafe tokens the compiler counts into the invoking
// crate) are no longer expanded anywhere under `cfg(test)` in this crate,
// and the former test-build downgrade to `deny` is unnecessary. The macro
// itself emits `#[allow(unsafe_code)]` onto the generated static for
// invoking crates that only deny (see `macros.rs`; rustc ignores `allow`
// applied to a macro invocation).
// Crate-root attribute rather than a `[lints.rust]` table because ARCH-11
// centralizes lint levels in `[workspace.lints]` and Cargo cannot merge a
// per-crate lints table with `workspace = true` inheritance. Verified:
// `cargo check -p ops-extension [--features sqlite]` clean under the
// forbid, `cargo clippy -p ops-extension --all-targets` clean, and any
// hand-written unsafe block fails the build.
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
    ExtensionType, Stack, EXTENSION_REGISTRY,
};

#[cfg(feature = "sqlite")]
pub use db_handle::SqliteHandle;
