//! Minimal backlog.md-compatible task manager: the task create/edit/list/view,
//! search, and cleanup subset the ops skills use, operating directly on the
//! `.backlog` markdown tree.
//!
//! The file shapes mirror what `backlog task ...` (backlog.md CLI v1.51.0)
//! reads and writes, so files produced here are indistinguishable from
//! CLI-created ones when re-read by that CLI — and every file the CLI has
//! already written (~2073 in this repository alone) parses here unchanged.

// UNSAFE-12 / TASK-2102: this crate holds no `unsafe` and must stay that way.
// `forbid` (not `deny`) so a later scoped `#[allow(unsafe_code)]` cannot lift
// it. Crate-root attribute rather than a `[lints.rust]` table because ARCH-11
// centralizes lint levels in `[workspace.lints]` and Cargo cannot merge a
// per-crate lints table with `workspace = true` inheritance.
#![forbid(unsafe_code)]

pub mod clock;
pub mod cmd;
pub mod config;
pub mod model;
pub mod render;
pub mod store;
