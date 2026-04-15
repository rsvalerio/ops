//! Stack detection and stack-specific defaults.
//!
//! A "stack" represents a language, framework, or toolchain (Rust, Node, Go, etc.).
//! Each stack has:
//! - Manifest files used for detection (Cargo.toml, package.json, go.mod)
//! - Default commands (build, test, lint) loaded from embedded `.default.<stack>.ops.toml`
//! - Default data directory location

use crate::config::{CommandSpec, Config};
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
    #[strum(serialize = "java-maven")]
    JavaMaven,
    #[strum(serialize = "java-gradle")]
    JavaGradle,
    Generic,
}

impl Stack {
    pub fn as_str(&self) -> &'static str {
        (*self).into()
    }

    pub fn manifest_files(&self) -> &[&str] {
        self.metadata().0
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
        // Priority order for detection (Generic is excluded — no manifest files).
        const DETECT_ORDER: &[Stack] = &[
            Stack::Rust,
            Stack::Node,
            Stack::Go,
            Stack::Python,
            Stack::Terraform,
            Stack::Ansible,
            Stack::JavaGradle,
            Stack::JavaMaven,
        ];

        let mut current = start.to_path_buf();
        loop {
            if let Some(&stack) = DETECT_ORDER
                .iter()
                .find(|s| s.manifest_files().iter().any(|f| current.join(f).exists()))
            {
                return Some(stack);
            }
            if !current.pop() {
                return None;
            }
        }
    }

    /// Embedded TOML content for this stack's default commands, or None for Generic.
    fn default_commands_toml(&self) -> Option<&'static str> {
        self.metadata().1
    }

    /// Single source of truth for per-stack metadata: (manifest_files, default_commands_toml).
    ///
    /// Consolidates two parallel match blocks (CD-11) so that adding a new stack
    /// requires updating exactly one match arm.
    fn metadata(&self) -> (&[&str], Option<&'static str>) {
        match self {
            Stack::Rust => (
                &["Cargo.toml"],
                Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/.default.rust.ops.toml"
                ))),
            ),
            Stack::Node => (
                &["package.json"],
                Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/.default.node.ops.toml"
                ))),
            ),
            Stack::Go => (
                &["go.mod"],
                Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/.default.go.ops.toml"
                ))),
            ),
            Stack::Python => (
                &["pyproject.toml", "setup.py", "requirements.txt"],
                Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/.default.python.ops.toml"
                ))),
            ),
            Stack::Terraform => (
                &["main.tf", "terraform.tf"],
                Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/.default.terraform.ops.toml"
                ))),
            ),
            Stack::Ansible => (
                &["site.yml", "playbook.yml", "ansible.cfg"],
                Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/.default.ansible.ops.toml"
                ))),
            ),
            Stack::JavaMaven => (
                &["pom.xml"],
                Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/.default.java-maven.ops.toml"
                ))),
            ),
            Stack::JavaGradle => (
                &["build.gradle", "build.gradle.kts"],
                Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/.default.java-gradle.ops.toml"
                ))),
            ),
            Stack::Generic => (&[], None),
        }
    }

    pub fn default_commands(&self) -> IndexMap<String, CommandSpec> {
        let toml = match self.default_commands_toml() {
            Some(t) => t,
            None => return IndexMap::new(),
        };
        let config: Config =
            toml::from_str(toml).expect("stack default commands TOML must be valid");
        config.commands
    }
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
            Stack::JavaMaven,
            Stack::JavaGradle,
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

    // DUP-011 regression: detect must check all manifest_files()
    #[test]
    fn detect_finds_requirements_txt() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("requirements.txt"), "flask\n").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Python));
    }

    #[test]
    fn detect_finds_ansible_cfg() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("ansible.cfg"), "[defaults]\n").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::Ansible));
    }

    #[test]
    fn detect_finds_pom_xml() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("pom.xml"), "<project/>").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::JavaMaven));
    }

    #[test]
    fn detect_finds_build_gradle() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("build.gradle"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::JavaGradle));
    }

    #[test]
    fn detect_finds_build_gradle_kts() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("build.gradle.kts"), "").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::JavaGradle));
    }

    #[test]
    fn gradle_prioritized_over_maven() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("build.gradle"), "").expect("write");
        std::fs::write(dir.path().join("pom.xml"), "<project/>").expect("write");
        assert_eq!(Stack::detect(dir.path()), Some(Stack::JavaGradle));
    }

    #[test]
    fn each_stack_default_toml_parses_and_includes_verify() {
        for stack in [
            Stack::Rust,
            Stack::Node,
            Stack::Go,
            Stack::Python,
            Stack::Terraform,
            Stack::Ansible,
            Stack::JavaMaven,
            Stack::JavaGradle,
        ] {
            let cmds = stack.default_commands();
            assert!(
                cmds.contains_key("verify"),
                "stack {} default TOML must define verify",
                stack.as_str()
            );
            assert!(
                !cmds.is_empty(),
                "stack {} default TOML must define at least one command",
                stack.as_str()
            );
        }
    }

    #[test]
    fn stacks_with_test_define_qa() {
        for stack in [
            Stack::Rust,
            Stack::Node,
            Stack::Go,
            Stack::Python,
            Stack::JavaMaven,
            Stack::JavaGradle,
        ] {
            let cmds = stack.default_commands();
            assert!(
                cmds.contains_key("qa"),
                "stack {} default TOML must define qa",
                stack.as_str()
            );
        }
    }
}
