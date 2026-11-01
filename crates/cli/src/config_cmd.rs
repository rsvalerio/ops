//! `ops config check`: whether an `ops` release accepts this project's config.
//!
//! A `.ops.toml` field newer than the `ops` CI installs passes every local
//! gate when the developer's `ops` is newer still. This names the fields a
//! target release would reject, without installing it. The field history
//! lives in `ops_core::config`.

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use crate::args::ConfigAction;
use ops_core::config::{fields_used, minimum_version, mise_ops_pin, OpsVersion};

/// Run an `ops config` action.
///
/// # Errors
///
/// As the action; see [`run_config_check`].
pub fn run_config(root: &Path, action: &ConfigAction) -> anyhow::Result<ExitCode> {
    match action {
        ConfigAction::Check { target } => run_config_check(root, *target),
    }
}

/// `ops config check`: compare the config against `--as`, else `mise.toml`'s
/// `ops` pin, and always print the minimum accepting version. Without a
/// target there is nothing to reject, so that is all it reports.
///
/// # Errors
///
/// A config file or `mise.toml` cannot be read or parsed, or the pin names
/// no exact release and `--as` was not given.
fn run_config_check(root: &Path, target: Option<OpsVersion>) -> anyhow::Result<ExitCode> {
    run_check_to(root, target, &mut std::io::stdout().lock())
}

fn run_check_to(
    root: &Path,
    target: Option<OpsVersion>,
    w: &mut dyn Write,
) -> anyhow::Result<ExitCode> {
    let target = match target {
        Some(version) => Some((version, "--as")),
        None => mise_ops_pin(root)?.map(|version| (version, "mise.toml pin")),
    };
    let fields = fields_used(root)?;
    let minimum = minimum_version(&fields);
    let mut rejected = 0_usize;
    if let Some((version, source)) = target {
        for used in fields.iter().filter(|used| used.since > version) {
            rejected = rejected.saturating_add(1);
            let (file, field) = (safe(&used.file), safe(&used.field));
            writeln!(w, "{file}: {field} needs ops {}", used.since)?;
        }
        match rejected {
            0 => writeln!(w, "ops {version} ({source}) accepts this config.")?,
            n => writeln!(w, "ops {version} ({source}) rejects {n} field(s).")?,
        }
    }
    writeln!(w, "Minimum ops version for this config: {minimum}")?;
    Ok(if rejected == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Field names and fragment file names come from the repo's own files.
fn safe(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    ops_core::ui::sanitise_line(text, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project(ops_toml: &str, mise: Option<&str>) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), ops_toml).expect("write");
        if let Some(mise) = mise {
            std::fs::write(dir.path().join("mise.toml"), mise).expect("write");
        }
        dir
    }

    fn check(dir: &tempfile::TempDir, target: Option<&str>) -> (ExitCode, String) {
        let target = target.map(|t| t.parse().expect("version"));
        let mut out = Vec::new();
        let code = run_check_to(dir.path(), target, &mut out).expect("check");
        (code, String::from_utf8(out).expect("utf-8"))
    }

    const ARGS_BESIDE_CLONE: &str = "[commands.a]\nprogram = \"x\"\n\
                                     [commands.b]\nclone = \"a\"\nargs = [\"z\"]\n";

    #[test]
    fn as_names_the_rejected_field_its_release_and_the_minimum() {
        let dir = project(ARGS_BESIDE_CLONE, None);
        let (code, out) = check(&dir, Some("0.85.1"));
        assert_eq!(code, ExitCode::FAILURE);
        assert_eq!(
            out,
            ".ops.toml: commands.b.args (beside clone) needs ops 0.86.0\n\
             ops 0.85.1 (--as) rejects 1 field(s).\n\
             Minimum ops version for this config: 0.86.0\n"
        );
    }

    #[test]
    fn without_as_the_mise_pin_is_the_target() {
        let dir = project(ARGS_BESIDE_CLONE, Some("[tools]\nops = \"0.85.1\"\n"));
        let (code, out) = check(&dir, None);
        assert_eq!(code, ExitCode::FAILURE);
        assert!(out.contains("ops 0.85.1 (mise.toml pin) rejects"), "{out}");
    }

    #[test]
    fn as_wins_over_the_mise_pin() {
        let dir = project(ARGS_BESIDE_CLONE, Some("[tools]\nops = \"0.85.1\"\n"));
        let (code, out) = check(&dir, Some("0.86.0"));
        assert_eq!(code, ExitCode::SUCCESS);
        assert_eq!(
            out,
            "ops 0.86.0 (--as) accepts this config.\n\
             Minimum ops version for this config: 0.86.0\n"
        );
    }

    #[test]
    fn with_no_target_it_prints_the_minimum_and_passes() {
        let dir = project(ARGS_BESIDE_CLONE, None);
        let (code, out) = check(&dir, None);
        assert_eq!(code, ExitCode::SUCCESS);
        assert_eq!(out, "Minimum ops version for this config: 0.86.0\n");
    }

    #[test]
    fn a_hostile_key_stays_on_one_inert_line() {
        let dir = project(
            "[commands.\"a\\u001b[31m\\nb\"]\nprogram = \"x\"\nraw = true\n",
            None,
        );
        let (_, out) = check(&dir, Some("0.1.0"));
        assert!(!out.contains('\u{1b}'), "{out:?}");
        assert_eq!(out.lines().count(), 3, "{out:?}");
    }
}
