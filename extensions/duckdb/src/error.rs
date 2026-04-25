//! Database error type for DuckDb.

use thiserror::Error;

/// Database operations error.
#[derive(Debug, Error)]
#[allow(dead_code)]
pub enum DbError {
    #[error("database mutex poisoned: {0}")]
    MutexPoisoned(String),

    #[error("database error: {0}")]
    DuckDb(#[from] duckdb::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("{context}: {source}")]
    QueryFailed {
        context: String,
        #[source]
        source: duckdb::Error,
    },

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("record count overflow: {0} exceeds i64::MAX")]
    RecordCountOverflow(u64),

    #[error("invalid record count for {table}: {count} (must be non-negative)")]
    InvalidRecordCount { table: String, count: i64 },

    #[error("path is not valid UTF-8 (cannot persist to data_sources): {0:?}")]
    NonUtf8Path(std::ffi::OsString),

    #[error("SQL validation failed: {0}")]
    SqlValidation(#[from] crate::sql::SqlError),

    /// Subprocess exceeded its bounded-wait deadline.
    ///
    /// Distinct from [`DbError::Io`] so retry policies and operator messages
    /// can branch on a real timeout vs. a generic IO failure.
    #[error("{label} timed out after {timeout_secs}s")]
    Timeout { label: String, timeout_secs: u64 },
}

impl DbError {
    pub fn query_failed(context: impl Into<String>, source: duckdb::Error) -> Self {
        DbError::QueryFailed {
            context: context.into(),
            source,
        }
    }
}

/// Result alias for database operations.
pub type DbResult<T> = Result<T, DbError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_error_mutex_poisoned_message() {
        let err = DbError::MutexPoisoned("test panic".to_string());
        assert!(err.to_string().contains("mutex poisoned"));
        assert!(err.to_string().contains("test panic"));
    }

    #[test]
    fn db_error_serialization_message() {
        let json = serde_json::from_str::<serde_json::Value>("not valid json");
        let err = json.unwrap_err();
        let db_err = DbError::Serialization(err);
        assert!(db_err.to_string().contains("serialization error"));
    }

    #[test]
    fn db_error_record_count_overflow_message() {
        let err = DbError::RecordCountOverflow(u64::MAX);
        let msg = err.to_string();
        assert!(msg.contains("record count overflow"));
        assert!(msg.contains(&u64::MAX.to_string()));
    }

    #[test]
    fn db_error_query_failed_context() {
        let err = DbError::query_failed(
            "test_op",
            duckdb::Error::InvalidParameterName("test".into()),
        );
        assert!(err.to_string().contains("test_op"));
    }
}
