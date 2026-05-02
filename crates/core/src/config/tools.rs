//! Config types for tool specifications.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ToolSpec {
    Simple(String),
    Extended(ExtendedToolSpec),
}

impl ToolSpec {
    pub fn description(&self) -> &str {
        match self {
            ToolSpec::Simple(desc) => desc,
            ToolSpec::Extended(ext) => &ext.description,
        }
    }

    pub fn rustup_component(&self) -> Option<&str> {
        match self {
            ToolSpec::Simple(_) => None,
            ToolSpec::Extended(ext) => ext.rustup_component.as_deref(),
        }
    }

    pub fn package(&self) -> Option<&str> {
        match self {
            ToolSpec::Simple(_) => None,
            ToolSpec::Extended(ext) => ext.package.as_deref(),
        }
    }

    pub fn source(&self) -> ToolSource {
        match self {
            ToolSpec::Simple(_) => ToolSource::Cargo,
            ToolSpec::Extended(ext) => ext.source,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields, rename_all = "kebab-case")]
pub struct ExtendedToolSpec {
    pub description: String,
    #[serde(default)]
    pub rustup_component: Option<String>,
    #[serde(default)]
    pub package: Option<String>,
    #[serde(default)]
    pub source: ToolSource,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolSource {
    #[default]
    Cargo,
    System,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_tool_spec(toml_value: &str) -> ToolSpec {
        let doc = format!("tool = {}", toml_value);
        let parsed: toml::Value = toml::from_str(&doc).unwrap();
        let value = parsed.get("tool").unwrap().clone();
        ToolSpec::deserialize(value).unwrap()
    }

    #[test]
    fn parse_simple_tool() {
        let spec = parse_tool_spec(r#""A description""#);
        assert_eq!(spec.description(), "A description");
        assert!(spec.rustup_component().is_none());
        assert!(spec.package().is_none());
        assert!(matches!(spec.source(), ToolSource::Cargo));
    }

    #[test]
    fn parse_extended_tool() {
        let spec = parse_tool_spec(
            r#"{ description = "A tool", rustup-component = "llvm-tools-preview" }"#,
        );
        assert_eq!(spec.description(), "A tool");
        assert_eq!(spec.rustup_component(), Some("llvm-tools-preview"));
    }

    #[test]
    fn parse_extended_tool_with_source() {
        let spec = parse_tool_spec(r#"{ description = "System tool", source = "system" }"#);
        assert!(matches!(spec.source(), ToolSource::System));
    }
}
