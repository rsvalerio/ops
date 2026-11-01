//! `[cargo] workspaces = [".", "fuzz"]`: run the workspace-wide Rust stack
//! defaults over every listed cargo workspace.
//!
//! A repository can hold a second cargo workspace beside the root one —
//! cargo-fuzz's `fuzz/` is its own `[workspace]` and sits in the root's
//! `exclude` — and `cargo fmt --all` / `cargo clippy --workspace` never reach
//! it. Without a setting every such repo re-declares a `--manifest-path`
//! clone of each built-in and rewires the composites that name them. This is
//! the one list instead:
//!
//! ```toml
//! [cargo]
//! workspaces = [".", "fuzz"]
//! ```
//!
//! [`apply`] runs after `[extend]` and before `[cargo] locked`. Each Rust
//! stack default in [`WORKSPACE_COMMANDS`] becomes a matrix over a
//! `workspace` axis holding the list, with `--manifest-path
//! ${matrix.workspace}/Cargo.toml` spliced in before any `--` separator, so
//! `clippy` runs once per workspace as one plan step whose cells are
//! labelled `clippy [workspace=fuzz]`. Stack defaults are materialized into
//! `Config::commands` the way `[extend]` materializes them; a stack default
//! `[extend]` already materialized is expanded in place, and so is a config
//! command identical to the default — what `ops init --commands` writes. A
//! command the config defines differently under one of these names is left
//! alone: its args are the author's, and may already name a package or
//! manifest. Test commands stay root-only. `ops deps` reads the same list
//! itself.
//!
//! Every entry, and its `Cargo.toml`, must resolve inside the project: a
//! symlink out of it is a load error, so neither cargo nor `ops deps`
//! follows one.
//!
//! A list that is empty or names only `"."` changes nothing.

use std::path::{Component, Path};

use anyhow::Context as _;

use indexmap::IndexMap;

use super::{CommandSpec, Config, ExecCommandSpec, Matrix, Strategy};

/// The Rust stack defaults that act on a whole workspace and so run once per
/// configured workspace. Test commands are absent on purpose: a nested
/// workspace such as `fuzz/` has no tests of its own to run.
pub const WORKSPACE_COMMANDS: &[&str] = &["fmt", "fmt-check", "check", "clippy", "build"];

/// The matrix axis the workspace directories become.
const AXIS: &str = "workspace";

/// Reject an entry that is not a plain relative directory inside the
/// project: an absolute path or a `..` component would point cargo (and
/// `ops deps`) outside the workspace, the escape `cwd` validation refuses.
fn validate_entry(entry: &str) -> anyhow::Result<()> {
    anyhow::ensure!(
        !entry.is_empty(),
        "[cargo] workspaces: an entry is empty; use \".\" for the root workspace"
    );
    let path = Path::new(entry);
    anyhow::ensure!(
        path.is_relative()
            && path.components().all(|c| matches!(c, Component::Normal(_) | Component::CurDir)),
        "[cargo] workspaces: '{entry}' must be a relative directory inside the project, \
         without '..'"
    );
    Ok(())
}

/// Refuse an entry whose directory or manifest resolves outside `root` (the
/// canonical project root) through a symlink. A path that does not exist
/// yet is left for cargo to report.
fn validate_resolved(root: &Path, entry: &str) -> anyhow::Result<()> {
    let dir = root.join(entry);
    for path in [dir.join("Cargo.toml"), dir] {
        match std::fs::canonicalize(&path) {
            Ok(resolved) => anyhow::ensure!(
                resolved.starts_with(root),
                "[cargo] workspaces: '{entry}' resolves outside the project ({}); ops does not \
                 follow symlinks out of the workspace",
                resolved.display()
            ),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => {
                return Err(e).with_context(|| {
                    format!("[cargo] workspaces: resolving '{entry}' ({})", path.display())
                })
            }
        }
    }
    Ok(())
}

/// Whether a config command is a verbatim copy of the stack default, as
/// `ops init --commands` writes it. Compared through the serialized form:
/// the spec has no `PartialEq`, and the serialized fields are exactly what a
/// config file can express.
pub(super) fn is_default_copy<T: serde::Serialize>(spec: &T, default: &T) -> bool {
    match (toml::Value::try_from(spec), toml::Value::try_from(default)) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

fn fan_out(spec: &mut ExecCommandSpec, workspaces: &[String]) {
    super::extend::append_exec_args(
        &mut spec.args,
        &["--manifest-path".to_string(), format!("${{matrix.{AXIS}}}/Cargo.toml")],
    );
    spec.strategy = Some(Strategy::new(Matrix {
        axes: IndexMap::from([(AXIS.to_string(), workspaces.to_vec())]),
        ..Matrix::default()
    }));
}

/// Apply `[cargo] workspaces` to `config`.
///
/// # Errors
///
/// An entry is empty, absolute, climbs out of the project with `..`, or
/// resolves outside it through a symlink; or resolving one fails for a
/// reason other than its absence.
pub(super) fn apply(config: &mut Config, workspace_root: &Path) -> anyhow::Result<()> {
    let workspaces = config.cargo.workspaces().to_vec();
    let root =
        std::fs::canonicalize(workspace_root).unwrap_or_else(|_| workspace_root.to_path_buf());
    for entry in &workspaces {
        validate_entry(entry)?;
        validate_resolved(&root, entry)?;
    }
    if workspaces.is_empty() || workspaces == ["."] {
        return Ok(());
    }
    let Some(stack) = crate::stack::Stack::resolve(config.stack.as_deref(), workspace_root) else {
        return Ok(());
    };
    let defaults = stack.default_commands_ref();
    for &name in WORKSPACE_COMMANDS {
        let Some(CommandSpec::Exec(default)) = defaults.get(name) else {
            continue;
        };
        let materialized = config.provenance.extended_stack_defaults.iter().any(|n| n == name);
        match config.commands.get_mut(name) {
            Some(CommandSpec::Exec(exec))
                if exec.strategy.is_none() && (materialized || is_default_copy(exec, default)) =>
            {
                fan_out(exec, &workspaces);
            }
            Some(_) => {}
            None => {
                let mut exec = default.clone();
                fan_out(&mut exec, &workspaces);
                config.commands.insert(name.to_string(), CommandSpec::Exec(exec));
                config.provenance.extended_stack_defaults.push(name.to_string());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_workspace() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), "[package]\nname = \"x\"\n").unwrap();
        dir
    }

    fn with_workspaces(list: &[&str]) -> Config {
        let mut config = Config::empty();
        config.cargo.workspaces = Some(list.iter().map(ToString::to_string).collect());
        config
    }

    fn exec<'a>(config: &'a Config, name: &str) -> &'a ExecCommandSpec {
        match config.commands.get(name) {
            Some(CommandSpec::Exec(e)) => e,
            other => panic!("{name} must be a materialized exec command, got {other:?}"),
        }
    }

    #[test]
    fn unset_or_root_only_changes_nothing() {
        let dir = rust_workspace();
        for list in [&[][..], &["."][..]] {
            let mut config = with_workspaces(list);
            apply(&mut config, dir.path()).unwrap();
            assert!(config.commands.is_empty(), "{list:?}");
            assert!(config.provenance.extended_stack_defaults.is_empty());
        }
    }

    #[test]
    fn fans_every_workspace_command_out_over_the_list() {
        let dir = rust_workspace();
        let mut config = with_workspaces(&[".", "fuzz"]);
        apply(&mut config, dir.path()).unwrap();

        for name in WORKSPACE_COMMANDS {
            let spec = exec(&config, name);
            let cells = spec.matrix_cells(name).unwrap();
            let ids: Vec<&str> = cells.iter().map(|c| c.id.as_str()).collect();
            assert_eq!(
                ids,
                [format!("{name} [workspace=.]"), format!("{name} [workspace=fuzz]")],
                "{name}"
            );
            let fuzz = &cells[1].spec.args;
            let at = fuzz.iter().position(|a| a == "--manifest-path").unwrap();
            assert_eq!(fuzz[at + 1], "fuzz/Cargo.toml", "{name}: {fuzz:?}");
            assert!(config.provenance.extended_stack_defaults.iter().any(|n| n == name));
        }
        for root_only in ["test", "next", "doc", "verify"] {
            assert!(!config.commands.contains_key(root_only), "{root_only}");
        }
    }

    /// `--manifest-path` must stay a cargo flag: after `--` it would reach
    /// clippy-driver or rustfmt.
    #[test]
    fn manifest_path_goes_before_the_separator() {
        let dir = rust_workspace();
        let mut config = with_workspaces(&[".", "fuzz"]);
        apply(&mut config, dir.path()).unwrap();
        for name in ["clippy", "fmt-check"] {
            let args = &exec(&config, name).args;
            let sep = args.iter().position(|a| a == "--").unwrap();
            let flag = args.iter().position(|a| a == "--manifest-path").unwrap();
            assert!(flag < sep, "{name}: {args:?}");
        }
    }

    #[test]
    fn config_command_of_the_same_name_is_left_alone() {
        let dir = rust_workspace();
        let mut config = with_workspaces(&[".", "fuzz"]);
        config.commands.insert(
            "clippy".into(),
            CommandSpec::Exec(ExecCommandSpec::new("cargo", ["clippy", "-p", "mine"])),
        );
        apply(&mut config, dir.path()).unwrap();
        let clippy = exec(&config, "clippy");
        assert_eq!(clippy.args, ["clippy", "-p", "mine"]);
        assert!(clippy.strategy.is_none());
    }

    /// A stack default `[extend]` materialized keeps its extra args and is
    /// expanded in place.
    #[test]
    fn extended_stack_default_is_expanded_in_place() {
        let dir = rust_workspace();
        let mut config = with_workspaces(&[".", "fuzz"]);
        config.commands.insert(
            "build".into(),
            CommandSpec::Exec(ExecCommandSpec::new("cargo", ["build", "--release"])),
        );
        config.provenance.extended_stack_defaults.push("build".into());
        apply(&mut config, dir.path()).unwrap();
        let build = exec(&config, "build");
        assert_eq!(
            build.args,
            ["build", "--release", "--manifest-path", "${matrix.workspace}/Cargo.toml"]
        );
        assert!(build.strategy.is_some());
        assert_eq!(
            config.provenance.extended_stack_defaults.iter().filter(|n| *n == "build").count(),
            1
        );
    }

    /// `ops init --commands` writes each default verbatim; such a copy
    /// fans out like the default it copies.
    #[test]
    fn verbatim_copy_of_the_default_fans_out() {
        let dir = rust_workspace();
        let mut config = with_workspaces(&[".", "fuzz"]);
        let default =
            crate::stack::Stack::Rust.default_commands_ref().get("clippy").cloned().unwrap();
        let written = toml::to_string(&default).unwrap();
        let copy: CommandSpec = toml::from_str(&written).unwrap();
        config.commands.insert("clippy".into(), copy);
        apply(&mut config, dir.path()).unwrap();
        assert!(exec(&config, "clippy").strategy.is_some());
        assert!(
            !config.provenance.extended_stack_defaults.iter().any(|n| n == "clippy"),
            "a config command stays a config command"
        );
    }

    #[cfg(unix)]
    #[test]
    fn rejects_an_entry_that_symlinks_out_of_the_project() {
        let dir = rust_workspace();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        std::os::unix::fs::symlink(outside.path(), dir.path().join("fuzz")).unwrap();
        let mut config = with_workspaces(&[".", "fuzz"]);
        let err = apply(&mut config, dir.path()).unwrap_err().to_string();
        assert!(err.contains("resolves outside the project"), "{err}");

        let inner = dir.path().join("real-fuzz");
        std::fs::create_dir(&inner).unwrap();
        std::fs::write(inner.join("Cargo.toml"), "[workspace]\n").unwrap();
        std::os::unix::fs::symlink(&inner, dir.path().join("alias")).unwrap();
        let mut config = with_workspaces(&[".", "alias"]);
        apply(&mut config, dir.path()).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn rejects_a_manifest_that_symlinks_out_of_the_project() {
        let dir = rust_workspace();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("Cargo.toml"), "[workspace]\n").unwrap();
        std::fs::create_dir(dir.path().join("fuzz")).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("Cargo.toml"),
            dir.path().join("fuzz/Cargo.toml"),
        )
        .unwrap();
        let mut config = with_workspaces(&[".", "fuzz"]);
        let err = apply(&mut config, dir.path()).unwrap_err().to_string();
        assert!(err.contains("resolves outside the project"), "{err}");
    }

    #[test]
    fn non_rust_stack_is_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = with_workspaces(&[".", "fuzz"]);
        apply(&mut config, dir.path()).unwrap();
        assert!(config.commands.is_empty());
    }

    #[test]
    fn rejects_entries_that_leave_the_project() {
        let dir = rust_workspace();
        for bad in ["", "/abs", "../sibling", "fuzz/../.."] {
            let mut config = with_workspaces(&[".", bad]);
            let err = apply(&mut config, dir.path()).unwrap_err().to_string();
            assert!(err.contains("[cargo] workspaces"), "{bad:?}: {err}");
        }
        for good in ["fuzz", "./fuzz", "tools/fuzz/"] {
            let mut config = with_workspaces(&[".", good]);
            apply(&mut config, dir.path()).unwrap();
        }
    }
}
