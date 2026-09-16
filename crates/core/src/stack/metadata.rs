//! Per-stack manifest list and embedded default-commands TOML.
//!
//! ARCH-1 / TASK-1185: extracted from the monolithic `stack.rs` so adding a
//! new stack touches only this metadata table — the detection walk and
//! enum live in sibling modules.

use super::Stack;

/// Single source of truth for per-stack metadata:
/// (`manifest_files`, `default_commands_toml`, `build_dirs`).
///
/// Consolidates parallel match blocks (CD-11) so adding a new stack
/// updates exactly one match arm.
///
/// `build_dirs` (TASK-2264) are the stack's default build output and
/// dependency directories — generated artefacts, not source. `ops sec`
/// skips them at any depth in every Trivy scan so it reads source instead
/// of `target/`-shaped junk and does not race the builds producing it.
/// Ansible's collection/role caches install under `$HOME/.ansible` by
/// default, not inside the repo, so it declares none.
pub(super) const fn metadata(
    stack: Stack,
) -> (
    &'static [&'static str],
    Option<&'static str>,
    &'static [&'static str],
) {
    // Reduces the include_str!(concat!(env!(...), "/src/", file)) boilerplate to one line per arm.
    macro_rules! meta {
        ($files:expr, $toml:literal, $build_dirs:expr) => {
            (
                $files,
                Some(include_str!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/src/",
                    $toml
                ))),
                $build_dirs,
            )
        };
    }
    match stack {
        Stack::Rust => meta!(&["Cargo.toml"], ".default.rust.ops.toml", &["target"]),
        Stack::Vite => meta!(
            &[
                "vite.config.ts",
                "vite.config.js",
                "vite.config.mjs",
                "vite.config.mts",
                "vite.config.cjs",
                "vite.config.cts",
            ],
            ".default.vite.ops.toml",
            &["node_modules", "dist"]
        ),
        Stack::Node => meta!(
            &["package.json"],
            ".default.node.ops.toml",
            &["node_modules", "dist"]
        ),
        Stack::Go => meta!(&["go.mod"], ".default.go.ops.toml", &["vendor"]),
        Stack::Python => meta!(
            &["pyproject.toml", "setup.py", "requirements.txt"],
            ".default.python.ops.toml",
            &[".venv", "venv", "__pycache__", "build", "dist"]
        ),
        Stack::Terraform => meta!(
            &["main.tf", "terraform.tf"],
            ".default.terraform.ops.toml",
            &[".terraform"]
        ),
        Stack::Ansible => meta!(
            &["site.yml", "playbook.yml", "ansible.cfg"],
            ".default.ansible.ops.toml",
            &[]
        ),
        Stack::JavaMaven => meta!(&["pom.xml"], ".default.java-maven.ops.toml", &["target"]),
        Stack::JavaGradle => meta!(
            &["build.gradle", "build.gradle.kts"],
            ".default.java-gradle.ops.toml",
            &["build", ".gradle"]
        ),
        Stack::Generic => (&[], None, &[]),
    }
}
