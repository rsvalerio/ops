//! Table creation, sidecar I/O, and data pipeline helpers, split into
//! per-concern submodules and re-exported through `super::sql`.

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
    execute_json_load, in_transaction, load_json_string, query_rows_to_json, source_is_loaded,
    table_has_data, CreateTableSql, CreateViewSql, JsonColumn, JsonColumnType, JsonLoadShape,
    JsonTableLoad,
};

pub(crate) use sidecar::check_workspace_path;
pub(super) use sql::table_exists;
