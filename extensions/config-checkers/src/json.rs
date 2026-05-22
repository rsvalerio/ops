//! Parse-only JSON validation. Strict mode uses `serde_json`; lenient mode
//! (`allow_json5 = true`) uses the `json5` crate, which is a strict superset
//! of JSONC and additionally accepts unquoted keys, single-quoted strings,
//! hex/`Infinity`/`NaN` numbers, and other JSON5 extensions.

use crate::CheckError;

/// Validate that `bytes` parses as JSON (or JSON5 when `allow_json5`).
///
/// # Errors
/// Returns [`CheckError::InvalidUtf8`] when JSON5 mode is on and the input
/// is not UTF-8, or [`CheckError::Parse`] when the parser rejects the input.
pub fn check_json(bytes: &[u8], allow_json5: bool) -> Result<(), CheckError> {
    if allow_json5 {
        let text = std::str::from_utf8(bytes).map_err(CheckError::InvalidUtf8)?;
        json5::from_str::<serde_json::Value>(text)
            .map(|_| ())
            .map_err(|e| CheckError::Parse(e.to_string()))
    } else {
        serde_json::from_slice::<serde_json::Value>(bytes)
            .map(|_| ())
            .map_err(|e| CheckError::Parse(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_json_passes_strict() {
        assert!(check_json(br#"{"a": 1, "b": [true, null]}"#, false).is_ok());
    }

    #[test]
    fn invalid_json_fails_strict() {
        let err = check_json(br#"{"a": }"#, false).unwrap_err();
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn json5_extensions_fail_strict_pass_with_flag() {
        let bytes = br#"{ /* note */ "a": 1, }"#;
        assert!(check_json(bytes, false).is_err());
        assert!(check_json(bytes, true).is_ok());

        // JSON5-only construct (unquoted key) — accepted under allow_json5 to
        // document that the flag is JSON5, not strict JSONC.
        let json5_only = br"{ a: 1 }";
        assert!(check_json(json5_only, false).is_err());
        assert!(check_json(json5_only, true).is_ok());
    }
}
