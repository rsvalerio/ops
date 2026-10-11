//! `ops msrv`: build the workspace on its declared `rust-version`.
//!
//! `clippy::incompatible_msrv` (driven by `clippy.toml`'s `msrv`) catches
//! standard-library calls above the floor but is blind to language features;
//! only compiling on the floor covers both. The floor is read from
//! `Cargo.toml`, never written down a second time, and `clippy.toml`'s `msrv`
//! must agree with it — otherwise the lint and the build each let through
//! what the other rejects.

use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::process::{Command, ExitCode};

use anyhow::{bail, Context};

/// What `cargo check` runs with on the declared toolchain. `--all-targets`
/// so test and bench code is type-checked too.
const CHECK_ARGS: &[&str] = &[
    "cargo",
    "check",
    "--workspace",
    "--all-features",
    "--all-targets",
];

/// `ops msrv`: verify `clippy.toml`'s `msrv` matches `rust-version`, then
/// `cargo check` the workspace on that toolchain (installed first under
/// `install`). `dry_run` prints the plan after the agreement check.
///
/// # Errors
///
/// No `rust-version`, a malformed one, a missing or mismatched clippy
/// `msrv`, or a failure to spawn rustup.
pub fn run_msrv(root: &Path, install: bool, dry_run: bool) -> anyhow::Result<ExitCode> {
    let version = checked_msrv(root)?;
    let rustup = ops_core::subprocess::resolve_rustup_bin();
    let mut steps: Vec<Vec<OsString>> = Vec::new();
    if install {
        steps.push(os(&[
            "toolchain",
            "install",
            &version,
            "--profile",
            "minimal",
        ]));
    }
    let mut check = os(&["run", &version]);
    check.extend(os(CHECK_ARGS));
    steps.push(check);

    let mut out = std::io::stdout().lock();
    writeln!(
        out,
        "rust-version {version} matches clippy.toml msrv; checking on {version}"
    )?;
    for args in steps {
        if dry_run {
            writeln!(out, "{}", render(&rustup, &args))?;
            continue;
        }
        out.flush()?;
        let status = Command::new(&rustup)
            .args(&args)
            .current_dir(root)
            // A parent `cargo run` / cargo-subcommand context would point the
            // nested cargo at the parent toolchain's binaries, silently
            // building on the wrong compiler.
            .env_remove("CARGO")
            .env_remove("RUSTC")
            .status()
            .with_context(|| format!("failed to spawn {}", render(&rustup, &args)))?;
        if !status.success() {
            writeln!(out, "{} failed ({status})", render(&rustup, &args))?;
            return Ok(ExitCode::FAILURE);
        }
    }
    Ok(ExitCode::SUCCESS)
}

/// The declared floor, after checking `clippy.toml` agrees with it.
fn checked_msrv(root: &Path) -> anyhow::Result<String> {
    let manifest_path = root.join("Cargo.toml");
    let manifest = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("reading {}", manifest_path.display()))?;
    let declared = declared_rust_version(&manifest)?;
    let (clippy_file, configured) = clippy_msrv(root)?;
    match configured {
        Some(configured) if same_version(&declared, &configured) => Ok(declared),
        Some(configured) => bail!(
            "MSRV mismatch: Cargo.toml rust-version is {declared:?} but {clippy_file} msrv is \
             {configured:?}; set them equal"
        ),
        None => bail!(
            "{clippy_file} sets no msrv; add `msrv = {declared:?}` so clippy::incompatible_msrv \
             gates the same floor as the build"
        ),
    }
}

/// `rust-version` from `[workspace.package]`, else from `[package]`.
fn declared_rust_version(manifest: &str) -> anyhow::Result<String> {
    let doc: toml::Table = toml::from_str(manifest).context("parsing Cargo.toml")?;
    let rust_version = |table: Option<&toml::Value>| table?.get("rust-version").cloned();
    let value = rust_version(doc.get("workspace").and_then(|w| w.get("package")))
        .or_else(|| rust_version(doc.get("package")))
        .context("Cargo.toml declares no rust-version ([workspace.package] or [package])")?;
    let Some(version) = value.as_str() else {
        bail!("Cargo.toml rust-version must be a string such as \"1.85\", found {value}");
    };
    validate(version)?;
    Ok(version.to_owned())
}

/// The file clippy reads (`.clippy.toml` first, as clippy does) and its
/// `msrv`, if set.
fn clippy_msrv(root: &Path) -> anyhow::Result<(&'static str, Option<String>)> {
    for name in [".clippy.toml", "clippy.toml"] {
        let path = root.join(name);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Err(e).with_context(|| format!("reading {}", path.display())),
        };
        let doc: toml::Table = toml::from_str(&text).with_context(|| format!("parsing {name}"))?;
        let msrv = match doc.get("msrv") {
            None => None,
            Some(toml::Value::String(s)) => Some(s.clone()),
            Some(other) => bail!("{name} msrv must be a string such as \"1.85\", found {other}"),
        };
        return Ok((name, msrv));
    }
    Ok(("clippy.toml", None))
}

/// Only `digits(.digits){0,2}` reaches rustup as a toolchain name.
fn validate(version: &str) -> anyhow::Result<()> {
    let parts: Vec<&str> = version.split('.').collect();
    let well_formed = (1..=3).contains(&parts.len())
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()));
    if !well_formed {
        bail!("rust-version {version:?} is not a version such as \"1.85\" or \"1.85.1\"");
    }
    Ok(())
}

/// `1.85` and `1.85.0` name the same floor.
fn same_version(a: &str, b: &str) -> bool {
    let norm = |v: &str| {
        let mut parts: Vec<&str> = v.trim().split('.').collect();
        while parts.len() > 2 && parts.last() == Some(&"0") {
            parts.pop();
        }
        parts.join(".")
    };
    norm(a) == norm(b)
}

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

fn render(program: &OsString, args: &[OsString]) -> String {
    let words: Vec<String> = std::iter::once(program)
        .chain(args)
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    shlex::try_join(words.iter().map(String::as_str)).unwrap_or_else(|_| words.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root(cargo: &str, clippy: Option<(&str, &str)>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("Cargo.toml"), cargo).unwrap();
        if let Some((name, text)) = clippy {
            std::fs::write(dir.path().join(name), text).unwrap();
        }
        dir
    }

    const WORKSPACE: &str = "[workspace.package]\nrust-version = \"1.85\"\n";

    #[test]
    fn reads_workspace_then_package_rust_version() {
        assert_eq!(declared_rust_version(WORKSPACE).unwrap(), "1.85");
        assert_eq!(
            declared_rust_version("[package]\nname = \"x\"\nrust-version = \"1.80.1\"\n").unwrap(),
            "1.80.1"
        );
        let inherited = "[workspace.package]\nrust-version = \"1.90\"\n\
                         [package]\nrust-version.workspace = true\n";
        assert_eq!(declared_rust_version(inherited).unwrap(), "1.90");
    }

    #[test]
    fn missing_or_malformed_rust_version_is_an_error() {
        let err = declared_rust_version("[package]\nname = \"x\"\n").unwrap_err();
        assert!(err.to_string().contains("no rust-version"), "{err}");
        for bad in ["stable", "1.85; rm -rf /", "1..2", "1.2.3.4", ""] {
            let manifest = format!("[package]\nrust-version = {bad:?}\n");
            assert!(declared_rust_version(&manifest).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn matching_clippy_msrv_passes() {
        let dir = root(WORKSPACE, Some(("clippy.toml", "msrv = \"1.85.0\"\n")));
        assert_eq!(checked_msrv(dir.path()).unwrap(), "1.85");
    }

    #[test]
    fn dot_clippy_toml_takes_precedence() {
        let dir = root(WORKSPACE, Some((".clippy.toml", "msrv = \"1.85\"\n")));
        std::fs::write(dir.path().join("clippy.toml"), "msrv = \"1.70\"\n").unwrap();
        assert_eq!(checked_msrv(dir.path()).unwrap(), "1.85");
    }

    #[test]
    fn mismatched_clippy_msrv_fails() {
        let dir = root(WORKSPACE, Some(("clippy.toml", "msrv = \"1.84\"\n")));
        let err = checked_msrv(dir.path()).unwrap_err().to_string();
        assert!(err.contains("MSRV mismatch"), "{err}");
    }

    #[test]
    fn unset_clippy_msrv_fails() {
        for clippy in [
            None,
            Some(("clippy.toml", "too-many-lines-threshold = 80\n")),
        ] {
            let dir = root(WORKSPACE, clippy);
            let err = checked_msrv(dir.path()).unwrap_err().to_string();
            assert!(err.contains("sets no msrv"), "{err}");
        }
    }

    #[test]
    fn same_version_ignores_trailing_zero_patch() {
        assert!(same_version("1.85", "1.85.0"));
        assert!(same_version("1.85.0", "1.85"));
        assert!(!same_version("1.85", "1.85.1"));
        assert!(!same_version("1.8", "1.80"));
    }

    #[test]
    fn dry_run_checks_agreement_without_spawning() {
        let dir = root(WORKSPACE, Some(("clippy.toml", "msrv = \"1.85\"\n")));
        assert_eq!(run_msrv(dir.path(), true, true).unwrap(), ExitCode::SUCCESS);
        let bad = root(WORKSPACE, Some(("clippy.toml", "msrv = \"1.84\"\n")));
        assert!(run_msrv(bad.path(), false, true).is_err());
    }
}
