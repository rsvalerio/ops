#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]
// Tests mutate the process environment and probe processes through libc.
// Production unsafe is not covered: each site carries its own scoped
// `#[expect(unsafe_code)]`.
#![cfg_attr(test, allow(unsafe_code))]

pub mod command;
pub mod display;
pub mod terminal;

#[cfg(feature = "test-support")]
pub mod test_support;
