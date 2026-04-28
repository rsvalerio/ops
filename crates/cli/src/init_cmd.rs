//! Handler for the `ops init` command.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use ops_core::config::atomic_write;

pub(crate) fn run_init(
    force: bool,
    sections: ops_core::config::InitSections,
) -> anyhow::Result<()> {
    run_init_to(force, sections, &mut std::io::stdout())
}

fn run_init_to(
    force: bool,
    sections: ops_core::config::InitSections,
    w: &mut dyn Write,
) -> anyhow::Result<()> {
    let path = PathBuf::from(".ops.toml");
    let cwd = std::env::current_dir()?;
    let content = ops_core::config::init_template(&cwd, &sections)?;
    match write_init(&path, content.as_bytes(), force) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            tracing::warn!(
                "{} already exists; not overwriting (use --force to overwrite)",
                path.display()
            );
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    }
    tracing::info!("created {}", path.display());
    if sections.commands {
        let stack = ops_core::stack::Stack::detect(&cwd);
        if stack.is_some() {
            writeln!(
                w,
                "Created .ops.toml with default commands for the detected stack. Run `cargo ops <command>` (e.g. cargo ops build, cargo ops verify)."
            )?;
        } else {
            writeln!(w, "Created .ops.toml. Add commands in [commands.<name>] or run in a project with a detected stack, then run `cargo ops <command>`.")?;
        }
    } else {
        writeln!(w, "Created .ops.toml with output settings. Use `ops init --commands --themes` to include more sections.")?;
    }
    Ok(())
}

/// SEC-25 / TASK-0409: collapse the prior `path.exists()` + `fs::write`
/// pair into atomic primitives.
///
/// - Without `--force`, `OpenOptions::create_new` fails with
///   `AlreadyExists` if the target is present, so an attacker (or a racing
///   second `ops init`) cannot insert the file between the existence check
///   and the write.
/// - With `--force`, the user has explicitly asked to clobber. Delegate to
///   `ops_core::config::atomic_write` so the staged-temp + `rename(2)` +
///   parent-dir fsync hardening stays in one place.
fn write_init(path: &Path, bytes: &[u8], force: bool) -> std::io::Result<()> {
    if !force {
        let mut f = OpenOptions::new().write(true).create_new(true).open(path)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        return Ok(());
    }
    atomic_write(path, bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CwdGuard;

    fn all_sections() -> ops_core::config::InitSections {
        ops_core::config::InitSections::from_flags(true, true, true)
    }

    fn default_sections() -> ops_core::config::InitSections {
        ops_core::config::InitSections::from_flags(false, false, false)
    }

    #[test]
    fn run_init_creates_minimal_ops_toml() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        run_init(false, default_sections()).expect("run_init should succeed");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(
            content.contains("[output]"),
            "should contain output section"
        );
        assert!(
            !content.contains("[themes.classic]"),
            "default init should not contain themes"
        );
    }

    #[test]
    fn run_init_all_sections_includes_themes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        run_init(false, all_sections()).expect("run_init should succeed");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(
            content.contains("[output]"),
            "should contain output section"
        );
        assert!(
            content.contains("[themes.classic]"),
            "should contain classic theme"
        );
    }

    #[test]
    fn run_init_no_overwrite_without_force() {
        let (dir, _guard) = crate::test_utils::with_temp_config("existing");
        run_init(false, default_sections()).expect("run_init should succeed (noop)");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert_eq!(content, "existing", "file should not be overwritten");
    }

    #[test]
    fn run_init_force_overwrites() {
        let (dir, _guard) = crate::test_utils::with_temp_config("existing");
        run_init(true, default_sections()).expect("run_init should succeed");
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(
            content.contains("[output]"),
            "file should be overwritten with defaults"
        );
    }

    #[test]
    fn run_init_to_output_message_no_flags() {
        let dir = tempfile::tempdir().expect("tempdir");
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        let mut buf = Vec::new();
        run_init_to(false, default_sections(), &mut buf).expect("run_init_to");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("Created .ops.toml with output settings"),
            "expected minimal message, got: {output}"
        );
    }

    #[test]
    fn run_init_force_overwrite_message() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join(".ops.toml"), "existing").unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        let mut buf = Vec::new();
        run_init_to(true, default_sections(), &mut buf).expect("run_init_to");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("Created .ops.toml"),
            "force overwrite should produce creation message, got: {output}"
        );
        // Verify the file was actually overwritten
        let content = std::fs::read_to_string(dir.path().join(".ops.toml")).unwrap();
        assert!(
            content.contains("[output]"),
            "should be new content, got: {content}"
        );
    }

    #[test]
    fn run_init_to_output_message_commands_no_stack() {
        let dir = tempfile::tempdir().expect("tempdir");
        // No Cargo.toml, no package.json — no stack detected
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        let mut buf = Vec::new();
        let sections = ops_core::config::InitSections::from_flags(true, false, true);
        run_init_to(false, sections, &mut buf).expect("run_init_to");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("Add commands"),
            "no-stack message expected, got: {output}"
        );
    }

    #[test]
    fn run_init_to_output_message_with_commands_and_rust_stack() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Write a Cargo.toml so Stack::detect returns Some(Rust)
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname = \"test\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let _guard = CwdGuard::new(dir.path()).expect("CwdGuard");
        let mut buf = Vec::new();
        let sections = ops_core::config::InitSections::from_flags(true, false, true);
        run_init_to(false, sections, &mut buf).expect("run_init_to");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("detected stack"),
            "expected stack message, got: {output}"
        );
    }
}
