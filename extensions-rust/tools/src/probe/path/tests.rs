//! Tests for `$PATH` walking and the PATH-index cache.

use super::*;

#[test]
#[ignore = "requires rustup installed; run with: cargo test -- --ignored"]
fn check_binary_installed_finds_rustup() {
    assert!(check_binary_installed("rustup"));
}

#[test]
fn check_binary_installed_nonexistent() {
    assert!(!check_binary_installed("nonexistent-binary-abc123xyz"));
}

/// SEC-13 AC #2: cross-platform — a binary placed in a directory on PATH is
/// located. Uses `find_on_path_in` so the test does not have to mutate the
/// process-wide PATH (which would race against parallel tests).
#[cfg(unix)]
#[test]
fn find_on_path_in_locates_executable_unix() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let bin_path = dir.path().join("ops_marker_unix");
    std::fs::write(&bin_path, b"#!/bin/sh\n").unwrap();
    let mut perms = std::fs::metadata(&bin_path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&bin_path, perms).unwrap();

    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    assert_eq!(
        find_on_path_in("ops_marker_unix", &path_var),
        Some(bin_path)
    );
}

/// SEC-13: on Unix a non-executable file in a PATH directory must not be
/// reported — `is_executable` requires the executable bit.
#[cfg(unix)]
#[test]
fn find_on_path_in_skips_non_executable_unix() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin_path = dir.path().join("ops_marker_unix_noexec");
    std::fs::write(&bin_path, b"data\n").unwrap();
    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    assert_eq!(find_on_path_in("ops_marker_unix_noexec", &path_var), None);
}

/// SEC-13: documented Windows fallback is the PATHEXT suffix loop (mirrors
/// `which` / PowerShell). The helper appends each suffix and checks for a
/// regular file.
#[cfg(windows)]
#[test]
fn find_on_path_in_locates_executable_with_pathext_windows() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin_path = dir.path().join("ops_marker_win.exe");
    std::fs::write(&bin_path, b"\0").unwrap();
    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    assert_eq!(find_on_path_in("ops_marker_win", &path_var), Some(bin_path));
}

#[test]
fn find_on_path_returns_none_for_missing_binary() {
    assert!(find_on_path("nonexistent-binary-abc123xyz-zzz").is_none());
}

/// ERR-1 / TASK-0607: a broken symlink on PATH (target removed mid-run, e.g.
/// nix-env update) must not silently coerce the lookup to "missing"; the
/// walk continues but emits a warning. The functional contract here pins
/// that lookup keeps working when a sibling directory holds the real binary.
#[cfg(unix)]
#[test]
fn find_on_path_in_skips_broken_symlink_continues_walk() {
    use std::os::unix::fs::PermissionsExt;

    let broken_dir = tempfile::tempdir().expect("tempdir");
    let real_dir = tempfile::tempdir().expect("tempdir");

    // Broken symlink: PATH/<broken_dir>/ops_marker -> nonexistent target.
    let symlink_path = broken_dir.path().join("ops_marker_broken_sym");
    let nonexistent_target = broken_dir.path().join("does-not-exist");
    std::os::unix::fs::symlink(&nonexistent_target, &symlink_path).unwrap();

    // Real executable in a later PATH entry.
    let real_bin = real_dir.path().join("ops_marker_broken_sym");
    std::fs::write(&real_bin, b"#!/bin/sh\n").unwrap();
    let mut perms = std::fs::metadata(&real_bin).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&real_bin, perms).unwrap();

    let path_var = std::env::join_paths([
        broken_dir.path().to_path_buf(),
        real_dir.path().to_path_buf(),
    ])
    .unwrap();
    assert_eq!(
        find_on_path_in("ops_marker_broken_sym", &path_var),
        Some(real_bin),
        "broken symlink in earlier PATH dir must not block lookup"
    );
}

/// PERF-3 AC #1: a precomputed PATH index resolves a binary placed in a PATH
/// directory without walking PATH per call. Mirrors
/// `find_on_path_in_locates_executable_unix` so the test does not mutate the
/// process-wide PATH.
#[cfg(unix)]
#[test]
fn path_index_finds_executable_basename() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().expect("tempdir");
    let bin_path = dir.path().join("ops_marker_index_hit");
    std::fs::write(&bin_path, b"#!/bin/sh\n").unwrap();
    let mut perms = std::fs::metadata(&bin_path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&bin_path, perms).unwrap();

    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    let index = capture_path_index_from(&path_var);
    assert!(is_in_path_index(&index, "ops_marker_index_hit"));
    assert!(check_binary_installed_with(
        "ops_marker_index_hit",
        Some(&index)
    ));
}

/// PERF-3 AC #1: a non-executable file in a PATH directory must not appear
/// in the index — same contract as `find_on_path_in_skips_non_executable_unix`.
#[cfg(unix)]
#[test]
fn path_index_skips_non_executable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let bin_path = dir.path().join("ops_marker_index_noexec");
    std::fs::write(&bin_path, b"data\n").unwrap();
    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    let index = capture_path_index_from(&path_var);
    assert!(!is_in_path_index(&index, "ops_marker_index_noexec"));
    assert!(!check_binary_installed_with(
        "ops_marker_index_noexec",
        Some(&index)
    ));
}

/// PERF-3 AC #2: when `index` is `None`, [`check_binary_installed_with`]
/// falls back to the per-call PATH walk so one-off callers keep working.
#[test]
fn check_binary_installed_with_none_falls_back() {
    assert!(!check_binary_installed_with(
        "nonexistent-binary-abc123xyz-perf3",
        None
    ));
}

/// PERF-3: a missing binary is not in the index even when the PATH dir is
/// readable — confirms `is_in_path_index` doesn't false-positive on the
/// fallback path.
#[cfg(unix)]
#[test]
fn path_index_missing_binary_not_present() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path_var = std::env::join_paths([dir.path().to_path_buf()]).unwrap();
    let index = capture_path_index_from(&path_var);
    assert!(!is_in_path_index(&index, "definitely-not-there"));
}
