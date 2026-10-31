//! Drop `test-doc` from the Rust stack-default composites when the project
//! has no library target.
//!
//! Doctests live only in a library, and `cargo test --doc` on a package
//! without one fails ("no library targets found in package") instead of
//! running nothing. The default `qa` and `qa-next` both name `test-doc`, so
//! a binary-only crate would fail both gates over doctests it cannot have.
//!
//! [`apply`] runs after `[extend]` and `[cargo] workspaces`. When the root
//! `Cargo.toml` is a single `[package]` with no library target — no `[lib]`
//! table, and no `src/lib.rs` or `autolib = false` — and no `Cargo.toml`
//! above it declares a `[workspace]`, each of those composites loses its
//! `test-doc` step and the step's mention in its help. A stack default
//! `[extend]` already materialized is edited in place, and so is a config
//! composite identical to the default (what `ops init --commands` writes);
//! any other default is materialized without it. Composites the config
//! defines differently are the author's and are left alone, and so is every
//! composite when the config defines its own `test-doc`.
//!
//! A workspace keeps `test-doc`, including when ops runs from a lib-less
//! member (one the nearest enclosing `[workspace]` does not `exclude`):
//! `--workspace` then reaches the sibling libraries, and cargo only
//! fails when no selected package has one — telling that apart needs every
//! member manifest, which is not worth reading on each dispatch. A manifest
//! that cannot be read or parsed also keeps it, so cargo reports the real
//! error.

use std::path::Path;

use super::workspaces::is_default_copy;
use super::{CommandSpec, Config};

/// The stack-default step this module removes.
const TEST_DOC: &str = "test-doc";

/// How the default composites' help names the step, e.g. "build, test-doc,
/// check-json".
const TEST_DOC_IN_HELP: &str = "test-doc, ";

fn read_manifest(dir: &Path) -> Option<toml::Table> {
    let text = std::fs::read_to_string(dir.join("Cargo.toml")).ok()?;
    text.parse().ok()
}

/// Whether the root package belongs to a workspace declared above it, so its
/// siblings may have doctests. Like cargo, this looks at the nearest
/// `Cargo.toml` above with a `[workspace]` table: a package one of its
/// `exclude` entries covers is a standalone root. A manifest above that
/// cannot be read counts as an enclosing workspace.
fn has_enclosing_workspace(workspace_root: &Path) -> bool {
    for dir in workspace_root.ancestors().skip(1) {
        if !dir.join("Cargo.toml").exists() {
            continue;
        }
        let Some(manifest) = read_manifest(dir) else {
            return true;
        };
        let Some(workspace) = manifest.get("workspace").and_then(toml::Value::as_table) else {
            continue;
        };
        let Ok(relative) = workspace_root.strip_prefix(dir) else {
            return true;
        };
        let excluded = workspace
            .get("exclude")
            .and_then(toml::Value::as_array)
            .is_some_and(|entries| {
                entries
                    .iter()
                    .filter_map(toml::Value::as_str)
                    .any(|entry| relative.starts_with(entry.trim_start_matches("./")))
            });
        return !excluded;
    }
    false
}

/// Whether the root manifest is a lone package with no library target.
/// `false` whenever that cannot be established.
fn is_lib_less_package(workspace_root: &Path) -> bool {
    let Some(manifest) = read_manifest(workspace_root) else {
        return false;
    };
    let Some(package) = manifest.get("package").and_then(toml::Value::as_table) else {
        return false;
    };
    if manifest.contains_key("workspace")
        || manifest.contains_key("lib")
        || has_enclosing_workspace(workspace_root)
    {
        return false;
    }
    let autolib = package
        .get("autolib")
        .and_then(toml::Value::as_bool)
        .unwrap_or(true);
    !(autolib && workspace_root.join("src/lib.rs").exists())
}

fn drop_test_doc(composite: &mut super::CompositeCommandSpec) {
    composite.commands.retain(|c| c != TEST_DOC);
    if let Some(help) = &mut composite.help {
        *help = help.replace(TEST_DOC_IN_HELP, "");
    }
}

/// Remove `test-doc` from the Rust stack-default composites of a project
/// without a library target; a no-op otherwise.
pub(super) fn apply(config: &mut Config, workspace_root: &Path) {
    if crate::stack::Stack::resolve(config.stack.as_deref(), workspace_root)
        != Some(crate::stack::Stack::Rust)
    {
        return;
    }
    let defaults = crate::stack::Stack::Rust.default_commands_ref();
    if let Some(own) = config.commands.get(TEST_DOC) {
        let Some(default) = defaults.get(TEST_DOC) else {
            return;
        };
        if !is_default_copy(own, default) {
            return;
        }
    }
    if !is_lib_less_package(workspace_root) {
        return;
    }
    for (name, spec) in defaults {
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
            Some(CommandSpec::Composite(composite))
                if materialized || is_default_copy(&*composite, default) =>
            {
                drop_test_doc(composite);
            }
            Some(_) => {}
            None => {
                let mut composite = default.clone();
                drop_test_doc(&mut composite);
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

        for name in ["qa", "qa-next"] {
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
        assert!(steps(&config, "qa").iter().any(|s| s == "deps"));
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
        assert!(!steps(&config, "qa").iter().any(|s| s == TEST_DOC));
    }

    #[test]
    fn config_composites_and_a_config_test_doc_are_left_alone() {
        let dir = project(PACKAGE, false);
        let own = CompositeCommandSpec::new(["build", TEST_DOC]);

        let mut config = Config::empty();
        config
            .commands
            .insert("qa".into(), CommandSpec::Composite(own.clone()));
        apply(&mut config, dir.path());
        assert_eq!(steps(&config, "qa"), ["build", TEST_DOC]);

        let mut config = Config::empty();
        config
            .commands
            .insert(TEST_DOC.into(), CommandSpec::Composite(own));
        apply(&mut config, dir.path());
        assert!(!config.commands.contains_key("qa"));
    }

    /// Greptile on PR #118: `ops init --commands` writes verbatim copies of
    /// the defaults, `test-doc` included, and a bin-only scaffold must still
    /// get a passing `qa`.
    #[test]
    fn verbatim_default_copies_from_init_are_treated_as_defaults() {
        let dir = project(PACKAGE, false);
        let mut config = Config::empty();
        for (name, spec) in crate::stack::Stack::Rust.default_commands_ref() {
            config.commands.insert(name.clone(), spec.clone());
        }
        apply(&mut config, dir.path());
        for name in ["qa", "qa-next"] {
            assert!(
                !steps(&config, name).iter().any(|s| s == TEST_DOC),
                "{name}"
            );
        }
    }

    /// Greptile on PR #118: from a bin-only member, `--workspace` still
    /// reaches the sibling libraries' doctests.
    #[test]
    fn a_member_of_an_enclosing_workspace_keeps_test_doc() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"app\", \"lib\"]\n",
        )
        .unwrap();
        let member = dir.path().join("app");
        std::fs::create_dir_all(member.join("src")).unwrap();
        std::fs::write(member.join("Cargo.toml"), PACKAGE).unwrap();
        std::fs::write(member.join("src/main.rs"), "").unwrap();

        let mut config = Config::empty();
        apply(&mut config, &member);
        assert!(config.commands.is_empty());
    }

    /// Greptile on PR #118: a package the enclosing workspace excludes is
    /// its own root, so `--workspace` reaches no sibling library.
    #[test]
    fn a_package_the_enclosing_workspace_excludes_drops_test_doc() {
        for exclude in ["tools", "./tools/app"] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(
                dir.path().join("Cargo.toml"),
                format!("[workspace]\nmembers = [\"lib\"]\nexclude = [\"{exclude}\"]\n"),
            )
            .unwrap();
            let member = dir.path().join("tools/app");
            std::fs::create_dir_all(member.join("src")).unwrap();
            std::fs::write(member.join("Cargo.toml"), PACKAGE).unwrap();
            std::fs::write(member.join("src/main.rs"), "").unwrap();

            let mut config = Config::empty();
            apply(&mut config, &member);
            assert!(
                !steps(&config, "qa").iter().any(|s| s == TEST_DOC),
                "exclude = {exclude:?}"
            );
        }
    }

    #[test]
    fn help_no_longer_names_the_dropped_step() {
        let dir = project(PACKAGE, false);
        let mut config = Config::empty();
        apply(&mut config, dir.path());
        for name in ["qa", "qa-next"] {
            let Some(CommandSpec::Composite(c)) = config.commands.get(name) else {
                panic!("{name} must be materialized");
            };
            let help = c.help.as_deref().unwrap_or_default();
            assert!(!help.contains(TEST_DOC), "{name}: {help}");
            assert!(help.contains("Run "), "{name} lost its help: {help}");
        }
    }

    #[test]
    fn a_materialized_default_is_edited_in_place() {
        let dir = project(PACKAGE, false);
        let mut config = Config::empty();
        config.commands.insert(
            "qa".into(),
            CommandSpec::Composite(CompositeCommandSpec::new(["build", TEST_DOC, "extra"])),
        );
        config.provenance.extended_stack_defaults.push("qa".into());
        apply(&mut config, dir.path());
        assert_eq!(steps(&config, "qa"), ["build", "extra"]);
    }
}
