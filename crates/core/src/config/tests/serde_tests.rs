use super::*;

#[test]
fn extension_config_default_is_none() {
    let config = ExtensionConfig::default();
    assert!(config.enabled.is_none());
}

#[test]
fn parse_config_with_extensions() {
    let toml = r#"
[output]
theme = "classic"

[extensions]
enabled = ["ops-db", "metadata"]
"#;
    let overlay: ConfigOverlay = toml::from_str(toml).expect("should parse");
    assert!(overlay.extensions.is_some());
    let ext = overlay.extensions.unwrap();
    assert_eq!(
        ext.enabled,
        Some(vec!["ops-db".to_string(), "metadata".to_string()])
    );
}

#[test]
fn parse_exec_command_with_aliases() {
    let toml_str = r#"
[commands.install]
program = "cargo"
args = ["install"]
aliases = ["i", "inst"]
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    let spec = config.commands.get("install").unwrap();
    assert_eq!(spec.aliases(), &["i", "inst"]);
}

#[test]
fn parse_exec_command_with_alias_key() {
    let toml_str = r#"
[commands.install]
program = "cargo"
args = ["install"]
alias = ["i"]
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    let spec = config.commands.get("install").unwrap();
    assert_eq!(spec.aliases(), &["i"]);
}

#[test]
fn parse_composite_command_with_aliases() {
    let toml_str = r#"
[commands.verify]
commands = ["fmt", "clippy"]
aliases = ["v", "check"]
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    let spec = config.commands.get("verify").unwrap();
    assert_eq!(spec.aliases(), &["v", "check"]);
}

#[test]
fn resolve_alias_finds_command() {
    let toml_str = r#"
[commands.install]
program = "cargo"
args = ["install"]
aliases = ["i", "inst"]

[commands.build]
program = "cargo"
args = ["build"]
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    assert_eq!(config.resolve_alias("i"), Some("install"));
    assert_eq!(config.resolve_alias("inst"), Some("install"));
    assert_eq!(config.resolve_alias("build"), None);
    assert_eq!(config.resolve_alias("unknown"), None);
}

#[test]
fn no_aliases_by_default() {
    let toml_str = r#"
[commands.build]
program = "cargo"
args = ["build"]
"#;
    let config: Config = toml::from_str(toml_str).unwrap();
    let spec = config.commands.get("build").unwrap();
    assert!(spec.aliases().is_empty());
}
