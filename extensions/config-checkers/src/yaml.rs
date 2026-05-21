//! Parse-only YAML validation via `saphyr`. Accepts multi-document streams
//! (the parser returns one node per `---`-separated document); we only care
//! that all documents parse.

use saphyr::{LoadableYamlNode, Yaml};

pub fn check_yaml(bytes: &[u8]) -> Result<(), String> {
    let text = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    Yaml::load_from_str(text)
        .map(|_| ())
        .map_err(|e| e.to_string())
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
        assert!(!err.is_empty());
    }
}
