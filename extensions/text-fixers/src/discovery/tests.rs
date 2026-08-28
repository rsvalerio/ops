use std::collections::BTreeSet;
use std::path::Path;

use super::{discover, walk, Fallback};
use crate::test_support::{git_add, git_init, is_inside_repo};

fn names(paths: &[std::path::PathBuf]) -> Vec<String> {
    paths
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

/// Paths relative to `root`, as a set, so the two discovery modes can be
/// compared regardless of order.
fn relative_set(paths: &[std::path::PathBuf], root: &Path) -> BTreeSet<std::path::PathBuf> {
    paths
        .iter()
        .map(|p| p.strip_prefix(root).unwrap_or(p).to_path_buf())
        .collect()
}

#[test]
fn walk_skips_deny_listed_dirs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("keep.txt"), b"a").unwrap();
    std::fs::create_dir_all(root.join("target/sub")).unwrap();
    std::fs::write(root.join("target/sub/skip.txt"), b"a").unwrap();
    std::fs::create_dir_all(root.join("node_modules")).unwrap();
    std::fs::write(root.join("node_modules/skip.txt"), b"a").unwrap();

    let (files, errors) = walk(root).unwrap();
    assert!(errors.is_empty(), "unexpected walk errors: {errors:?}");
    let names = names(&files);
    assert!(names.contains(&"keep.txt".to_string()));
    assert!(!names.iter().any(|n| n == "skip.txt"));
}

#[test]
fn walk_skips_gitignored_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join(".gitignore"), b"*.tsbuildinfo\ngenerated/\n").unwrap();
    std::fs::write(root.join("keep.ts"), b"a").unwrap();
    std::fs::write(root.join("tsconfig.app.tsbuildinfo"), b"{}").unwrap();
    std::fs::create_dir_all(root.join("generated")).unwrap();
    std::fs::write(root.join("generated/out.txt"), b"a").unwrap();

    let names = names(&walk(root).unwrap().0);
    assert!(names.contains(&"keep.ts".to_string()));
    // dotfiles stay in scope; only ignore rules and the deny-list filter.
    assert!(names.contains(&".gitignore".to_string()));
    assert!(!names.iter().any(|n| n == "tsconfig.app.tsbuildinfo"));
    assert!(!names.iter().any(|n| n == "out.txt"));
}

#[test]
fn walk_does_not_filter_the_root_itself() {
    let dir = tempfile::tempdir().unwrap();
    // root is named `build`, which the deny-list drops for subdirectories.
    let root = dir.path().join("build");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(root.join("keep.txt"), b"a").unwrap();

    let (files, _) = walk(&root).unwrap();
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("keep.txt"));
}

#[test]
fn walk_descends_into_normal_subdirs() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::create_dir_all(root.join("src/nested")).unwrap();
    std::fs::write(root.join("src/nested/deep.txt"), b"a").unwrap();
    let (files, errors) = walk(root).unwrap();
    assert!(errors.is_empty(), "unexpected walk errors: {errors:?}");
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("src/nested/deep.txt"));
}

#[cfg(unix)]
#[test]
fn walk_drops_symlinks() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("real.txt"), b"a").unwrap();
    std::os::unix::fs::symlink(root.join("real.txt"), root.join("link.txt")).unwrap();

    let names = names(&walk(root).unwrap().0);
    assert_eq!(names, vec!["real.txt".to_string()]);
}

#[test]
fn a_directory_the_walk_cannot_enter_is_reported_not_swallowed() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    std::fs::write(root.join("keep.txt"), b"a").unwrap();
    let locked = root.join("locked");
    std::fs::create_dir_all(&locked).unwrap();
    std::fs::write(locked.join("hidden.txt"), b"a").unwrap();

    let Some(guard) = crate::test_support::UnsearchableDir::new(&locked) else {
        return; // running as root: the directory is searchable regardless.
    };

    let found = discover(root, false).unwrap();
    drop(guard);

    assert!(
        !found.walk_errors.is_empty(),
        "an unreadable directory contributing zero files must not be silent"
    );
    assert!(
        found.walk_errors.iter().any(|e| e.contains("locked")),
        "the error must name the directory: {:?}",
        found.walk_errors
    );
    assert_eq!(
        relative_set(&found.files, root),
        BTreeSet::from([std::path::PathBuf::from("keep.txt")]),
    );
}

#[test]
fn discover_walk_mode_never_reports_a_fallback() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.txt"), b"a").unwrap();

    let found = discover(dir.path(), false).unwrap();
    assert!(found.fallback.is_none());
    assert_eq!(found.files.len(), 1);
}

#[test]
fn tracked_mode_returns_tracked_files_and_excludes_untracked() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    if !git_init(root) {
        return; // git unusable here; the fallback path is covered separately.
    }
    std::fs::write(root.join("tracked.txt"), b"a").unwrap();
    std::fs::write(root.join("untracked.txt"), b"a").unwrap();
    assert!(git_add(root, &[Path::new("tracked.txt")]));

    let found = discover(root, true).unwrap();
    assert!(found.fallback.is_none(), "a real repo must not fall back");
    assert_eq!(found.undecodable_paths, 0);
    assert_eq!(
        relative_set(&found.files, root),
        BTreeSet::from([std::path::PathBuf::from("tracked.txt")]),
    );
}

#[test]
fn tracked_mode_joins_paths_relative_to_a_subdirectory_root() {
    // `git ls-files -z` prints paths relative to the directory it runs in, not
    // to the repository root. Joining them onto a subdirectory root is only
    // correct because of that; pin it.
    let dir = tempfile::tempdir().unwrap();
    let repo = dir.path();
    if !git_init(repo) {
        return;
    }
    let sub = repo.join("sub");
    std::fs::create_dir_all(&sub).unwrap();
    std::fs::write(repo.join("top.txt"), b"a").unwrap();
    std::fs::write(sub.join("inner.txt"), b"a").unwrap();
    assert!(git_add(
        repo,
        &[Path::new("top.txt"), Path::new("sub/inner.txt")]
    ));

    let found = discover(&sub, true).unwrap();
    assert_eq!(
        relative_set(&found.files, &sub),
        BTreeSet::from([std::path::PathBuf::from("inner.txt")]),
        "only files under the subdirectory, and joined onto it"
    );
    assert!(
        found.files.iter().all(|p| p.exists()),
        "a mis-joined path would not exist: {:?}",
        found.files
    );
}

#[test]
fn tracked_mode_falls_back_and_reports_when_root_is_not_a_repository() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    if is_inside_repo(root) {
        return; // TMPDIR lives inside a repository; the fixture is void.
    }
    std::fs::write(root.join("untracked.txt"), b"a").unwrap();

    let found = discover(root, true).unwrap();
    assert_eq!(found.fallback, Some(Fallback::NotARepository));
    assert_eq!(
        relative_set(&found.files, root),
        BTreeSet::from([std::path::PathBuf::from("untracked.txt")]),
        "the fallback is the full walk, untracked files included"
    );
}

#[test]
fn a_genuine_git_failure_is_an_error_not_a_silent_fallback() {
    // The case that motivated the change: git exits non-zero on a directory
    // that *is* a repository (corrupt index here; `dubious ownership` in the
    // field). Falling back would quietly rewrite untracked files.
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    if !git_init(root) {
        return;
    }
    std::fs::write(root.join("a.txt"), b"a").unwrap();
    assert!(git_add(root, &[Path::new("a.txt")]));
    std::fs::write(root.join(".git/index"), b"not an index").unwrap();

    let err = discover(root, true).expect_err("a corrupt index must not fall back");
    assert!(
        err.to_string().contains("git ls-files"),
        "the error must say what failed: {err}"
    );
}

#[cfg(unix)]
#[test]
fn tracked_mode_drops_symlinks_and_agrees_with_the_walk() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    if !git_init(root) {
        return;
    }
    std::fs::write(root.join("real.txt"), b"a").unwrap();
    std::os::unix::fs::symlink("real.txt", root.join("link.txt")).unwrap();
    assert!(git_add(
        root,
        &[Path::new("real.txt"), Path::new("link.txt")]
    ));

    let tracked = discover(root, true).unwrap();
    let walked = discover(root, false).unwrap();
    assert_eq!(
        relative_set(&tracked.files, root),
        BTreeSet::from([std::path::PathBuf::from("real.txt")]),
        "a tracked symlink must never be a candidate"
    );
    assert_eq!(
        relative_set(&tracked.files, root),
        relative_set(&walked.files, root),
        "the two modes must agree about symlinks"
    );
}

#[cfg(unix)]
#[test]
fn tracked_mode_keeps_a_non_utf8_filename_and_agrees_with_the_walk() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    if !git_init(root) {
        return;
    }
    // Latin-1 "café.txt": no valid UTF-8 decoding, a perfectly good path.
    let raw = OsStr::from_bytes(&[b'c', b'a', b'f', 0xE9, b'.', b't', b'x', b't']);
    std::fs::write(root.join(raw), b"a").unwrap();
    if !git_add(root, &[Path::new(raw)]) {
        return; // git configured to reject the name; nothing to assert.
    }

    let tracked = discover(root, true).unwrap();
    let walked = discover(root, false).unwrap();
    assert_eq!(
        tracked.undecodable_paths, 0,
        "unix decodes bytes losslessly"
    );
    assert_eq!(
        relative_set(&tracked.files, root),
        BTreeSet::from([std::path::PathBuf::from(raw)]),
        "a non-UTF-8 filename must not be silently dropped"
    );
    assert_eq!(
        relative_set(&tracked.files, root),
        relative_set(&walked.files, root),
        "the two modes must agree about non-UTF-8 names"
    );
}
