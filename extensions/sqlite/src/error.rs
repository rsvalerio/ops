//! Database error type for `Sqlite`.

use thiserror::Error;

/// Database operations error.
// READ-10 / TASK-1873: no enum-wide `#[allow(dead_code)]`. A container-level
// allow hides *future* dead code too — a variant nobody constructs would stay
// invisible forever. Any variant that genuinely has no producer yet carries
// its own `#[expect(dead_code, reason = "…")]`, which deletes itself as soon
// as a producer appears.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DbError {
    /// ERR-5 / TASK-1214: `PoisonError`'s Display embeds the panic payload from
    /// arbitrary user-supplied callbacks, which can contain newlines, ANSI
    /// escapes, or other operator-controlled bytes. Rendering via `{0:?}`
    /// (Debug) escapes control characters (`\n`, `\u{1b}`, …) so the captured
    /// string can flow safely to logs, JSON error responses, and anyhow
    /// `.context()` chains without log-injection / payload-tampering risk.
    #[error("database mutex poisoned: {0:?}")]
    MutexPoisoned(String),

    /// An error raised by the underlying `rusqlite` library.
    #[error("database error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// A filesystem error while opening or reading database artifacts.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// A query failed; `context` names what was being run.
    #[error("{context}: {source}")]
    QueryFailed {
        /// What the failing query was doing, for the operator message.
        context: String,
        /// The underlying `rusqlite` error.
        #[source]
        source: rusqlite::Error,
    },

    /// A JSON payload failed to (de)serialize.
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// A row count exceeded the `i64` column range.
    #[error("record count overflow: {0} exceeds i64::MAX")]
    RecordCountOverflow(u64),

    /// A table reported a negative row count, which no table can have.
    #[error("invalid record count for {table}: {count} (must be non-negative)")]
    InvalidRecordCount {
        /// The table whose count was read.
        table: String,
        /// The negative count that was read.
        count: i64,
    },

    /// A database path is not valid UTF-8 and cannot be persisted.
    #[error("path is not valid UTF-8 (cannot persist to data_sources): {0:?}")]
    NonUtf8Path(std::ffi::OsString),

    /// READ-5 / TASK-1867: the ingest pipeline stages JSON next to the
    /// database file, so it needs a real filesystem path. `:memory:` is a
    /// SQLite connection string, not a path — appending `.ingest` to it
    /// produced the *relative* `:memory:.ingest`, which the pipeline then
    /// created inside whatever the process working directory happened to be
    /// (and once got committed to this repository).
    #[error("database {0:?} is not file-backed; the ingest pipeline needs a real database path")]
    NotFileBacked(std::path::PathBuf),

    /// A generated SQL statement failed shared validation.
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
    /// Builds a [`DbError::QueryFailed`] from a context string and the
    /// underlying `rusqlite` error.
    #[must_use = "return the constructed error; building it reports nothing"]
    pub fn query_failed(context: impl Into<String>, source: rusqlite::Error) -> Self {
        Self::QueryFailed {
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
            rusqlite::Error::InvalidParameterName("test".into()),
        );
        assert!(err.to_string().contains("test_op"));
    }
}
