//! Parse-only JSON validation. Strict mode uses `serde_json`; JSONC mode
//! (`allow_jsonc = true`) uses the `json5` crate to also accept comments
//! and trailing commas.

pub fn check_json(bytes: &[u8], allow_jsonc: bool) -> Result<(), String> {
    if allow_jsonc {
        let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
        json5::from_str::<serde_json::Value>(text)
            .map(|_| ())
            .map_err(|e| e.to_string())
    } else {
        serde_json::from_slice::<serde_json::Value>(bytes)
            .map(|_| ())
            .map_err(|e| e.to_string())
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
        assert!(!err.is_empty());
    }

    #[test]
    fn jsonc_fails_strict_passes_with_flag() {
        let bytes = br#"{ /* note */ "a": 1, }"#;
        assert!(check_json(bytes, false).is_err());
        assert!(check_json(bytes, true).is_ok());
    }
}
