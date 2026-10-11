//! Handler for `ops init --rust` and `ops init --rust --check`.
//!
//! The templates, the scaffold and the semantic drift check live in
//! `ops-rust-foundation`; this module resolves the project root, feeds in the
//! `[foundation.waivers]` from `.ops.toml`, and renders the report.

use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use indexmap::IndexMap;
use ops_core::config::Config;
use ops_rust_foundation::{Outcome, Report};

const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run `ops init --rust` / `ops init --rust --check`: scaffold or
/// drift-check the Rust foundation files via `ops_rust_foundation`.
pub fn run_init_rust(force: bool, check: bool, config: &Config) -> anyhow::Result<ExitCode> {
    let cwd = crate::cwd()?;
    let mut out = std::io::stdout().lock();
    if check {
        run_check_to(&cwd, &config.foundation.waivers, &mut out)
    } else {
        run_scaffold_to(&cwd, force, &mut out)?;
        Ok(ExitCode::SUCCESS)
    }
}

fn run_scaffold_to(root: &Path, force: bool, w: &mut dyn Write) -> anyhow::Result<()> {
    for written in ops_rust_foundation::scaffold(root, force)? {
        let target = safe(&written.target);
        match written.outcome {
            Outcome::Created => writeln!(w, "created  {target}")?,
            Outcome::Replaced => writeln!(w, "replaced {target}")?,
            Outcome::Added => writeln!(w, "added    {target}")?,
            Outcome::Kept => writeln!(
                w,
                "kept     {target} (already present; --force replaces it)"
            )?,
        }
    }
    writeln!(
        w,
        "Rust foundation from ops {VERSION}. Check it any time with `ops init --rust --check`."
    )?;
    Ok(())
}

fn run_check_to(
    root: &Path,
    waivers: &IndexMap<String, String>,
    w: &mut dyn Write,
) -> anyhow::Result<ExitCode> {
    let report = ops_rust_foundation::check(root, waivers)?;
    render_report(&report, w)?;
    Ok(if report.drift.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// Locations, messages and reasons come from the repo's own files, so they
/// go through the shared display policy: a quoted TOML key cannot smuggle
/// ANSI or newlines onto the operator's terminal.
fn safe(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    ops_core::ui::sanitise_line(text, &mut out);
    out
}

fn render_report(report: &Report, w: &mut dyn Write) -> anyhow::Result<()> {
    for d in &report.drift {
        writeln!(w, "drift   {}: {}", safe(&d.location), safe(&d.message))?;
    }
    for (d, reason) in &report.waived {
        writeln!(
            w,
            "waived  {}: {} ({})",
            safe(&d.location),
            safe(&d.message),
            safe(reason)
        )?;
    }
    for key in &report.unused_waivers {
        writeln!(
            w,
            "unused  waiver {}: matches no drift, remove it from [foundation.waivers]",
            safe(key)
        )?;
    }
    let waived = report.waived.len();
    match report.drift.len() {
        0 => writeln!(
            w,
            "Rust foundation matches the ops {VERSION} templates ({waived} waived)."
        )?,
        n => writeln!(
            w,
            "Rust foundation: {n} drift(s) from the ops {VERSION} templates. Fix them, reset a \
             file with `ops init --rust --force`, or record why under [foundation.waivers] in \
             .ops.toml."
        )?,
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rust_project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"p\"\nversion = \"0.1.0\"\n",
        )
        .expect("manifest");
        dir
    }

    fn output(f: impl FnOnce(&mut Vec<u8>) -> anyhow::Result<()>) -> String {
        let mut buf = Vec::new();
        f(&mut buf).expect("command");
        String::from_utf8(buf).expect("utf8")
    }

    #[test]
    fn scaffold_reports_each_target_then_check_passes() {
        let dir = rust_project();
        let text = output(|w| run_scaffold_to(dir.path(), false, w));
        assert!(text.contains("created  clippy.toml\n"), "{text}");
        assert!(text.contains("added    Cargo.toml:lints\n"), "{text}");

        let mut buf = Vec::new();
        let code = run_check_to(dir.path(), &IndexMap::new(), &mut buf).expect("check");
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(String::from_utf8(buf)
            .expect("utf8")
            .contains("matches the ops"));
    }

    #[test]
    fn a_second_scaffold_keeps_everything() {
        let dir = rust_project();
        output(|w| run_scaffold_to(dir.path(), false, w));
        let text = output(|w| run_scaffold_to(dir.path(), false, w));
        assert!(
            text.contains("kept     deny.toml (already present; --force replaces it)"),
            "{text}"
        );
        assert!(!text.contains("created"), "{text}");
    }

    #[test]
    fn check_fails_on_drift_and_names_the_escape_hatches() {
        let dir = rust_project();
        let mut buf = Vec::new();
        let code = run_check_to(dir.path(), &IndexMap::new(), &mut buf).expect("check");
        let text = String::from_utf8(buf).expect("utf8");
        assert_eq!(code, ExitCode::FAILURE);
        assert!(text.contains("drift   clippy.toml: missing\n"), "{text}");
        assert!(text.contains("[foundation.waivers]"), "{text}");
    }

    #[test]
    fn waived_and_unused_waivers_are_listed() {
        let dir = rust_project();
        output(|w| run_scaffold_to(dir.path(), false, w));
        std::fs::remove_file(dir.path().join("rustfmt.toml")).expect("rm");
        let waivers = IndexMap::from([
            (
                "rustfmt.toml".to_owned(),
                "keeps rustfmt defaults".to_owned(),
            ),
            ("deny.toml:nope".to_owned(), "stale".to_owned()),
        ]);
        let mut buf = Vec::new();
        let code = run_check_to(dir.path(), &waivers, &mut buf).expect("check");
        let text = String::from_utf8(buf).expect("utf8");
        assert_eq!(code, ExitCode::SUCCESS);
        assert!(
            text.contains("waived  rustfmt.toml: missing (keeps rustfmt defaults)"),
            "{text}"
        );
        assert!(text.contains("unused  waiver deny.toml:nope"), "{text}");
        assert!(text.contains("(1 waived)"), "{text}");
    }

    #[test]
    fn hostile_keys_are_escaped_in_the_report() {
        let report = Report {
            drift: vec![ops_rust_foundation::Drift {
                location: "clippy.toml:a\u{1b}[31m\nb".to_owned(),
                message: "missing".to_owned(),
            }],
            ..Report::default()
        };
        let text = output(|w| render_report(&report, w));
        assert!(!text.contains('\u{1b}'), "{text:?}");
        assert_eq!(text.lines().count(), 2, "{text:?}");
    }
}
