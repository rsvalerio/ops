//! Table creation, sidecar I/O, and data pipeline helpers.
//!
//! ARCH-1 / TASK-1146: split the previous 1500+ line module into four
//! per-concern submodules. The crate-level `pub use` surface in
//! `super::sql` is preserved so downstream callers see no churn.

pub(super) mod dir;
pub(super) mod orchestrator;
pub(super) mod sidecar;
pub(super) mod sql;

pub use dir::{checksum_file, data_dir_for_db, default_data_dir, default_db_path, external_err};
pub use orchestrator::provide_via_ingestor;
pub use sidecar::{
    read_workspace_sidecar, remove_workspace_sidecar, sidecar_path, write_workspace_sidecar,
    MAX_SIDECAR_BYTES,
};
pub use sql::{create_table_from_json_sql, query_rows_to_json, table_has_data};

pub(super) use sql::table_exists;

/// DUP-032: Macro to generate standard path validation tests for `*_create_sql` functions.
///
/// Generates four tests: valid path, path with spaces, injection rejection, traversal rejection.
#[cfg(any(test, feature = "test-helpers"))]
#[macro_export]
macro_rules! test_create_sql_validation {
    ($create_fn:path, $file_name:expr) => {
        #[test]
        fn create_sql_valid_path() {
            let path = std::path::PathBuf::from(concat!("/home/user/data/", $file_name));
            let result = $create_fn(&path);
            assert!(result.is_ok());
            let sql = result.unwrap();
            assert!(sql.contains("read_json_auto"));
            assert!(sql.contains($file_name));
        }

        #[test]
        fn create_sql_accepts_path_with_spaces() {
            let path = std::path::PathBuf::from(concat!("/home/my user/project dir/", $file_name));
            let result = $create_fn(&path);
            assert!(result.is_ok());
            assert!(result.unwrap().contains("my user/project dir"));
        }

        #[test]
        fn create_sql_rejects_injection() {
            let path = std::path::PathBuf::from("/path;DROP TABLE users;");
            let result = $create_fn(&path);
            assert!(result.is_err());
        }

        #[test]
        fn create_sql_rejects_traversal() {
            let path = std::path::PathBuf::from("../../../etc/passwd");
            let result = $create_fn(&path);
            assert!(result.is_err());
        }
    };
}
