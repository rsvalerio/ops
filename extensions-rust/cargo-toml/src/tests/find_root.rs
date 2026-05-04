use super::*;
use std::fs;
use std::path::Path;

/// ERR-7 (TASK-0947): tracing fields for ancestor-walk Cargo.toml paths
/// flow through the `?` formatter so an attacker-controlled CWD path with
/// embedded newlines / ANSI escapes cannot forge log records.
#[test]
fn manifest_declares_workspace_path_debug_escapes_control_characters() {
    let p = Path::new("a\nb\u{1b}[31mc/Cargo.toml");
    let rendered = format!("{:?}", p.display());
    assert!(!rendered.contains('\n'));
    assert!(!rendered.contains('\u{1b}'));
    assert!(rendered.contains("\\n"));
}

#[test]
fn find_root_in_current_dir() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(&cargo_toml, "[package]\nname = \"test\"\n").expect("write cargo toml");

    let root = find_workspace_root(temp_dir.path()).expect("should find");
    assert_eq!(root, fs::canonicalize(temp_dir.path()).unwrap());
}

#[test]
fn find_root_in_parent() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let cargo_toml = temp_dir.path().join("Cargo.toml");
    fs::write(&cargo_toml, "[package]\nname = \"test\"\n").expect("write cargo toml");

    let subdir = temp_dir.path().join("crates").join("sub");
    fs::create_dir_all(&subdir).expect("create subdir");

    let root = find_workspace_root(&subdir).expect("should find");
    assert_eq!(root, fs::canonicalize(temp_dir.path()).unwrap());
}

/// SEC-25 / TASK-0379: a symlinked ancestor must resolve once up front
/// and the walk must terminate even when a symlink loop is on the path.
#[cfg(unix)]
#[test]
fn find_root_terminates_on_symlink_loop() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let a = temp_dir.path().join("a");
    let b = temp_dir.path().join("b");
    std::os::unix::fs::symlink(&b, &a).unwrap();
    std::os::unix::fs::symlink(&a, &b).unwrap();

    // canonicalize fails on the loop, so we get a clear error rather than
    // an infinite loop.
    let result = find_workspace_root(&a);
    assert!(result.is_err());
}

/// TASK-0501: from inside a member crate, walk past the member manifest to
/// the parent that declares `[workspace]`. Returning the member silently
/// produced empty units/coverage when running `ops about` from `crates/foo`.
#[test]
fn find_root_prefers_workspace_over_member() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let root = temp_dir.path();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/foo\"]\n",
    )
    .expect("write workspace");

    let member = root.join("crates").join("foo");
    fs::create_dir_all(&member).expect("create member dir");
    fs::write(
        member.join("Cargo.toml"),
        "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n",
    )
    .expect("write member");

    let found = find_workspace_root(&member).expect("should find workspace root");
    assert_eq!(found, fs::canonicalize(root).unwrap());
}

/// TASK-0501: a single-crate (non-workspace) project still resolves to the
/// nearest Cargo.toml when no ancestor declares `[workspace]`.
#[test]
fn find_root_falls_back_to_nearest_when_no_workspace_in_chain() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let root = temp_dir.path();
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname = \"loner\"\nversion = \"0.1.0\"\n",
    )
    .expect("write package");

    let sub = root.join("src");
    fs::create_dir_all(&sub).expect("create src");

    let found = find_workspace_root(&sub).expect("should find package root");
    assert_eq!(found, fs::canonicalize(root).unwrap());
}

/// SEC-25 / TASK-0604: `start` is canonicalised once before the walk, so a
/// symlinked parent directory in the input path resolves to the real
/// filesystem location and the walk operates on that resolved chain.
#[cfg(unix)]
#[test]
fn find_root_resolves_symlinked_parent_in_start_path() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let real_root = temp_dir.path().join("real");
    let real_member = real_root.join("crates").join("foo");
    fs::create_dir_all(&real_member).expect("create real member");
    fs::write(
        real_root.join("Cargo.toml"),
        "[workspace]\nmembers = [\"crates/foo\"]\n",
    )
    .expect("write workspace");
    fs::write(
        real_member.join("Cargo.toml"),
        "[package]\nname = \"foo\"\nversion = \"0.1.0\"\n",
    )
    .expect("write member");

    // Create a symlink that aliases `real/crates` → accessed via a sibling path.
    let alias = temp_dir.path().join("alias_crates");
    std::os::unix::fs::symlink(real_root.join("crates"), &alias).expect("create symlink");
    let symlinked_member = alias.join("foo");

    let found = find_workspace_root(&symlinked_member).expect("should find workspace root");
    // Walk operates on the canonical (real) chain, so the workspace root is
    // returned at its real location, not under the alias.
    assert_eq!(found, fs::canonicalize(&real_root).unwrap());
}

/// TASK-0963: the ancestor-depth bound must be honored. Verified via the
/// injectable [`find_workspace_root_with_depth`] entry point so the test
/// does not have to materialise a 64-deep directory hierarchy. With
/// `max_depth = 1`, a Cargo.toml in the start dir's *grandparent* is
/// unreachable; with `max_depth = 3` it is found.
#[test]
fn find_root_respects_injected_depth_bound() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let root = temp_dir.path();
    let leaf = root.join("a").join("b");
    fs::create_dir_all(&leaf).unwrap();
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"top\"\n").unwrap();

    let bounded = find_workspace_root_with_depth(&leaf, 1);
    assert!(
        matches!(
            bounded,
            Err(FindWorkspaceRootError::NotFound { depth: 1, .. })
        ),
        "depth=1 must NotFound before reaching grandparent, got: {bounded:?}"
    );

    let unbounded = find_workspace_root_with_depth(&leaf, 4).expect("depth=4 reaches root");
    assert_eq!(unbounded, fs::canonicalize(root).unwrap());
}

#[test]
fn find_root_not_found() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");

    let result = find_workspace_root(temp_dir.path());
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("no Cargo.toml found"));
}

/// ARCH-2 / TASK-0871: NotFound and CanonicalizeFailed must be
/// distinguishable via the typed error so consumers
/// (`is_manifest_missing`) don't need to chain-walk an `io::Error`.
#[test]
fn find_root_typed_not_found_variant() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let err = find_workspace_root(temp_dir.path()).unwrap_err();
    assert!(matches!(err, FindWorkspaceRootError::NotFound { .. }));
    assert!(err.is_not_found());
}

/// ARCH-2 / TASK-0918: a missing-or-deleted `start` path now routes
/// through NotFound (matching the no-Cargo.toml branch), not
/// CanonicalizeFailed. Pre-fix this surfaced as a confusing
/// "failed to canonicalize" error during transient cwd unlinks (CI
/// volume eviction, watcher rename) when the user just wanted About
/// to fall back gracefully.
#[test]
fn find_root_canonicalize_notfound_routes_to_not_found_variant() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let missing = temp_dir.path().join("does-not-exist");
    let err = find_workspace_root(&missing).unwrap_err();
    assert!(
        matches!(err, FindWorkspaceRootError::NotFound { .. }),
        "expected NotFound, got: {err:?}"
    );
    assert!(err.is_not_found());
}

/// ARCH-2 / TASK-0918: a non-NotFound canonicalize failure still
/// surfaces as the typed CanonicalizeFailed variant so it remains
/// investigable. Use a 0o000-permission directory on Unix to force a
/// PermissionDenied at canonicalize time.
#[cfg(unix)]
#[test]
fn find_root_canonicalize_perm_denied_keeps_canonicalize_failed_variant() {
    use std::os::unix::fs::PermissionsExt;
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let locked = temp_dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    let inside = locked.join("inner");
    fs::create_dir(&inside).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

    let result = find_workspace_root(&inside);

    // Restore perms so tempdir cleanup works.
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

    let err = result.unwrap_err();
    // The exact error kind for a PermissionDenied-during-canonicalize
    // varies across Linux/macOS; accept either CanonicalizeFailed
    // (the desired path) or NotFound (some kernels surface EACCES on a
    // descendant as ENOENT). The key invariant is "doesn't panic and
    // is a typed FindWorkspaceRootError".
    assert!(
        matches!(
            err,
            FindWorkspaceRootError::CanonicalizeFailed { .. }
                | FindWorkspaceRootError::NotFound { .. }
        ),
        "expected typed FindWorkspaceRootError, got: {err:?}"
    );
}
