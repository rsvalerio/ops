//! Stack detection and stack-specific defaults.
//!
//! A "stack" represents a language, framework, or toolchain (Rust, Node, Go, etc.).
//! Each stack has:
//! - Manifest files used for detection (Cargo.toml, package.json, go.mod)
//! - Default commands (build, test, lint)
//! - Default data directory location

use crate::config::{CommandSpec, CompositeCommandSpec, ExecCommandSpec};
use indexmap::IndexMap;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
        match self {
            Stack::Rust => "rust",
            Stack::Node => "node",
            Stack::Go => "go",
            Stack::Python => "python",
            Stack::Terraform => "terraform",
            Stack::Ansible => "ansible",
            Stack::Generic => "generic",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "rust" => Some(Stack::Rust),
            "node" => Some(Stack::Node),
            "go" => Some(Stack::Go),
            "python" => Some(Stack::Python),
            "terraform" => Some(Stack::Terraform),
            "ansible" => Some(Stack::Ansible),
            "generic" => Some(Stack::Generic),
            _ => None,
        }
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
            Stack::Rust => rust_commands(),
            Stack::Node => node_commands(),
            Stack::Go => go_commands(),
            Stack::Python => python_commands(),
            Stack::Terraform => terraform_commands(),
            Stack::Ansible => ansible_commands(),
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

fn rust_commands() -> IndexMap<String, CommandSpec> {
    let mut cmds = IndexMap::new();

    cmds.insert(
        "fmt".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "cargo".into(),
            args: vec!["fmt".into(), "--all".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "check".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "cargo".into(),
            args: vec!["check".into(), "--all".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "clippy".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "cargo".into(),
            args: vec![
                "clippy".into(),
                "--all".into(),
                "--".into(),
                "-D".into(),
                "warnings".into(),
            ],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "build".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "cargo".into(),
            args: vec!["build".into(), "--all".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "test".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "cargo".into(),
            args: vec!["test".into(), "--all".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "verify".into(),
        CommandSpec::Composite(CompositeCommandSpec {
            commands: vec![
                "fmt".into(),
                "check".into(),
                "clippy".into(),
                "build".into(),
                "test".into(),
            ],
            parallel: false,
            fail_fast: true,
        }),
    );

    cmds
}

fn node_commands() -> IndexMap<String, CommandSpec> {
    let mut cmds = IndexMap::new();

    cmds.insert(
        "install".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "npm".into(),
            args: vec!["install".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "build".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "npm".into(),
            args: vec!["run".into(), "build".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "test".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "npm".into(),
            args: vec!["test".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "lint".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "npm".into(),
            args: vec!["run".into(), "lint".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "verify".into(),
        CommandSpec::Composite(CompositeCommandSpec {
            commands: vec![
                "install".into(),
                "lint".into(),
                "build".into(),
                "test".into(),
            ],
            parallel: false,
            fail_fast: true,
        }),
    );

    cmds
}

fn go_commands() -> IndexMap<String, CommandSpec> {
    let mut cmds = IndexMap::new();

    cmds.insert(
        "fmt".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "go".into(),
            args: vec!["fmt".into(), "./...".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "vet".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "go".into(),
            args: vec!["vet".into(), "./...".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "build".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "go".into(),
            args: vec!["build".into(), "./...".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "test".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "go".into(),
            args: vec!["test".into(), "./...".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "verify".into(),
        CommandSpec::Composite(CompositeCommandSpec {
            commands: vec!["fmt".into(), "vet".into(), "build".into(), "test".into()],
            parallel: false,
            fail_fast: true,
        }),
    );

    cmds
}

fn python_commands() -> IndexMap<String, CommandSpec> {
    let mut cmds = IndexMap::new();

    cmds.insert(
        "format".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "ruff".into(),
            args: vec!["format".into(), ".".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "lint".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "ruff".into(),
            args: vec!["check".into(), ".".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "test".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "pytest".into(),
            args: vec![],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "verify".into(),
        CommandSpec::Composite(CompositeCommandSpec {
            commands: vec!["format".into(), "lint".into(), "test".into()],
            parallel: false,
            fail_fast: true,
        }),
    );

    cmds
}

fn terraform_commands() -> IndexMap<String, CommandSpec> {
    let mut cmds = IndexMap::new();

    cmds.insert(
        "init".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "terraform".into(),
            args: vec!["init".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "fmt".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "terraform".into(),
            args: vec!["fmt".into(), "-recursive".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "validate".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "terraform".into(),
            args: vec!["validate".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "plan".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "terraform".into(),
            args: vec!["plan".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "verify".into(),
        CommandSpec::Composite(CompositeCommandSpec {
            commands: vec!["fmt".into(), "validate".into()],
            parallel: false,
            fail_fast: true,
        }),
    );

    cmds
}

fn ansible_commands() -> IndexMap<String, CommandSpec> {
    let mut cmds = IndexMap::new();

    cmds.insert(
        "lint".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "ansible-lint".into(),
            args: vec![],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "check".into(),
        CommandSpec::Exec(ExecCommandSpec {
            program: "ansible-playbook".into(),
            args: vec!["--check".into(), "site.yml".into()],
            env: HashMap::new(),
            cwd: None,
            timeout_secs: None,
        }),
    );

    cmds.insert(
        "verify".into(),
        CommandSpec::Composite(CompositeCommandSpec {
            commands: vec!["lint".into()],
            parallel: false,
            fail_fast: true,
        }),
    );

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
            assert_eq!(Stack::from_str(stack.as_str()), Some(stack));
        }
    }

    #[test]
    fn stack_from_str_unknown() {
        assert_eq!(Stack::from_str("unknown"), None);
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
    fn data_dir_name_rust_is_target() {
        assert_eq!(Stack::Rust.data_dir_name(), "target");
    }

    #[test]
    fn data_dir_name_node_is_node_modules() {
        assert_eq!(Stack::Node.data_dir_name(), "node_modules");
    }
}
