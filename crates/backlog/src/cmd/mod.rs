//! Command handlers: plain option structs (no clap), writer-injectable
//! cores following the CLI crate's `_to` idom.
//!
//! Errors are `anyhow` with the file path attached (ERR-13); the CLI prints
//! them as `ops: error: …`.

pub mod cleanup;
pub mod create;
pub mod edit;
pub mod list;
pub mod search;
pub mod view;

pub use cleanup::{run_cleanup, CleanupOptions};
pub use create::{run_create, CreateOptions};
pub use edit::{run_edit, EditOptions};
pub use list::{run_list, ListOptions};
pub use search::{run_search, SearchOptions};
pub use view::{run_view, ViewOptions};
