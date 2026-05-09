use super::*;

#[test]
fn default_ops_file_exists_and_deserializes() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/.default.ops.toml");
    assert!(
        path.exists(),
        "src/.default.ops.toml must exist in the repo (base config; stack commands in .default.<stack>.ops.toml)"
    );
    let c: Config = toml::from_str(default_ops_toml()).expect("default config must deserialize");
    assert_eq!(c.output.theme, "classic");
    // READ-5 / TASK-1219: columns defaults to the AUTO sentinel (0) so
    // deserialisation is terminal-independent; render-time `resolve_columns()`
    // probes the live terminal.
    assert_eq!(c.output.columns, 0);
    assert!(c.output.resolve_columns() > 0);
    assert!(c.output.show_error_detail);
    assert_eq!(c.output.stderr_tail_lines, 5);
    // Commands are provided by stack defaults (from .default.<stack>.ops.toml), not the base file.
    assert!(
        c.commands.is_empty(),
        "default config should have no commands; stack defaults are loaded separately"
    );
}

#[test]
fn init_template_with_rust_stack_includes_commands() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
    )
    .expect("write Cargo.toml");
    let sections = InitSections::from_flags(true, true, true);
    let content = init_template(dir.path(), &sections).expect("init_template must succeed");
    assert!(
        content.contains("[commands.build]"),
        "Rust stack init template must include [commands.build]"
    );
    assert!(
        content.contains("[commands.clippy]"),
        "Rust stack init template must include [commands.clippy]"
    );
    assert!(
        content.contains("[commands.verify]"),
        "Rust stack init template must include [commands.verify]"
    );
    assert!(
        content.contains("stack = \"rust\""),
        "Rust stack init template must set stack = \"rust\""
    );
}

#[test]
fn init_template_without_stack_omits_stack_commands() {
    let dir = tempfile::tempdir().expect("tempdir");
    let sections = InitSections::from_flags(true, true, true);
    let content = init_template(dir.path(), &sections).expect("init_template must succeed");
    assert!(
        content.contains("[output]"),
        "init template must include base [output]"
    );
    // No stack detected, so no stack-specific commands; base has no commands.
    let config: Config = toml::from_str(&content).expect("init template must deserialize");
    assert!(
        config.commands.is_empty(),
        "init without detected stack should have no commands"
    );
}
