//! End-to-end tests for the two fixers.
//!
//! The happy path is the cheap half. The half that matters for a tool which
//! rewrites the user's files in place on a pre-commit path is everything
//! below it: unreadable and unwritable files, symlinks, oversized files,
//! binary payloads past the old sniff window, mode preservation, and the
//! fixed-point property the exit-code contract depends on.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::*;
use crate::test_support::{git_add, git_init, ReadOnlyDir, UnreadableFile};

fn write(p: &Path, content: &[u8]) {
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, content).unwrap();
}

fn opts(root: &Path) -> FixerOptions {
    FixerOptions::new(root.to_path_buf(), false)
}

/// Every regular file under `root`, keyed by relative path.
fn snapshot(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut out = BTreeMap::new();
    for path in discovery::discover(root, false).unwrap().files {
        let rel = path.strip_prefix(root).unwrap_or(&path).to_path_buf();
        out.insert(rel, std::fs::read(&path).unwrap());
    }
    out
}

#[test]
fn trailing_whitespace_rewrites_dirty_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("a.txt"), b"hello   \nworld\n");
    write(&root.join("clean.txt"), b"clean\n");
    write(&root.join("bin.dat"), b"hello\0world   ");

    let mut buf = Vec::new();
    let report = run_trailing_whitespace(&opts(root), &mut buf).unwrap();

    assert_eq!(report.files_changed, vec![PathBuf::from("a.txt")]);
    assert_eq!(report.files_skipped, 1, "the binary file is a skip");
    assert!(!report.failed());
    assert_eq!(
        std::fs::read(root.join("a.txt")).unwrap(),
        b"hello\nworld\n"
    );
    assert_eq!(std::fs::read(root.join("clean.txt")).unwrap(), b"clean\n");
    assert_eq!(
        std::fs::read(root.join("bin.dat")).unwrap(),
        b"hello\0world   ",
        "binary file must be left alone"
    );
}

#[test]
fn eof_fixer_adds_missing_newline() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("a.txt"), b"hello");
    write(&root.join("b.txt"), b"hello\n");
    write(&root.join("c.txt"), b"hello\n\n\n");

    let mut buf = Vec::new();
    let report = run_end_of_file_fixer(&opts(root), &mut buf).unwrap();

    assert_eq!(report.files_changed.len(), 2);
    assert_eq!(std::fs::read(root.join("a.txt")).unwrap(), b"hello\n");
    assert_eq!(std::fs::read(root.join("b.txt")).unwrap(), b"hello\n");
    assert_eq!(std::fs::read(root.join("c.txt")).unwrap(), b"hello\n");
}

#[test]
fn every_discovered_file_is_accounted_for() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("text.txt"), b"a   \n");
    write(&root.join("bin.dat"), b"a\0b");
    write(&root.join("clean.txt"), b"a\n");

    let mut buf = Vec::new();
    let report = run_trailing_whitespace(&opts(root), &mut buf).unwrap();

    let discovered = discovery::discover(root, false).unwrap().files.len();
    assert_eq!(
        report.files_scanned + report.files_skipped + report.files_failed.len(),
        discovered,
        "scanned + skipped + failed must account for every discovered path"
    );
}

/// A file whose rewrite fails belongs in exactly one bucket. It used to be
/// tallied as *scanned* on the way in and as *failed* on the way out, so the
/// summary line double-counted it and `scanned + skipped + failed` overshot
/// the number of paths discovery actually handed the runner.
#[test]
fn a_file_whose_write_fails_is_counted_once() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("dirty.txt"), b"a   \n");

    // Read-only root: the file is still readable and needs fixing, but the
    // atomic replace cannot stage its sibling.
    let Some(_guard) = ReadOnlyDir::new(root) else {
        return; // Running as root, or the chmod did not deny anything.
    };

    let mut buf = Vec::new();
    let report = run_trailing_whitespace(&opts(root), &mut buf).unwrap();

    assert_eq!(report.files_failed.len(), 1, "the write must fail");
    assert_eq!(
        report.files_scanned, 0,
        "a failed file is not also a scanned file"
    );
    let discovered = discovery::discover(root, false).unwrap().files.len();
    assert_eq!(
        report.files_scanned + report.files_skipped + report.files_failed.len(),
        discovered,
        "scanned + skipped + failed must still account for every discovered path"
    );
}

#[test]
fn both_fixers_over_a_mixed_tree_reach_a_fixed_point() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("lf.txt"), b"a   \nb\t\n\n\n");
    write(&root.join("crlf.txt"), b"a  \r\nb\r\n\r\n\r\n");
    write(&root.join("no-newline.txt"), b"tail  ");
    write(&root.join(".dotfile"), b"dot \n");
    write(&root.join("nested/deep/inner.txt"), b"deep\t\nmore\n");
    write(&root.join("empty.txt"), b"");

    let o = opts(root);
    let mut buf = Vec::new();
    run_trailing_whitespace(&o, &mut buf).unwrap();
    run_end_of_file_fixer(&o, &mut buf).unwrap();
    let after_first = snapshot(root);

    let mut buf = Vec::new();
    let r1 = run_trailing_whitespace(&o, &mut buf).unwrap();
    let r2 = run_end_of_file_fixer(&o, &mut buf).unwrap();

    assert!(!r1.changed(), "trailing-whitespace is not idempotent");
    assert!(!r2.changed(), "end-of-file-fixer is not idempotent");
    assert!(!r1.failed() && !r2.failed());
    assert_eq!(
        snapshot(root),
        after_first,
        "the tree must be byte-identical after a second pass"
    );
}

#[test]
fn a_zero_byte_file_stays_zero_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(&root.join("empty.txt"), b"");

    let o = opts(root);
    let mut buf = Vec::new();
    let r1 = run_trailing_whitespace(&o, &mut buf).unwrap();
    let r2 = run_end_of_file_fixer(&o, &mut buf).unwrap();

    assert!(!r1.changed() && !r2.changed());
    assert_eq!(std::fs::read(root.join("empty.txt")).unwrap(), b"");
}

#[test]
fn a_binary_payload_past_the_old_sniff_window_is_left_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    // >8 KiB of ASCII, then a payload holding both the `0x20 0x0A` pair
    // `fix_trailing` would eat and the NUL the old prefix sniff never saw.
    let mut content = vec![b'a'; 9000];
    content.extend_from_slice(b"P5 header\n");
    content.extend_from_slice(&[0x20, 0x0A, 0xFF, 0x00, 0x20, 0x0A]);
    let path = root.join("image.pgm");
    write(&path, &content);

    let o = opts(root);
    let mut buf = Vec::new();
    let r1 = run_trailing_whitespace(&o, &mut buf).unwrap();
    let r2 = run_end_of_file_fixer(&o, &mut buf).unwrap();

    assert!(!r1.changed() && !r2.changed());
    assert_eq!(
        std::fs::read(&path).unwrap(),
        content,
        "a binary payload past 8 KiB must not be edited"
    );
    assert_eq!(r1.files_skipped, 1);
}

#[test]
fn a_file_over_the_cap_is_skipped_reported_and_counted() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let path = root.join("big.txt");
    write(&path, b"trailing space   \n");

    let o = opts(root).with_max_bytes(4);
    let mut buf = Vec::new();
    let report = run_trailing_whitespace(&o, &mut buf).unwrap();

    assert!(!report.changed());
    assert_eq!(report.files_skipped, 1);
    assert_eq!(report.files_scanned, 0);
    let out = String::from_utf8(buf).unwrap();
    assert!(
        out.contains("big.txt"),
        "the skip must name the file: {out}"
    );
    assert!(out.contains("exceeds cap"), "{out}");
    assert_eq!(std::fs::read(&path).unwrap(), b"trailing space   \n");
}

#[cfg(unix)]
#[test]
fn file_mode_survives_the_rewrite() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let path = root.join("a.txt");
    write(&path, b"hello   \n");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640)).unwrap();

    let mut buf = Vec::new();
    let report = run_trailing_whitespace(&opts(root), &mut buf).unwrap();

    assert!(report.changed());
    assert_eq!(std::fs::read(&path).unwrap(), b"hello\n");
    let mode = std::fs::metadata(&path).unwrap().permissions().mode();
    assert_eq!(
        mode & 0o777,
        0o640,
        "the temp-file-and-rename rewrite must preserve the mode"
    );
}

#[cfg(unix)]
#[test]
fn a_read_only_target_is_still_rewritten_with_its_mode_intact() {
    // Pinning a consequence of the move to rename-based writes: `fs::write`
    // used to fail with EACCES on a 0444 file, while `rename(2)` only needs a
    // writable *directory*. The mode is carried onto the new inode, so the
    // file stays read-only afterwards.
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let path = root.join("ro.txt");
    write(&path, b"hello   \n");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o444)).unwrap();

    let mut buf = Vec::new();
    let report = run_trailing_whitespace(&opts(root), &mut buf).unwrap();

    assert!(!report.failed());
    assert_eq!(report.files_changed, vec![PathBuf::from("ro.txt")]);
    assert_eq!(std::fs::read(&path).unwrap(), b"hello\n");
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o444
    );
}

#[test]
fn a_failing_write_names_the_path_keeps_going_and_leaves_the_file_intact() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let locked_dir = root.join("locked");
    std::fs::create_dir_all(&locked_dir).unwrap();
    write(&locked_dir.join("blocked.txt"), b"blocked   \n");
    write(&root.join("later.txt"), b"later   \n");

    let Some(guard) = ReadOnlyDir::new(&locked_dir) else {
        return; // running as root: the directory is writable regardless.
    };

    let mut buf = Vec::new();
    let report = run_trailing_whitespace(&opts(root), &mut buf).unwrap();
    drop(guard);

    assert!(report.failed(), "an unwritable file must fail the run");
    let failure = &report.files_failed[0];
    assert!(
        failure.path.ends_with("blocked.txt"),
        "the failure must name the path: {failure:?}"
    );
    assert!(matches!(failure.kind, FailureKind::Write(_)));
    let out = String::from_utf8(buf).unwrap();
    assert!(
        out.contains("blocked.txt"),
        "the rendered line must name the path: {out}"
    );

    assert_eq!(
        std::fs::read(locked_dir.join("blocked.txt")).unwrap(),
        b"blocked   \n",
        "a failed write must leave the original byte-identical"
    );
    assert_eq!(
        report.files_changed,
        vec![PathBuf::from("later.txt")],
        "the run must continue past the failure and still report what it fixed"
    );
    assert_eq!(std::fs::read(root.join("later.txt")).unwrap(), b"later\n");
}

#[test]
fn an_unreadable_file_is_reported_rather_than_making_the_run_look_clean() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let path = root.join("secret.txt");
    write(&path, b"secret   \n");

    let Some(guard) = UnreadableFile::new(&path) else {
        return; // running as root: the file is readable regardless.
    };

    let mut buf = Vec::new();
    let report = run_trailing_whitespace(&opts(root), &mut buf).unwrap();
    drop(guard);

    assert!(
        report.failed(),
        "exit zero must not mean 'clean' for a file that was never read"
    );
    assert!(!report.changed());
    assert_eq!(report.files_scanned, 0);
    let failure = &report.files_failed[0];
    assert_eq!(failure.path, PathBuf::from("secret.txt"));
    assert!(matches!(failure.kind, FailureKind::Read(_)));
    let out = String::from_utf8(buf).unwrap();
    assert!(out.contains("secret.txt"), "{out}");
    assert!(out.contains("read:"), "{out}");
}

#[cfg(unix)]
#[test]
fn tracked_mode_never_rewrites_through_a_symlink_out_of_the_root() {
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("app.conf");
    let original: &[u8] = b"key = value   \nno trailing newline here  ";
    std::fs::write(&target, original).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    if !git_init(root) {
        return;
    }
    std::os::unix::fs::symlink(&target, root.join("escape.conf")).unwrap();
    write(&root.join("inside.txt"), b"inside   \n");
    assert!(git_add(
        root,
        &[Path::new("escape.conf"), Path::new("inside.txt")]
    ));

    let o = FixerOptions::new(root.to_path_buf(), true);
    let mut buf = Vec::new();
    let r1 = run_trailing_whitespace(&o, &mut buf).unwrap();
    let r2 = run_end_of_file_fixer(&o, &mut buf).unwrap();

    assert_eq!(
        std::fs::read(&target).unwrap(),
        original,
        "a repository symlink must never reach a file outside the root"
    );
    assert!(r1.changed(), "the real file inside the root is still fixed");
    assert!(!r2.failed());
    assert_eq!(std::fs::read(root.join("inside.txt")).unwrap(), b"inside\n");
}

#[test]
fn tracked_mode_end_to_end_fixes_only_tracked_files() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    if !git_init(root) {
        return;
    }
    write(&root.join("tracked.txt"), b"tracked   \n");
    write(&root.join("untracked.txt"), b"untracked   \n");
    assert!(git_add(root, &[Path::new("tracked.txt")]));

    let o = FixerOptions::new(root.to_path_buf(), true);
    let mut buf = Vec::new();
    let report = run_trailing_whitespace(&o, &mut buf).unwrap();

    assert_eq!(report.files_changed, vec![PathBuf::from("tracked.txt")]);
    assert_eq!(
        std::fs::read(root.join("untracked.txt")).unwrap(),
        b"untracked   \n",
        "--tracked must not touch untracked files"
    );
}

#[test]
fn a_tracked_run_outside_a_repository_announces_the_downgrade() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    if crate::test_support::is_inside_repo(root) {
        return; // TMPDIR lives inside a repository; the fixture is void.
    }
    write(&root.join("untracked.txt"), b"untracked   \n");

    let o = FixerOptions::new(root.to_path_buf(), true);
    let mut buf = Vec::new();
    let report = run_trailing_whitespace(&o, &mut buf).unwrap();

    let out = String::from_utf8(buf).unwrap();
    assert!(
        out.contains("--tracked unavailable") && out.contains("not a git repository"),
        "the widened scope must be announced: {out}"
    );
    assert_eq!(report.files_changed, vec![PathBuf::from("untracked.txt")]);
}

#[test]
fn extension_constants_kebab_case() {
    for n in ["trailing-whitespace", "end-of-file-fixer", NAME, SHORTNAME] {
        assert!(
            n.chars().all(|c| c.is_ascii_lowercase() || c == '-'),
            "must be kebab-case: {n}"
        );
    }
}
