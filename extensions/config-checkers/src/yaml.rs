//! Parse-only YAML validation via `saphyr`. Accepts multi-document streams
//! (the parser returns one node per `---`-separated document); we only care
//! that all documents parse.

use saphyr::{LoadableYamlNode, Yaml};

use crate::CheckError;

/// Validate that `bytes` parses as YAML (one or more documents).
///
/// # Errors
/// Returns [`CheckError::InvalidUtf8`] when the bytes are not UTF-8, or
/// [`CheckError::Parse`] when the YAML parser rejects the input.
pub fn check_yaml(bytes: &[u8]) -> Result<(), CheckError> {
    let text = std::str::from_utf8(bytes).map_err(CheckError::InvalidUtf8)?;
    Yaml::load_from_str(text)
        .map(|_| ())
        .map_err(|e| CheckError::Parse(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_yaml_passes() {
        assert!(check_yaml(b"a: 1\nb:\n  - 2\n  - 3\n").is_ok());
    }

    #[test]
    fn multi_doc_yaml_passes() {
        assert!(check_yaml(b"a: 1\n---\nb: 2\n").is_ok());
    }

    #[test]
    fn invalid_yaml_fails() {
        let err = check_yaml(b"a: : :\n").unwrap_err();
        assert!(!err.to_string().is_empty());
    }
}
