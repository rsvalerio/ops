#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

pub mod config;
pub mod expand;
pub mod output;
pub mod paths;
pub mod project_identity;
pub mod report;
pub mod serde_defaults;
pub mod stack;
pub mod style;
pub mod subprocess;
pub(crate) mod sync;
pub mod table;
pub mod text;
pub mod ui;

#[cfg(any(test, feature = "test-support"))]
pub mod test_utils;
