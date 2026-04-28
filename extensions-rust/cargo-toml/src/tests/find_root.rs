use super::*;
use std::fs;

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
