//! Minimal backlog.md-compatible task manager: the task create/edit/list/view
//! and search subset the ops skills use, operating directly on the `.backlog`
//! markdown tree.
//!
//! The file shapes mirror what `backlog task ...` (backlog.md CLI v1.51.0)
//! reads and writes, so files produced here are indistinguishable from
//! CLI-created ones when re-read by that CLI — and every file the CLI has
//! already written (~2073 in this repository alone) parses here unchanged.

pub mod clock;
pub mod cmd;
pub mod config;
pub mod model;
pub mod render;
pub mod store;
