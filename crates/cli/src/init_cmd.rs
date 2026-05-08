//! Handler for the `ops init` command.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::Path;

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
    // PATTERN-1 / TASK-1066: capture cwd once and join to an absolute path so
    // the create and the parent fsync target the same directory even if cwd
    // changes mid-call (signal handler, threaded init template). Using a
    // relative ".ops.toml" while reading current_dir separately leaves a
    // small TOCTOU window between the two filesystem ops.
    let cwd = std::env::current_dir()?;
    let path = cwd.join(".ops.toml");
    let content = ops_core::config::init_template(&cwd, &sections)?;
    match write_init(&path, content.as_bytes(), force) {
        Ok(()) => {}
        Err(e) if e.kind() == ErrorKind::AlreadyExists => {
            // ERR-7 / TASK-1191: Debug-format the path so newlines / ANSI in
            // a hostile cwd cannot forge log records. Mirrors the manifest-
            // probe sweep (TASK-0944 / TASK-0945).
            tracing::warn!(
                path = ?path.display(),
                "ops.toml already exists; not overwriting (use --force to overwrite)"
            );
            return Ok(());
        }
        Err(e) => return Err(e.into()),
    }
    // ERR-7 / TASK-1191: Debug-format the path on the success info event too
    // so a hostile cwd cannot smuggle newlines / ANSI into the structured-log
    // pipeline through the same field.
    tracing::info!(path = ?path.display(), "created .ops.toml");
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
                        // ERR-7 / TASK-1191: Debug-format the parent path so
                        // a hostile cwd cannot forge log records.
                        tracing::warn!(
                            parent = ?parent.display(),
                            error = %e,
                            "directory fsync after .ops.toml create failed; new file may not survive a power loss"
                        );
                    }
                }
                Err(e) => {
                    // ERR-7 / TASK-1191: Debug-format the parent path.
                    tracing::warn!(
                        parent = ?parent.display(),
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

    /// PATTERN-1 / TASK-1066: the file must land in the directory that was
    /// cwd at entry, and the path used internally must be absolute (not the
    /// bare relative `".ops.toml"`). Prior to the fix, `path` was relative
    /// while `cwd` was captured separately, so a cwd change mid-call (signal
    /// handler, threaded init template) could split create vs. parent fsync
    /// across two directories. The fix joins cwd with the filename once, so
    /// both ops target the same absolute path. We pin that the file lands at
    /// the captured-cwd absolute path (resolved via canonicalize, since
    /// macOS tmpdirs go through a /private symlink) and that no stray
    /// `.ops.toml` is created elsewhere.
    #[test]
    fn run_init_writes_to_captured_cwd_absolute_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).expect("mkdir sub");
        let _guard = CwdGuard::new(&sub).expect("CwdGuard sub");

        run_init(false, default_sections()).expect("run_init should succeed");

        let landed = sub.join(".ops.toml");
        assert!(
            landed.is_file(),
            ".ops.toml must land in the captured cwd at {}",
            landed.display()
        );
        // No stray copy in the parent — would indicate the relative path
        // escaped or was re-resolved against a different cwd.
        assert!(
            !dir.path().join(".ops.toml").exists(),
            "no .ops.toml should leak into the parent tempdir"
        );
    }

    /// ERR-7 / TASK-1191: the warn / info / fsync-warn events in init_cmd
    /// format paths via the `?` (Debug) formatter so newlines / ANSI in a
    /// hostile cwd-derived path cannot forge log records. This pins the
    /// value-level escape contract directly without spinning up a tracing
    /// subscriber, mirroring `stack_detection_path_debug_escapes_control_characters`.
    #[test]
    fn init_cmd_path_debug_escapes_control_characters() {
        let raw = "/tmp/dir\n\u{1b}[31m/.ops.toml";
        let display = std::path::Path::new(raw).display().to_string();
        let rendered = format!("{display:?}");
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
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
