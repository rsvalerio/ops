//! Extension trait and registries: `CommandRegistry`, `DataRegistry`, Context.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

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
