#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

pub mod command;
pub mod display;
pub mod terminal;

#[cfg(feature = "test-support")]
pub mod test_support;
