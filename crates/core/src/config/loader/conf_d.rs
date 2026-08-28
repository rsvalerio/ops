//! ARCH-1 / TASK-1471: `.ops.d/*.toml` overlay walking and merging.
//!
//! Extracted from the historical grab-bag `loader.rs`. Owns the directory
//! walk ([`read_conf_d_files`]) and the per-file merge loop
//! ([`merge_conf_d`]) with the "loud failure" contract for parse / IO /
//! broken-symlink overlays.

use std::path::{Path, PathBuf};

use anyhow::Context;
use tracing::debug;

use super::super::{merge::merge_config, Config};

/// Read sorted `.toml` files from a directory.
///
/// Returns `Ok(None)` only when the directory itself does not exist —
/// every other failure (permission flip, racing rename on a `DirEntry`,
/// `read_dir` IO error) is surfaced as an `Err` so the layered-config
/// load fails loudly. See [`merge_conf_d`] for the "loud failure"
/// contract.
///
/// ERR-7 / TASK-1400: a `DirEntry` whose `?` access fails used to be
/// dropped with a warn-and-skip; this asymmetry meant a permission flip
/// or racing rename on a single overlay file made it disappear while
/// the rest of the merge proceeded, producing a config that differed
/// from what the operator authored.
fn read_conf_d_files(dir: &Path) -> anyhow::Result<Option<Vec<PathBuf>>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => {
            return Err(e)
                .with_context(|| format!("failed to read .ops.d directory: {:?}", dir.display()));
        }
    };
    let mut files: Vec<PathBuf> = Vec::new();
    for entry in entries {
        let entry = entry.with_context(|| {
            format!(
                "failed to read entry in .ops.d directory: {:?}",
                dir.display()
            )
        })?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "toml") {
            files.push(path);
        }
    }
    files.sort();
    Ok(Some(files))
}

/// Merge every `.ops.d/*.toml` overlay, in sorted order.
///
/// ERR-1: a parse or IO error on any single overlay file surfaces as a hard
/// error with the offending path in context rather than being silently
/// dropped. Users whose overlay "mysteriously does nothing" in CI should see
/// a loud failure instead of a tracing warning that gets swallowed.
///
/// ERR-4 / TASK-1448 + TEST-1 / TASK-1852: the `Ok(None)` arm below is a hard
/// error rather than benign absence. `read_dir` already proved the entry
/// existed, so an entry that then reads as "missing" means the tree changed
/// under us — the "loud failure" contract, not absence.
///
/// **What actually reaches that arm today.** The original ERR-4 wording named
/// the broken-symlink case, but SEC-25 / TASK-1468 put `O_NOFOLLOW` on the
/// config open, and a dangling symlink under `O_NOFOLLOW` returns `ELOOP`
/// (which `text::open_refusing_symlinks` maps to `InvalidInput`), not
/// `NotFound` — so a broken symlink now takes the `Err(e)` arm carrying the
/// symlink-refusal message. The only remaining route into `Ok(None)` is a
/// **delete or rename between `read_dir` and `open`**: a genuine TOCTOU race,
/// kept as defence in depth. It has no unit test because reproducing it needs
/// a scheduler-timed unlink inside the loop; `merge_conf_d_rejects_broken_\
/// symlink` pins the symlink route instead and asserts the *specific* message,
/// so deleting either branch fails the suite.
///
/// **Symlinks in `.ops.d` stay refused** — recorded decision, TASK-1852.
/// `.ops.d` is workspace-relative and therefore repo-supplied: the same
/// attacker-controlled trust boundary as `.ops.toml`. The SEC-14 / TASK-1847
/// relaxation applies only to the user's own `~/.config/ops/config.toml`; see
/// [`super::SymlinkPolicy`] for why the two differ. A repo that wants to share
/// config fragments should commit the fragment, not a link out of the
/// workspace.
pub(super) fn merge_conf_d(config: &mut Config, workspace_root: &Path) -> anyhow::Result<()> {
    let Some(files) = read_conf_d_files(&workspace_root.join(".ops.d"))? else {
        return Ok(());
    };
    for path in files {
        match super::read_config_file(&path) {
            Ok(Some(overlay)) => {
                debug!(path = ?path.display(), "merging conf.d config");
                merge_config(config, overlay);
            }
            Ok(None) => {
                anyhow::bail!(
                    "config overlay listed in .ops.d disappeared or is a broken symlink: {:?}",
                    path.display()
                );
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn read_conf_d_files_sorts_and_filters() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("b.toml"), "").unwrap();
        fs::write(dir.path().join("a.toml"), "").unwrap();
        fs::write(dir.path().join("readme.md"), "").unwrap();

        let files = read_conf_d_files(dir.path()).unwrap().unwrap();
        assert_eq!(files.len(), 2);
        assert!(files[0].ends_with("a.toml"));
        assert!(files[1].ends_with("b.toml"));
    }

    /// ERR-7 / TASK-0965: tracing fields for `.ops.d/*.toml` overlay paths
    /// flow through the `?` formatter so an attacker-controlled filename with
    /// embedded newlines / ANSI escapes cannot forge a log record. Mirrors the
    /// regression guard pattern used by the `manifest_declares_workspace` /
    /// hook-common ERR-7 sweep tests.
    #[test]
    fn conf_d_path_debug_escapes_control_characters() {
        let p = PathBuf::from("malicious\n[fake] info: pwned\u{1b}[31m.toml");
        let rendered = format!("{:?}", p.display());
        assert!(
            !rendered.contains('\n'),
            "raw newline must be escaped, got: {rendered}"
        );
        assert!(
            !rendered.contains('\u{1b}'),
            "ANSI escape must be escaped, got: {rendered}"
        );
        assert!(
            rendered.contains("\\n"),
            "newline must render as escape sequence, got: {rendered}"
        );
    }

    #[test]
    fn read_conf_d_files_missing_dir_returns_none() {
        let result = read_conf_d_files(std::path::Path::new("/nonexistent/ops.d")).unwrap();
        assert!(result.is_none());
    }

    #[test]
    #[serial_test::serial]
    fn merge_conf_d_applies_overlays() {
        let dir = tempfile::tempdir().unwrap();
        let ops_d = dir.path().join(".ops.d");
        fs::create_dir(&ops_d).unwrap();
        fs::write(
            ops_d.join("extra.toml"),
            r#"
[commands.extra]
program = "echo"
args = ["hello"]
"#,
        )
        .unwrap();

        let mut config = Config::default();
        merge_conf_d(&mut config, dir.path()).unwrap();

        assert!(config.commands.contains_key("extra"));
    }

    /// ERR-7 / TASK-1400: a `read_dir` failure (e.g. permission denied on
    /// the `.ops.d` directory itself) must surface as a hard error with the
    /// offending path attached rather than being warn-and-skipped. Mirrors
    /// the "loud failure" contract that already governs parse errors.
    #[cfg(unix)]
    #[test]
    fn read_conf_d_files_propagates_read_dir_error() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let unreadable = dir.path().join("locked");
        fs::create_dir(&unreadable).unwrap();
        // Strip read+execute bits so read_dir fails with EACCES.
        let mut perms = fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&unreadable, perms).unwrap();

        let result = read_conf_d_files(&unreadable);

        // Restore perms before asserting so tempdir cleanup succeeds.
        let mut restore = fs::metadata(&unreadable).unwrap().permissions();
        restore.set_mode(0o700);
        fs::set_permissions(&unreadable, restore).ok();

        let err = result.expect_err("unreadable .ops.d must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("locked") && msg.contains(".ops.d"),
            "error must name the offending directory, got: {msg}"
        );
    }

    /// ERR-4 / TASK-1448: a broken `.toml` symlink in `.ops.d` is listed by
    /// `read_dir` but fails to open at merge time. The "loud failure"
    /// contract on `merge_conf_d` requires this to abort the load, not to be
    /// silently mapped to `Ok(None)`.
    ///
    /// TEST-1 / TASK-1852: the assertion pins the **specific** error the code
    /// produces today — the SEC-25 `O_NOFOLLOW` refusal, since a dangling
    /// symlink returns `ELOOP` rather than `NotFound`. Asserting only that the
    /// message names `dangling.toml` was satisfied by *either* branch, so the
    /// `Ok(None)` bail this test is named for could have been deleted with the
    /// suite still green.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn merge_conf_d_rejects_broken_symlink() {
        let dir = tempfile::tempdir().unwrap();
        let ops_d = dir.path().join(".ops.d");
        fs::create_dir(&ops_d).unwrap();
        std::os::unix::fs::symlink(
            dir.path().join("does-not-exist.toml"),
            ops_d.join("dangling.toml"),
        )
        .unwrap();

        let mut config = Config::default();
        let result = merge_conf_d(&mut config, dir.path());

        let err = result.expect_err("broken symlink overlay must error");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("dangling.toml"),
            "error must name the broken overlay, got: {msg}"
        );
        assert!(
            msg.contains("refusing to follow symlink"),
            "a dangling symlink must take the SEC-25 refusal branch (ELOOP), \
             not the disappeared-overlay bail, got: {msg}"
        );
    }

    /// SEC-14 / TASK-1847 + TASK-1852 (recorded decision): the symlink
    /// relaxation for the user's own global config must NOT reach `.ops.d`.
    /// A `.ops.d` fragment symlinked to a real, readable file outside the
    /// workspace is still refused, because `.ops.d` is repo-supplied content.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn merge_conf_d_refuses_symlink_to_a_real_file() {
        let dir = tempfile::tempdir().unwrap();
        let ops_d = dir.path().join(".ops.d");
        fs::create_dir(&ops_d).unwrap();
        let target = dir.path().join("real.toml");
        fs::write(&target, "[commands.linked]\nprogram = \"echo\"\n").unwrap();
        std::os::unix::fs::symlink(&target, ops_d.join("linked.toml")).unwrap();

        let mut config = Config::default();
        let err = merge_conf_d(&mut config, dir.path())
            .expect_err("a symlinked .ops.d fragment must still be refused");

        assert!(
            format!("{err:#}").contains("refusing to follow symlink"),
            "workspace-relative config paths keep the SEC-25 policy, got: {err:#}"
        );
        assert!(
            !config.commands.contains_key("linked"),
            "the refused overlay must not have been merged"
        );
    }

    #[test]
    #[serial_test::serial]
    fn merge_conf_d_propagates_parse_error() {
        let dir = tempfile::tempdir().unwrap();
        let ops_d = dir.path().join(".ops.d");
        fs::create_dir(&ops_d).unwrap();
        fs::write(ops_d.join("broken.toml"), "not = = valid {{{").unwrap();

        let mut config = Config::default();
        let result = merge_conf_d(&mut config, dir.path());

        let err = result.expect_err("parse failure should surface");
        assert!(format!("{err:#}").contains("broken.toml"));
    }
}
