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
        // SEC-25 (TASK-0730): persist the new directory entry so a crash
        // between this return and the next sync(2) does not lose the
        // `.ops.toml` link on ext4/xfs. The --force branch already gets
        // this via `atomic_write`'s parent fsync (TASK-0340); the no-force
        // path is the common case (first run in a clean repo), so the
        // asymmetry was the loud bug. We cannot fold this branch into
        // `atomic_write` without losing the `create_new` exclusion that
        // gives no-force its "do not clobber" guarantee, hence the
        // inline parent fsync mirroring config::edit::atomic_write.
        #[cfg(unix)]
        if let Some(parent) = path.parent() {
            // Empty parent path means cwd; open(".") instead.
            let parent = if parent.as_os_str().is_empty() {
                Path::new(".")
            } else {
                parent
            };
            // ERR-1 / TASK-1096: mirror edit::atomic_write's TASK-0899 fix —
            // a failing parent fsync (ENOSPC, EIO) is non-fatal because the
            // file write has already returned success, but it is the only
            // signal that crash-safety is currently broken. Warn rather than
            // swallow. The parent open failure is also surfaced for the same
            // reason: silently skipping the fsync hides the regression.
            match std::fs::File::open(parent) {
                Ok(dir) => {
                    if let Err(e) = dir.sync_all() {
                        tracing::warn!(
                            parent = %parent.display(),
                            error = %e,
                            "directory fsync after .ops.toml create failed; new file may not survive a power loss"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        parent = %parent.display(),
                        error = %e,
                        "could not open parent directory to fsync after .ops.toml create; new file may not survive a power loss"
                    );
                }
            }
        }
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

    /// SEC-25 (TASK-0730): both write_init branches must reach the parent
    /// fsync codepath and produce byte-identical output for the same input,
    /// so a crash between file-fsync and the next sync(2) is the only
    /// scenario in which the directory entry could be lost — and that
    /// scenario is now covered by the parent fsync on both branches. Direct
    /// fault injection is impractical from a test, so we pin the symmetry
    /// of the success path instead.
    #[test]
    fn write_init_force_and_no_force_produce_identical_bytes() {
        let dir_a = tempfile::tempdir().expect("tempdir a");
        let path_a = dir_a.path().join(".ops.toml");
        let bytes = b"[output]\ntheme = \"compact\"\n";
        write_init(&path_a, bytes, false).expect("no-force write");

        let dir_b = tempfile::tempdir().expect("tempdir b");
        let path_b = dir_b.path().join(".ops.toml");
        // Pre-existing file so the --force branch actually clobbers.
        std::fs::write(&path_b, b"old content").expect("seed");
        write_init(&path_b, bytes, true).expect("force write");

        let a = std::fs::read(&path_a).expect("read a");
        let b = std::fs::read(&path_b).expect("read b");
        assert_eq!(
            a, b,
            "force and no-force paths must produce identical bytes"
        );
        assert_eq!(a, bytes, "and the bytes must be exactly what was written");
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
