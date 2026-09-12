//! Table creation, sidecar I/O, and data pipeline helpers.
//!
//! ARCH-1 / TASK-1146: split the previous 1500+ line module into four
//! per-concern submodules. The crate-level `pub use` surface in
//! `super::sql` is preserved so downstream callers see no churn.

pub(super) mod dir;
pub(super) mod orchestrator;
pub(super) mod sidecar;
pub(super) mod sql;

pub use dir::{data_dir_for_db, default_db_path, external_err, IngestDir};
pub use orchestrator::provide_via_ingestor;
pub use sidecar::{
    read_workspace_sidecar, remove_workspace_sidecar, sidecar_name, write_workspace_sidecar,
    MAX_SIDECAR_BYTES,
};
pub use sql::{
    execute_json_load, load_json_string, query_rows_to_json, table_has_data, CreateTableSql,
    CreateViewSql, JsonColumn, JsonColumnType, JsonLoadShape, JsonTableLoad,
};

pub(super) use sql::table_exists;
