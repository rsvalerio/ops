//! Foundation crate shared by every `ops` binary, crate, and extension.
//!
//! ops-core holds the stack-agnostic layers the rest of the workspace builds
//! on: configuration loading and merging ([`config`]), stack detection
//! ([`stack`]), `${VAR}` expansion ([`expand`]), subprocess running
//! ([`subprocess`]), size-capped file reads ([`bounded_read`]), output and
//! terminal handling ([`output`], [`ui`], [`style`], [`table`]), the project
//! identity model rendered by `ops about` ([`project_identity`], [`report`]),
//! and small text/path/serde utilities ([`text`], [`paths`],
//! [`serde_defaults`]). Extensions and the CLI consume these through
//! `ops_extension`/`ops_runner`; nothing here knows about a specific stack.

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

pub mod bounded_read;
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
pub mod sync;
pub mod table;
pub mod text;
pub mod ui;

#[cfg(any(test, feature = "test-support"))]
#[expect(
    unsafe_code,
    reason = "test support: process env and cwd guards, and a geteuid probe"
)]
pub mod test_utils;
