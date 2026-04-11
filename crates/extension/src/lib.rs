//! Extension trait and registries: CommandRegistry, DataRegistry, Context.

mod data;
mod error;
mod extension;
#[allow(clippy::module_inception)]
mod macros;

pub use data::{Context, DataField, DataProvider, DataProviderSchema, DataRegistry};
pub use error::{DataProviderError, SharedError};
pub use extension::{
    CommandRegistry, Extension, ExtensionFactory, ExtensionInfo, ExtensionType, Stack,
    EXTENSION_REGISTRY,
};

#[cfg(feature = "duckdb")]
pub use data::DuckDbHandle;

#[cfg(test)]
mod tests;
