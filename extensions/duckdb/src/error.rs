//! Database error type for DuckDb.

use thiserror::Error;

/// Database operations error.
#[derive(Debug, Error)]
#[allow(dead_code)]
#[non_exhaustive]
pub enum DbError {
    /// ERR-5 / TASK-1214: PoisonError's Display embeds the panic payload from
    /// arbitrary user-supplied callbacks, which can contain newlines, ANSI
    /// escapes, or other operator-controlled bytes. Rendering via `{0:?}`
    /// (Debug) escapes control characters (`\n`, `\u{1b}`, …) so the captured
    /// string can flow safely to logs, JSON error responses, and anyhow
    /// `.context()` chains without log-injection / payload-tampering risk.
    #[error("database mutex poisoned: {0:?}")]
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
    /// Produced by `MetadataIngestor::collect` when `run_cargo_metadata`
    /// exceeds its bounded wait. Distinct from [`DbError::Io`] so retry
    /// policies and operator messages can branch on a real timeout vs.
    /// a generic IO failure.
    #[error("{label} timed out after {timeout_secs}s")]
    Timeout { label: String, timeout_secs: u64 },

    /// External collection/validation error that is not IO or serialization.
    ///
    /// Used by `collect_tokei`, `collect_coverage`, `check_metadata_output`,
    /// and similar callers that return `anyhow::Error` — wrapping these as
    /// `DbError::Io` misleads operators into investigating filesystem problems
    /// when the real cause may be a parse failure, missing tool, or timeout.
    ///
    /// ERR-2 / TASK-1209: carries the wrapped `anyhow::Error` via `#[source]`
    /// so consumers walking `Error::source()` recover the cause graph.
    /// Display renders the alternate-format chain via `{0:#}` so log output
    /// remains identical to the previous flattened-string variant.
    #[error("external error: {0:#}")]
    External(#[source] anyhow::Error),
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

    /// ERR-5 / TASK-1214: a poisoned-mutex payload containing newlines and
    /// ANSI escapes (i.e. arbitrary user-supplied panic content) must not
    /// be forwarded verbatim into the Display body. Debug-formatting the
    /// captured string escapes the control bytes so the rendered error is
    /// safe to log without log-injection / payload-tampering risk.
    #[test]
    fn db_error_mutex_poisoned_escapes_control_bytes() {
        let payload = "panic at line 42\n\u{1b}[31mFAKE ERROR\u{1b}[0m";
        let err = DbError::MutexPoisoned(payload.to_string());
        let rendered = err.to_string();
        assert!(
            !rendered.contains('\n'),
            "rendered Display must not contain raw newline; got: {rendered:?}"
        );
        assert!(
            !rendered.contains('\u{1b}'),
            "rendered Display must not contain raw ESC byte; got: {rendered:?}"
        );
        // Escaped form should still carry the original substrings so the
        // operator can recover the panic payload by reading the log.
        assert!(
            rendered.contains("\\n"),
            "expected escaped newline; got: {rendered:?}"
        );
        assert!(
            rendered.contains("FAKE ERROR"),
            "payload text preserved; got: {rendered:?}"
        );
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
