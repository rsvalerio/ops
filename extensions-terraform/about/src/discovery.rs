//! Filesystem discovery for the terraform about provider: which `.tf` files
//! are read for `required_version`, and which `modules/` children count as
//! local modules.
//!
//! IO follows the crate-level manifest IO policy: a missing file or
//! directory is silent, any other failure is logged at `warn`.

use std::path::{Path, PathBuf};

use crate::hcl_scan::extract_required_version;

/// Well-known `.tf` filenames probed before the directory walk.
const CANDIDATE_FILES: [&str; 4] = ["versions.tf", "main.tf", "terraform.tf", "version.tf"];

/// Scan `.tf` files for `required_version` in a `terraform` block.
///
/// Looks for patterns like `required_version = ">= 1.5"` or
/// `required_version = "~> 1.0"`. Only the first match is used.
pub fn find_required_version(root: &Path) -> Option<String> {
    for candidate in CANDIDATE_FILES {
        let path = root.join(candidate);
        // The shared helper warns on a permission-denied / EIO / "is a
        // directory" failure; only a missing file is silent.
        if let Some(content) = ops_about::manifest_io::read_optional_text(&path, root, candidate) {
            if let Some(v) = extract_required_version(&content, candidate) {
                return Some(v);
            }
        }
    }
    for path in fallback_tf_paths(root) {
        let kind = path
            .file_name()
            .map_or_else(|| "<unnamed>.tf".to_string(), |n| n.to_string_lossy().into_owned());
        if let Some(content) = ops_about::manifest_io::read_optional_text(&path, root, &kind) {
            if let Some(v) = extract_required_version(&content, &kind) {
                return Some(v);
            }
        }
    }
    None
}

/// The `.tf` files in `root` that [`find_required_version`]'s fallback walk
/// should read, in a deterministic order.
///
/// `read_dir` ordering is filesystem-dependent (ext4 hashes, APFS is
/// insertion-ish, Windows is alphabetical), so the paths are sorted: the
/// alphabetically-first `.tf` carrying a constraint is the reproducible winner
/// when several `.tf` files declare different `required_version` strings.
///
/// Files already probed by the named-candidate loop are skipped, so a
/// `main.tf` with no constraint is read and parsed once. The comparison is
/// exact-name: on a case-sensitive filesystem a `Main.TF` is a different file
/// that `root.join("main.tf")` never opened.
fn fallback_tf_paths(root: &Path) -> Vec<PathBuf> {
    let Some(entries) = read_dir_logged(root, "workspace root") else {
        return Vec::new();
    };
    let mut tf_paths: Vec<PathBuf> = entries
        .map(|entry| entry.path())
        .filter(|p| has_tf_extension(p))
        .filter(|p| !is_named_candidate(p))
        .collect();
    tf_paths.sort();
    tf_paths
}

/// Enumerate `dir` under the module-level IO policy, yielding the entries
/// that could be read.
///
/// A missing `dir` is the expected "nothing there" case and returns `None`
/// silently. Any other `read_dir` failure (permission denied, EIO, "not a
/// directory") is logged at `warn` and also returns `None`. A failure reading
/// one entry is logged at `warn` and that entry is skipped. `what` names the
/// directory's role in both messages.
fn read_dir_logged<'a>(
    dir: &'a Path,
    what: &'static str,
) -> Option<impl Iterator<Item = std::fs::DirEntry> + 'a> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::warn!(
                dir = ?dir.display(),
                error = %e,
                "failed to enumerate {what}"
            );
            return None;
        }
    };
    Some(entries.filter_map(move |res| match res {
        Ok(entry) => Some(entry),
        Err(e) => {
            tracing::warn!(
                dir = ?dir.display(),
                error = %e,
                "failed to read directory entry in {what}"
            );
            None
        }
    }))
}

/// Whether `path` has a `.tf` extension, compared ASCII-case-insensitively.
///
/// A `Custom.TF` / `Versions.Tf` file (preserved-case on macOS APFS, Windows
/// NTFS) is therefore found by the fallback walk, matching the targeted
/// candidate list, which already resolves case-insensitively via the
/// filesystem.
fn has_tf_extension(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|s| s.eq_ignore_ascii_case("tf"))
}

/// Whether `path`'s file name is one the named-candidate loop already read.
fn is_named_candidate(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| CANDIDATE_FILES.contains(&n))
}

/// Count local modules under `modules/*/`.
///
/// A subdirectory counts as a module when it contains at least one `.tf` (or
/// `.tf.json`) file. That is terraform's own definition — `main.tf` is a
/// convention, not a requirement — so `modules/network/network.tf` and
/// `modules/vpc/{variables,outputs,resources}.tf` layouts count.
///
/// Returns `None` when there are no local modules. A missing `modules/`
/// directory is that case and is silent; a real IO failure on it, on one of
/// its entries, or on a subdirectory is logged at `warn` per the module-level
/// IO policy ([`read_dir_logged`]).
pub fn count_local_modules(root: &Path) -> Option<usize> {
    let modules_dir = root.join("modules");
    let count = read_dir_logged(&modules_dir, "modules directory")?.filter(is_module_dir).count();
    (count > 0).then_some(count)
}

/// Whether a `modules/` child is a directory holding terraform sources.
///
/// Symlinks are followed (`fs::metadata`): a `modules/` entry that is a
/// symlink to a real module directory is a normal terraform layout — shared
/// modules vendored once and linked per stack — and counts as a module.
/// Nothing under the target is opened or written, and
/// [`contains_terraform_source`] only lists names, so following the link
/// grants no capability an operator running `ops about` in their own checkout
/// did not already have.
fn is_module_dir(entry: &std::fs::DirEntry) -> bool {
    match std::fs::metadata(entry.path()) {
        Ok(m) if !m.is_dir() => return false,
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
        Err(e) => {
            // An unreadable entry is an IO failure, not evidence that it is
            // not a module. A dangling symlink resolves to `NotFound` above
            // and is simply not a module.
            tracing::warn!(
                entry = ?entry.path().display(),
                error = %e,
                "failed to stat entry under modules/"
            );
            return false;
        }
    }
    contains_terraform_source(&entry.path())
}

/// Whether `dir` directly contains at least one `.tf` / `.tf.json` file.
///
/// A `dir` that cannot be enumerated holds no visible source and is `false`;
/// the failure itself is reported by [`read_dir_logged`].
fn contains_terraform_source(dir: &Path) -> bool {
    read_dir_logged(dir, "module directory").is_some_and(|mut entries| {
        entries.any(|entry| is_terraform_source_name(&entry.file_name()))
    })
}

/// Whether `name` is a terraform source file: `.tf` or `.tf.json` (terraform's
/// native JSON syntax), compared ASCII-case-insensitively for consistency with
/// [`has_tf_extension`].
fn is_terraform_source_name(name: &std::ffi::OsStr) -> bool {
    let path = Path::new(name);
    if has_tf_extension(path) {
        return true;
    }
    // `<name>.tf.json` — terraform's native JSON syntax. `Path` exposes only
    // the last extension, so the `.tf` half is read off the file stem.
    let is_json =
        path.extension().and_then(|e| e.to_str()).is_some_and(|e| e.eq_ignore_ascii_case("json"));
    is_json && path.file_stem().is_some_and(|stem| has_tf_extension(Path::new(stem)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_about::test_support::write_file as write;
    use ops_core::test_utils::capture_warn;

    /// When none of the well-known candidates matches
    /// and the fallback walks every `*.tf` in the workspace root, the
    /// chosen winner must be deterministic across platforms — we sort the
    /// directory listing by filename so the alphabetically-first .tf
    /// file that carries a `required_version` is always the one returned.
    #[test]
    fn find_required_version_fallback_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        // Pick filenames that fall outside the named-candidate list
        // (versions.tf, main.tf, terraform.tf, version.tf) so we exercise
        // the read_dir fallback. The alphabetically-first should win.
        write(
            &dir.path().join("a-providers.tf"),
            "terraform {\n  required_version = \"~> 1.5\"\n}\n",
        );
        write(
            &dir.path().join("z-extras.tf"),
            "terraform {\n  required_version = \">= 99.0\"\n}\n",
        );
        let v = find_required_version(dir.path());
        assert_eq!(
            v,
            Some("~> 1.5".to_string()),
            "alphabetically-first .tf file with a constraint must win"
        );
    }

    /// A `.tf` file with a mixed-case extension (e.g. `Custom.TF`) carrying a
    /// `required_version` is picked up by the fallback walk — a case-sensitive
    /// comparison would skip these files on case-preserving filesystems.
    #[test]
    fn find_required_version_fallback_matches_uppercase_extension() {
        let dir = tempfile::tempdir().unwrap();
        // Use a name outside the targeted candidate list so we exercise
        // the read_dir fallback specifically.
        write(&dir.path().join("Custom.TF"), "terraform {\n  required_version = \"~> 1.7\"\n}\n");
        let v = find_required_version(dir.path());
        assert_eq!(
            v,
            Some("~> 1.7".to_string()),
            "uppercase .TF extension must be matched case-insensitively"
        );
    }

    /// A non-NotFound IO failure on `versions.tf`
    /// (e.g. the path is a directory) must surface a `tracing::warn!`
    /// instead of silently degrading to "no version declared".
    #[test]
    fn find_required_version_warns_when_versions_tf_is_a_directory() {
        let dir = tempfile::tempdir().unwrap();
        // Create `versions.tf` as a directory, so read_to_string fails
        // with a non-NotFound error (IsADirectory / Other on most OSes).
        std::fs::create_dir(dir.path().join("versions.tf")).unwrap();

        // The shared tracing-capture harness stands in for a per-crate
        // `BufWriter` + `MakeWriter` shim.
        let mut found = None;
        let logs = capture_warn(|| found = find_required_version(dir.path()));
        assert!(found.is_none(), "no required_version should be returned");
        assert!(
            logs.contains("failed to read manifest") && logs.contains("versions.tf"),
            "warn should name versions.tf and the read failure, got: {logs}"
        );
    }

    /// The counterpart for `count_local_modules`: a
    /// `modules` path that is a *file* fails `read_dir` with a non-NotFound
    /// error, which must be reported rather than folded into "no modules".
    #[test]
    fn count_local_modules_warns_when_modules_is_a_file() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("modules"), "not a directory\n");

        let mut count = Some(0);
        let logs = capture_warn(|| count = count_local_modules(dir.path()));
        assert!(count.is_none(), "a failed enumeration reports no modules");
        assert!(
            logs.contains("failed to enumerate modules directory"),
            "warn should name the enumeration failure, got: {logs}"
        );
    }

    /// The control-character drop is the smuggling signal an operator would
    /// actually investigate, and the workspace root
    /// can carry a dozen `.tf` files — the warn must name which one carried
    /// the payload. Drives the full `find_required_version` entry point via
    /// a candidate file so the `source` field is the real probe name.
    #[test]
    fn extract_required_version_control_char_warn_names_the_file() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("versions.tf"),
            "terraform {\n  required_version = \"1.0\u{1b}[2J\u{1b}[31mCOMPROMISED\"\n}\n",
        );

        let logs = capture_warn(|| {
            let found = find_required_version(dir.path());
            assert!(found.is_none(), "the control-char value must be dropped");
        });

        assert!(logs.contains("required_version"), "expected the drop breadcrumb, got: {logs}");
        assert!(
            logs.contains("versions.tf"),
            "the warn must name the offending .tf file, got: {logs}"
        );
    }

    /// The malformed-brace warn names the file too — it
    /// fires once per bad file during the fallback walk with nothing else to
    /// distinguish the repeats.
    #[test]
    fn extract_required_version_unbalanced_brace_warn_names_the_file() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("weird.tf"), "}\n\"unbalanced from the start\"\n");

        let logs = capture_warn(|| {
            let found = find_required_version(dir.path());
            assert!(found.is_none());
        });

        assert!(
            logs.contains("unbalanced closing brace"),
            "expected the brace breadcrumb, got: {logs}"
        );
        assert!(
            logs.contains("weird.tf"),
            "the warn must name the offending .tf file, got: {logs}"
        );
    }

    /// The fallback walk must not re-read a file the
    /// named-candidate loop already opened.
    #[test]
    fn fallback_tf_paths_skips_named_candidates() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["main.tf", "versions.tf", "terraform.tf", "version.tf"] {
            write(&dir.path().join(name), "");
        }
        write(&dir.path().join("extra.tf"), "");
        let paths = fallback_tf_paths(dir.path());
        assert_eq!(paths, vec![dir.path().join("extra.tf")]);
    }

    /// A missing `modules/` dir is the expected "no
    /// local modules" case and returns None silently.
    #[test]
    fn count_local_modules_missing_dir_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(count_local_modules(dir.path()), None);
    }

    /// Counts only subdirectories that hold terraform
    /// sources. Empty dirs and stray files are ignored.
    #[test]
    fn count_local_modules_counts_module_dirs() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("modules").join("a").join("main.tf"), "");
        write(&dir.path().join("modules").join("b").join("main.tf"), "");
        std::fs::create_dir_all(dir.path().join("modules").join("empty")).unwrap();
        write(&dir.path().join("modules").join("stray.txt"), "x");
        assert_eq!(count_local_modules(dir.path()), Some(2));
    }

    /// `main.tf` is a terraform convention, not a requirement — any `.tf`
    /// file makes the directory a module, so these layouts still render a
    /// `modules` line.
    #[test]
    fn count_local_modules_counts_modules_without_main_tf() {
        let dir = tempfile::tempdir().unwrap();
        let modules = dir.path().join("modules");
        write(&modules.join("network").join("network.tf"), "");
        write(&modules.join("vpc").join("variables.tf"), "");
        write(&modules.join("vpc").join("outputs.tf"), "");
        write(&modules.join("json").join("main.tf.json"), "{}");
        // Case-insensitive, matching find_required_version's `.TF` handling.
        write(&modules.join("shouty").join("MAIN.TF"), "");
        // Not a module: no terraform sources at all.
        write(&modules.join("docs").join("README.md"), "");
        std::fs::create_dir_all(modules.join("empty")).unwrap();
        assert_eq!(count_local_modules(dir.path()), Some(4));
    }

    /// Vendoring a shared module once and symlinking it into each stack's
    /// `modules/` is a normal terraform layout. A symlinked module directory
    /// counts as a module — its target is resolved via `fs::metadata` —
    /// while a dangling link resolves to `NotFound` and does not.
    #[cfg(unix)]
    #[test]
    fn count_local_modules_follows_symlinked_module_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let modules = dir.path().join("modules");
        write(&modules.join("real").join("main.tf"), "");

        let vendored = dir.path().join("vendored").join("shared");
        write(&vendored.join("main.tf"), "");
        std::os::unix::fs::symlink(&vendored, modules.join("linked")).unwrap();

        // A symlink to a directory with no terraform sources is followed and
        // then rejected on its contents, not on its link-ness.
        let empty = dir.path().join("vendored").join("no-sources");
        std::fs::create_dir_all(&empty).unwrap();
        std::os::unix::fs::symlink(&empty, modules.join("linked-empty")).unwrap();

        std::os::unix::fs::symlink(
            dir.path().join("vendored").join("missing"),
            modules.join("dangling"),
        )
        .unwrap();

        assert_eq!(count_local_modules(dir.path()), Some(2));
    }
}
