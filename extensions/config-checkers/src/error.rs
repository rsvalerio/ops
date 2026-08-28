//! The error type shared by [`crate::json`] and [`crate::yaml`].

use std::fmt;
use std::str::Utf8Error;

/// A validator-imposed bound was exceeded.
///
/// Distinct from a parser error: the input may be perfectly well-formed and
/// still be rejected because validating it would cost more than the checker
/// is willing to spend (`SEC-33`). Surfaced through
/// [`CheckError::Parse`] so callers keep a single failure shape.
#[derive(Debug, Clone, Copy)]
pub struct LimitExceeded {
    /// What was bounded, e.g. `"nesting depth"`.
    pub what: &'static str,
    /// The bound that was exceeded.
    pub limit: u64,
}

impl fmt::Display for LimitExceeded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "input exceeds the {} limit of {}", self.what, self.limit)
    }
}

impl std::error::Error for LimitExceeded {}

/// Typed parse error for [`crate::json::check_json`] and
/// [`crate::yaml::check_yaml`].
#[derive(Debug)]
pub enum CheckError {
    /// File bytes were not valid UTF-8 (only meaningful for parsers that
    /// require a `&str`).
    InvalidUtf8(Utf8Error),
    /// Underlying parser rejected the input, or a checker bound
    /// ([`LimitExceeded`]) was hit. The concrete parser error types
    /// (`serde_json::Error`, `json5::Error`, `saphyr::ScanError`) diverge, so
    /// the cause is type-erased rather than rendered to a `String` — that
    /// keeps `source()` walkable and preserves the positional data
    /// (`serde_json::Error::line`, `saphyr::Marker`) a structured report
    /// would need.
    Parse(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl CheckError {
    /// Wrap a parser error (or a [`LimitExceeded`]) as [`CheckError::Parse`].
    pub(crate) fn parse<E>(err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        Self::Parse(Box::new(err))
    }
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUtf8(e) => write!(f, "invalid UTF-8: {e}"),
            // Delegates rather than wrapping, so the rendered line stays
            // byte-identical to the parser's own message: this variant adds
            // no information of its own, and interpolating the source into a
            // longer sentence would be the `ERR-9` duplication.
            Self::Parse(e) => fmt::Display::fmt(e, f),
        }
    }
}

impl std::error::Error for CheckError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::InvalidUtf8(e) => Some(e),
            Self::Parse(e) => Some(&**e),
        }
    }
}
