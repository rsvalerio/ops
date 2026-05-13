//! Extension glue: resolve stack, collect compiled-in extensions, register commands/data providers.
//!
//! This module was originally an 832-line file mixing discovery,
//! registration, and audit machinery. Split into:
//!
//! - [`discovery`] — stack resolution, compiled-in extension enumeration,
//!   stack/config filtering, ref-conversion helpers.
//! - [`registration`] — symmetric command + data-provider registration with
//!   the shared [`Owner`]-tracked collision audit pipeline, plus
//!   [`build_data_registry`] convenience.
//!
//! Public surface is unchanged: callers under `crate::registry::*` get the
//! same names via the re-exports below.

mod discovery;
mod registration;

#[cfg(test)]
mod tests;

#[cfg(test)]
pub use discovery::collect_extension_info;
pub use discovery::{as_ext_refs, builtin_extensions, collect_compiled_extensions};
pub use registration::{
    build_data_registry, build_data_registry_from, register_extension_commands,
    register_extension_data_providers,
};
