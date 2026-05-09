//! Per-stack manifest list and embedded default-commands TOML.
//!
//! ARCH-1 / TASK-1185: extracted from the monolithic `stack.rs` so adding a
//! new stack touches only this metadata table — the detection walk and
//! enum live in sibling modules.

use super::Stack;

/// Single source of truth for per-stack metadata: (manifest_files, default_commands_toml).
///
/// Consolidates two parallel match blocks (CD-11) so adding a new stack
/// updates exactly one match arm.
pub(super) fn metadata(stack: Stack) -> (&'static [&'static str], Option<&'static str>) {
    // Reduces the include_str!(concat!(env!(...), "/src/", file)) boilerplate to one line per arm.
    macro_rules! meta {
        ($files:expr, $toml:literal) => {
            (
                $files as &[&str],
                Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/",
                    $toml
                ))),
            )
        };
    }
    match stack {
        Stack::Rust => meta!(&["Cargo.toml"], ".default.rust.ops.toml"),
        Stack::Node => meta!(&["package.json"], ".default.node.ops.toml"),
        Stack::Go => meta!(&["go.mod"], ".default.go.ops.toml"),
        Stack::Python => meta!(
            &["pyproject.toml", "setup.py", "requirements.txt"],
            ".default.python.ops.toml"
        ),
        Stack::Terraform => meta!(&["main.tf", "terraform.tf"], ".default.terraform.ops.toml"),
        Stack::Ansible => meta!(
            &["site.yml", "playbook.yml", "ansible.cfg"],
            ".default.ansible.ops.toml"
        ),
        Stack::JavaMaven => meta!(&["pom.xml"], ".default.java-maven.ops.toml"),
        Stack::JavaGradle => meta!(
            &["build.gradle", "build.gradle.kts"],
            ".default.java-gradle.ops.toml"
        ),
        Stack::Generic => (&[], None),
    }
}
