//! Drop `test-doc` from the Rust stack-default composites when the project
//! has no library target.
//!
//! Doctests live only in a library, and `cargo test --doc` on a package
//! without one fails ("no library targets found in package") instead of
//! running nothing. The default `verify`, `verify-fix`, `qa` and `qa-next`
//! all name `test-doc` (TASK-2635), so a binary-only crate would fail every
//! gate over doctests it cannot have.
//!
//! [`apply`] runs after `[extend]` and `[cargo] workspaces`. When the root
//! `Cargo.toml` is a single `[package]` (no `[workspace]` table) with no
//! library target — no `[lib]` table, and no `src/lib.rs` or `autolib =
//! false` — each of those composites loses its `test-doc` step: a stack
//! default `[extend]` already materialized is edited in place, any other is
//! materialized without it. Composites the config defines are the author's
//! and are left alone, and so is every composite when the config defines
//! `test-doc` itself. A workspace keeps `test-doc`: cargo only fails when no
//! selected package has a library, and telling that apart needs every
//! member manifest, which is not worth reading on each dispatch. A manifest
//! that cannot be read or parsed also keeps it, so cargo reports the real
//! error.

use std::path::Path;

use super::{CommandSpec, Config};

/// The stack-default step this module removes.
const TEST_DOC: &str = "test-doc";

/// Whether the root manifest is a lone package with no library target.
/// `false` whenever that cannot be established.
fn is_lib_less_package(workspace_root: &Path) -> bool {
    let Ok(text) = std::fs::read_to_string(workspace_root.join("Cargo.toml")) else {
        return false;
    };
    let Ok(manifest) = text.parse::<toml::Table>() else {
        return false;
    };
    let Some(package) = manifest.get("package").and_then(toml::Value::as_table) else {
        return false;
    };
    if manifest.contains_key("workspace") || manifest.contains_key("lib") {
        return false;
    }
    let autolib = package
        .get("autolib")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    !(autolib && workspace_root.join("src/lib.rs").exists())
}

/// Remove `test-doc` from the Rust stack-default composites of a project
/// without a library target; a no-op otherwise.
pub(super) fn apply(config: &mut Config, workspace_root: &Path) {
    if config.commands.contains_key(TEST_DOC) {
        return;
    }
    if crate::stack::Stack::resolve(config.stack.as_deref(), workspace_root)
        != Some(crate::stack::Stack::Rust)
    {
        return;
    }
    if !is_lib_less_package(workspace_root) {
        return;
    }
    for (name, spec) in crate::stack::Stack::Rust.default_commands_ref() {
        let CommandSpec::Composite(default) = spec else {
            continue;
        };
        if !default.commands.iter().any(|c| c == TEST_DOC) {
            continue;
        }
        let materialized = config
            .provenance
            .extended_stack_defaults
            .iter()
            .any(|n| n == name);
        match config.commands.get_mut(name) {
            Some(CommandSpec::Composite(composite)) if materialized => {
                composite.commands.retain(|c| c != TEST_DOC);
            }
            Some(_) => {}
            None => {
                let mut composite = default.clone();
                composite.commands.retain(|c| c != TEST_DOC);
                config
                    .commands
                    .insert(name.clone(), CommandSpec::Composite(composite));
                config.provenance.extended_stack_defaults.push(name.clone());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CompositeCommandSpec;

    fn project(manifest: &str, lib: bool) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), manifest).unwrap();
        std::fs::create_dir(dir.path().join("src")).unwrap();
        let file = if lib { "src/lib.rs" } else { "src/main.rs" };
        std::fs::write(dir.path().join(file), "").unwrap();
        dir
    }

    fn steps(config: &Config, name: &str) -> Vec<String> {
        match config.commands.get(name) {
            Some(CommandSpec::Composite(c)) => c.commands.clone(),
            other => panic!("{name} must be a materialized composite, got {other:?}"),
        }
    }

    const PACKAGE: &str = "[package]\nname = \"x\"\n";

    #[test]
    fn bin_only_package_drops_test_doc_from_every_default_composite() {
        let dir = project(PACKAGE, false);
        let mut config = Config::empty();
        apply(&mut config, dir.path());

        for name in ["verify", "verify-fix", "qa", "qa-next"] {
            let steps = steps(&config, name);
            assert!(!steps.iter().any(|s| s == TEST_DOC), "{name}: {steps:?}");
            assert!(
                steps.iter().any(|s| s != TEST_DOC),
                "{name} lost its other steps"
            );
            assert!(
                config
                    .provenance
                    .extended_stack_defaults
                    .iter()
                    .any(|n| n == name),
                "{name} must be recorded as a stack default"
            );
        }
        assert!(steps(&config, "verify").iter().any(|s| s == "build"));
    }

    #[test]
    fn a_library_keeps_test_doc() {
        for (manifest, lib) in [
            (PACKAGE, true),
            (
                "[package]\nname = \"x\"\n[lib]\npath = \"src/x.rs\"\n",
                false,
            ),
            ("[package]\nname = \"x\"\n[workspace]\n", false),
            ("[workspace]\nmembers = [\"a\"]\n", false),
            ("not = [toml", false),
        ] {
            let dir = project(manifest, lib);
            let mut config = Config::empty();
            apply(&mut config, dir.path());
            assert!(config.commands.is_empty(), "{manifest:?} lib={lib}");
        }
    }

    #[test]
    fn autolib_false_ignores_src_lib_rs() {
        let dir = project("[package]\nname = \"x\"\nautolib = false\n", true);
        let mut config = Config::empty();
        apply(&mut config, dir.path());
        assert!(!steps(&config, "verify").iter().any(|s| s == TEST_DOC));
    }

    #[test]
    fn config_composites_and_a_config_test_doc_are_left_alone() {
        let dir = project(PACKAGE, false);
        let own = CompositeCommandSpec::new(["build", TEST_DOC]);

        let mut config = Config::empty();
        config
            .commands
            .insert("verify".into(), CommandSpec::Composite(own.clone()));
        apply(&mut config, dir.path());
        assert_eq!(steps(&config, "verify"), ["build", TEST_DOC]);

        let mut config = Config::empty();
        config
            .commands
            .insert(TEST_DOC.into(), CommandSpec::Composite(own));
        apply(&mut config, dir.path());
        assert!(!config.commands.contains_key("verify"));
    }

    #[test]
    fn a_materialized_default_is_edited_in_place() {
        let dir = project(PACKAGE, false);
        let mut config = Config::empty();
        config.commands.insert(
            "verify".into(),
            CommandSpec::Composite(CompositeCommandSpec::new(["build", TEST_DOC, "extra"])),
        );
        config
            .provenance
            .extended_stack_defaults
            .push("verify".into());
        apply(&mut config, dir.path());
        assert_eq!(steps(&config, "verify"), ["build", "extra"]);
    }
}
