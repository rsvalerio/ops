//! Stack detection and stack-specific defaults.
//!
//! A "stack" represents a language, framework, or toolchain (Rust, Node, Go, etc.).
//! Each stack has:
//! - Manifest files used for detection (Cargo.toml, package.json, go.mod)
//! - Default commands (build, test, lint)
//! - Default data directory location

use crate::config::{CommandSpec, CompositeCommandSpec, ExecCommandSpec};
use indexmap::IndexMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, strum::EnumString, strum::IntoStaticStr)]
#[strum(serialize_all = "lowercase")]
pub enum Stack {
    Rust,
    Node,
    Go,
    Python,
    Terraform,
    Ansible,
    Generic,
}

impl Stack {
    pub fn as_str(&self) -> &'static str {
        (*self).into()
    }

    #[allow(dead_code)]
    pub fn manifest_files(&self) -> &[&str] {
        match self {
            Stack::Rust => &["Cargo.toml"],
            Stack::Node => &["package.json"],
            Stack::Go => &["go.mod"],
            Stack::Python => &["pyproject.toml", "setup.py", "requirements.txt"],
            Stack::Terraform => &["main.tf", "terraform.tf"],
            Stack::Ansible => &["site.yml", "playbook.yml", "ansible.cfg"],
            Stack::Generic => &[],
        }
    }

    /// DUP-001: Resolve stack from config override or auto-detection.
    ///
    /// Shared by `CommandRunner::new()` and `extensions::resolve_stack()`.
    pub fn resolve(config_stack: Option<&str>, workspace_root: &Path) -> Option<Self> {
        config_stack
            .and_then(|s| s.parse().ok())
            .or_else(|| Self::detect(workspace_root))
    }

    pub fn detect(start: &Path) -> Option<Self> {
        let candidates: &[(Stack, &[&str])] = &[
            (Stack::Rust, &["Cargo.toml"]),
            (Stack::Node, &["package.json"]),
            (Stack::Go, &["go.mod"]),
            (Stack::Python, &["pyproject.toml", "setup.py"]),
            (Stack::Terraform, &["main.tf", "terraform.tf"]),
            (Stack::Ansible, &["site.yml", "playbook.yml"]),
        ];

        let mut current = start.to_path_buf();
        loop {
            for (stack, files) in candidates {
                for file in *files {
                    if current.join(file).exists() {
                        return Some(*stack);
                    }
                }
            }
            if !current.pop() {
                return None;
            }
        }
    }

    pub fn default_commands(&self) -> IndexMap<String, CommandSpec> {
        match self {
            Stack::Rust => build_commands(
                "cargo",
                &[
                    ("fmt", &["fmt", "--all"]),
                    ("check", &["check", "--all"]),
                    ("clippy", &["clippy", "--all", "--", "-D", "warnings"]),
                    ("build", &["build", "--all"]),
                    ("test", &["test", "--all"]),
                ],
                &["fmt", "check", "clippy", "build", "test"],
            ),
            Stack::Node => build_commands(
                "npm",
                &[
                    ("install", &["install"]),
                    ("build", &["run", "build"]),
                    ("test", &["test"]),
                    ("lint", &["run", "lint"]),
                ],
                &["install", "lint", "build", "test"],
            ),
            Stack::Go => build_commands(
                "go",
                &[
                    ("fmt", &["fmt", "./..."]),
                    ("vet", &["vet", "./..."]),
                    ("build", &["build", "./..."]),
                    ("test", &["test", "./..."]),
                ],
                &["fmt", "vet", "build", "test"],
            ),
            Stack::Python => build_commands_multi(
                &[
                    ("format", "ruff", &["format", "."]),
                    ("lint", "ruff", &["check", "."]),
                    ("test", "pytest", &[]),
                ],
                &["format", "lint", "test"],
            ),
            Stack::Terraform => build_commands(
                "terraform",
                &[
                    ("init", &["init"]),
                    ("fmt", &["fmt", "-recursive"]),
                    ("validate", &["validate"]),
                    ("plan", &["plan"]),
                ],
                &["fmt", "validate"],
            ),
            Stack::Ansible => build_commands_multi(
                &[
                    ("lint", "ansible-lint", &[]),
                    ("check", "ansible-playbook", &["--check", "site.yml"]),
                ],
                &["lint"],
            ),
            Stack::Generic => IndexMap::new(),
        }
    }

    #[allow(dead_code)]
    pub fn data_dir_name(&self) -> &'static str {
        match self {
            Stack::Rust => "target",
            Stack::Node => "node_modules",
            Stack::Go => ".",
            Stack::Python => ".",
            Stack::Terraform => ".terraform",
            Stack::Ansible => ".",
            Stack::Generic => ".ops",
        }
    }

    #[allow(dead_code)]
    pub fn description(&self) -> &'static str {
        match self {
            Stack::Rust => "Rust (Cargo)",
            Stack::Node => "Node.js (npm/yarn)",
            Stack::Go => "Go modules",
            Stack::Python => "Python (pip/poetry)",
            Stack::Terraform => "Terraform",
            Stack::Ansible => "Ansible",
            Stack::Generic => "Generic (no stack detected)",
        }
    }
}

/// Build an exec command spec.
fn exec(program: &str, args: &[&str]) -> CommandSpec {
    CommandSpec::Exec(ExecCommandSpec {
        program: program.into(),
        args: args.iter().map(|a| (*a).into()).collect(),
        ..Default::default()
    })
}

/// Build a verify composite command spec.
fn verify(commands: &[&str]) -> CommandSpec {
    CommandSpec::Composite(CompositeCommandSpec {
        commands: commands.iter().map(|c| (*c).into()).collect(),
        parallel: false,
        fail_fast: true,
    })
}

/// Build commands for a stack where all exec commands share the same program.
fn build_commands(
    program: &str,
    steps: &[(&str, &[&str])],
    verify_steps: &[&str],
) -> IndexMap<String, CommandSpec> {
    let mut cmds = IndexMap::new();
    for (name, args) in steps {
        cmds.insert((*name).into(), exec(program, args));
    }
    cmds.insert("verify".into(), verify(verify_steps));
    cmds
}

/// Build commands for a stack where exec commands may use different programs.
fn build_commands_multi(
    steps: &[(&str, &str, &[&str])],
    verify_steps: &[&str],
) -> IndexMap<String, CommandSpec> {
    let mut cmds = IndexMap::new();
    for (name, program, args) in steps {
        cmds.insert((*name).into(), exec(program, args));
    }
    cmds.insert("verify".into(), verify(verify_steps));
    cmds
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_from_str_roundtrip() {
        for stack in [
            Stack::Rust,
            Stack::Node,
            Stack::Go,
            Stack::Python,
            Stack::Terraform,
            Stack::Ansible,
            Stack::Generic,
        ] {
            assert_eq!(stack.as_str().parse::<Stack>(), Ok(stack));
        }
    }

    #[test]
    fn stack_from_str_unknown() {
        assert!("unknown".parse::<Stack>().is_err());
    }

    #[test]
    fn rust_has_default_commands() {
        let cmds = Stack::Rust.default_commands();
        assert!(cmds.contains_key("build"));
        assert!(cmds.contains_key("test"));
        assert!(cmds.contains_key("verify"));
    }

    #[test]
    fn generic_has_no_default_commands() {
        let cmds = Stack::Generic.default_commands();
        assert!(cmds.is_empty());
    }

    #[test]
    fn detect_returns_none_without_manifest() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(Stack::detect(dir.path()), None);
    }

    #[test]
    fn detect_finds_cargo_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Cargo.toml"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Rust));
    }

    #[test]
    fn detect_finds_package_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("package.json"), "{}").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Node));
    }

    #[test]
    fn detect_finds_go_mod() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("go.mod"), "module test").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Go));
    }

    #[test]
    fn detect_finds_pyproject_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("pyproject.toml"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Python));
    }

    #[test]
    fn detect_finds_terraform() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("main.tf"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Terraform));
    }

    #[test]
    fn detect_finds_ansible() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("site.yml"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Ansible));
    }

    #[test]
    fn detect_walks_up_directories() {
        let dir = tempfile::tempdir().expect("tempdir");
        let subdir = dir.path().join("src").join("lib");
        std::fs::create_dir_all(&subdir).expect("create_dir");
        std::fs::write(dir.path().join("Cargo.toml"), "").expect("write");
        assert_eq!(Stack::detect(&subdir), Some(Stack::Rust));
    }

    #[test]
    fn rust_prioritized_over_node() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("Cargo.toml"), "").expect("write");
        std::fs::write(dir.path().join("package.json"), "{}").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Rust));
    }

    #[test]
    fn detect_prioritizes_go_over_python() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("go.mod"), "module test").expect("write");
        std::fs::write(dir.path().join("pyproject.toml"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Go));
    }

    #[test]
    fn detect_prioritizes_node_over_terraform() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("package.json"), "{}").expect("write");
        std::fs::write(dir.path().join("main.tf"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Node));
    }

    #[test]
    fn data_dir_name_rust_is_target() {
        assert_eq!(Stack::Rust.data_dir_name(), "target");
    }

    #[test]
    fn data_dir_name_node_is_node_modules() {
        assert_eq!(Stack::Node.data_dir_name(), "node_modules");
    }
}
