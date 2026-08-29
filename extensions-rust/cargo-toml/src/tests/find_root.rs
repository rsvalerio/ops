use super::*;
use crate::workspace_root::content_declares_workspace;
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

/// ARCH-2 / TASK-0871: `NotFound` and `CanonicalizeFailed` must be
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
/// through `NotFound` (matching the no-Cargo.toml branch), not
/// `CanonicalizeFailed`. Pre-fix this surfaced as a confusing
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

/// SEC-25 / TASK-1204: pin that the lenient walk follows a symlinked
/// ancestor into an attacker tree. A symlink replaces `inner_link` →
/// `attacker/`, and the lenient walker — which canonicalises once up
/// front — walks inside the attacker subtree and returns the attacker's
/// `[workspace]` manifest. This is the documented trade-off: the
/// lenient variant preserves today's behaviour. The strict variant is
/// contrasted against the same layout in
/// `find_root_strict_also_follows_symlink_inside_the_start_path`, and its
/// off-chain rejection arm is covered by
/// `strict_candidate_action_skips_off_chain_canonical_parent`.
#[cfg(unix)]
#[test]
fn find_root_lenient_follows_symlinked_ancestor_into_attacker_tree() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let real_root = fs::canonicalize(tmp.path()).expect("canonicalize tempdir");

    let attacker = real_root.join("attacker");
    fs::create_dir(&attacker).unwrap();
    fs::write(attacker.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    fs::create_dir(attacker.join("leaf")).unwrap();

    let symlink_at = real_root.join("inner_link");
    std::os::unix::fs::symlink(&attacker, &symlink_at).unwrap();
    let leaf_via_symlink = symlink_at.join("leaf");

    let lenient =
        find_workspace_root(&leaf_via_symlink).expect("lenient should find a workspace root");
    assert_eq!(
        lenient,
        fs::canonicalize(&attacker).unwrap(),
        "lenient walk must follow symlink into attacker tree"
    );
}

/// SEC-25 / TASK-1204: the strict variant accepts a candidate whose
/// canonical parent is on the canonical start's ancestor chain.
///
/// TEST-6 / TASK-1785: renamed from
/// `find_root_strict_skips_off_chain_canonical_ancestor`, which promised a
/// rejection this body never exercised (it plants no symlink and asserts
/// nothing the lenient walk does not already satisfy). The rejection arms
/// are covered by `strict_candidate_action_*` below.
#[cfg(unix)]
#[test]
fn find_root_strict_accepts_on_chain_canonical_ancestor() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let real_root = fs::canonicalize(tmp.path()).expect("canonicalize");

    // Legit chain: real_root/legit/leaf/Cargo.toml (package).
    let legit = real_root.join("legit");
    let leaf = legit.join("leaf");
    fs::create_dir_all(&leaf).unwrap();
    fs::write(
        leaf.join("Cargo.toml"),
        "[package]\nname = \"leaf\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();

    // The strict variant must accept the legit leaf manifest because
    // its canonical parent is on the canonical-start chain.
    let strict = find_workspace_root_strict(&leaf).expect("strict must find legit leaf manifest");
    assert_eq!(strict, leaf);

    // Sanity: the lenient variant agrees on this layout.
    let lenient = find_workspace_root(&leaf).expect("lenient must find legit leaf manifest");
    assert_eq!(lenient, leaf);
}

/// TEST-6 / TASK-1785: pin the actual scope of the strict variant against
/// the symlinked-ancestor layout its doc comment describes.
///
/// A symlink inside the *caller's* `start` path is resolved by the shared
/// walk before any candidate is inspected, so the strict variant walks the
/// attacker's real chain and — like the lenient variant — returns the
/// attacker's manifest. This is the paired contrast to
/// `find_root_lenient_follows_symlinked_ancestor_into_attacker_tree`: both
/// variants behave identically here, and the strict variant's extra
/// canonicalize buys nothing for this shape. Pinning it keeps a future
/// reader from mistaking the check for a defence it does not provide.
#[cfg(unix)]
#[test]
fn find_root_strict_also_follows_symlink_inside_the_start_path() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let real_root = fs::canonicalize(tmp.path()).expect("canonicalize tempdir");

    let attacker = real_root.join("attacker");
    fs::create_dir(&attacker).unwrap();
    fs::write(attacker.join("Cargo.toml"), "[workspace]\nmembers = []\n").unwrap();
    fs::create_dir(attacker.join("leaf")).unwrap();

    let symlink_at = real_root.join("inner_link");
    std::os::unix::fs::symlink(&attacker, &symlink_at).unwrap();
    let leaf_via_symlink = symlink_at.join("leaf");

    let canonical_attacker = fs::canonicalize(&attacker).unwrap();
    let strict = find_workspace_root_strict(&leaf_via_symlink).expect("strict finds a root");
    let lenient = find_workspace_root(&leaf_via_symlink).expect("lenient finds a root");
    assert_eq!(strict, canonical_attacker);
    assert_eq!(
        strict, lenient,
        "strict and lenient must agree when the symlink is inside the caller's start path"
    );
}

/// SEC-25 / TASK-2026: the strict variant's *reachable* defence — a
/// `Cargo.toml` that is itself a symlink into an attacker tree.
///
/// This is the layout the strict variant's ancestor-chain check cannot see:
/// every directory on the walk is genuine and canonical, so
/// `canonicalize(dir) == dir` holds throughout, but the manifest read
/// through `legit/Cargo.toml` is the attacker's `[workspace]` file. The
/// lenient walk accepts it and returns `legit` as the workspace root; the
/// strict walk resolves the manifest itself, sees its canonical parent is
/// `attacker`, skips the candidate, and — with no other manifest in the
/// chain — reports `NotFound`.
#[cfg(unix)]
#[test]
fn find_root_strict_rejects_symlinked_manifest_that_lenient_accepts() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let real_root = fs::canonicalize(tmp.path()).expect("canonicalize tempdir");

    let attacker = real_root.join("attacker");
    fs::create_dir(&attacker).unwrap();
    let planted = attacker.join("Cargo.toml");
    fs::write(&planted, "[workspace]\nmembers = []\n").unwrap();

    // A genuine (non-symlinked) directory chain whose manifest is a symlink
    // pointing into the attacker tree.
    let legit = real_root.join("legit");
    let leaf = legit.join("leaf");
    fs::create_dir_all(&leaf).unwrap();
    std::os::unix::fs::symlink(&planted, legit.join("Cargo.toml")).unwrap();

    let lenient = find_workspace_root(&leaf).expect("lenient follows the planted manifest symlink");
    assert_eq!(
        lenient, legit,
        "lenient walk must read the planted manifest through the symlink"
    );

    let err = find_workspace_root_strict(&leaf)
        .expect_err("strict must refuse a manifest that resolves outside its own directory");
    assert!(
        matches!(err, FindWorkspaceRootError::NotFound { .. }),
        "expected NotFound after the planted manifest is skipped, got: {err:?}"
    );
}

/// TEST-6 / TASK-1785: the off-chain rejection arm
/// (`workspace_root.rs`, `CandidateAction::Skip`) is unreachable through a
/// quiescent filesystem — every lexical ancestor of an already-canonical
/// path is canonical — so it is driven here through the injected
/// canonicalizer instead of by racing a symlink swap. Before this test,
/// deleting the whole arm left the suite green.
#[test]
fn strict_candidate_action_skips_off_chain_canonical_parent() {
    let start_canonical = Path::new("/real/legit/leaf");
    let lexical_parent = Path::new("/real/legit");
    let cargo_toml = lexical_parent.join("Cargo.toml");

    // The ancestor was swapped for a symlink into an attacker tree after
    // `start` was canonicalized, so its canonical form is off-chain.
    let action = crate::workspace_root::strict_candidate_action(
        lexical_parent,
        &cargo_toml,
        start_canonical,
        &|_| Ok(PathBuf::from("/attacker/tree")),
    );

    assert_eq!(
        action,
        crate::workspace_root::CandidateAction::Skip,
        "an off-chain canonical parent must be rejected, not recorded as a fallback root"
    );
}

/// TEST-6 / TASK-1785: the second `CandidateAction::Skip` arm — a candidate
/// ancestor whose `canonicalize` fails — is likewise unreachable through a
/// quiescent filesystem, since `walk_ancestors` already canonicalized the
/// start successfully.
#[test]
fn strict_candidate_action_skips_ancestor_that_fails_to_canonicalize() {
    let start_canonical = Path::new("/real/legit/leaf");
    let lexical_parent = Path::new("/real/legit");
    let cargo_toml = lexical_parent.join("Cargo.toml");

    let action = crate::workspace_root::strict_candidate_action(
        lexical_parent,
        &cargo_toml,
        start_canonical,
        &|_| {
            Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "denied",
            ))
        },
    );

    assert_eq!(
        action,
        crate::workspace_root::CandidateAction::Skip,
        "a candidate whose parent cannot be canonicalized must be skipped"
    );
}

/// TEST-6 / TASK-1785: the accept arms of the same function, so the two
/// `Skip` tests above cannot pass by the function returning `Skip`
/// unconditionally.
#[test]
fn strict_candidate_action_accepts_on_chain_parent() {
    use crate::workspace_root::{strict_candidate_action, CandidateAction};

    let tmp = tempfile::tempdir().expect("tempdir");
    let root = fs::canonicalize(tmp.path()).expect("canonicalize");
    let leaf = root.join("leaf");
    fs::create_dir(&leaf).unwrap();

    let ws_manifest = root.join("Cargo.toml");
    fs::write(&ws_manifest, "[workspace]\nmembers = []\n").unwrap();
    assert_eq!(
        strict_candidate_action(&root, &ws_manifest, &leaf, &|p| fs::canonicalize(p)),
        CandidateAction::AcceptWorkspace(root.clone())
    );

    let pkg_manifest = leaf.join("Cargo.toml");
    fs::write(&pkg_manifest, "[package]\nname = \"leaf\"\n").unwrap();
    assert_eq!(
        strict_candidate_action(&leaf, &pkg_manifest, &leaf, &|p| fs::canonicalize(p)),
        CandidateAction::RecordFirst(leaf.clone())
    );
}

/// ARCH-2 / TASK-0918: a non-NotFound canonicalize failure still
/// surfaces as a typed `FindWorkspaceRootError` variant so it remains
/// investigable. Use a 0o000-permission directory on Unix to force a
/// `PermissionDenied` at canonicalize time.
///
/// The exact variant (`CanonicalizeFailed` vs `NotFound`) depends on
/// how the kernel surfaces EACCES on a descendant — Linux and macOS
/// differ — so the assertion accepts either.
///
/// TEST-18 / TASK-1802: the 0o000 directory is owned by a `Drop` guard so a
/// panic in the body cannot leak an undeletable directory past the
/// `TempDir`, and the guard probes whether the mode bits are enforced at all
/// — as uid 0 the traversal succeeds and there is no error to assert on.
#[cfg(unix)]
#[test]
fn find_root_canonicalize_perm_denied_returns_typed_error() {
    let temp_dir = tempfile::tempdir().expect("create temp dir");
    let locked = temp_dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    let inside = locked.join("inner");
    fs::create_dir(&inside).unwrap();

    // Probe the operation actually under test: traversing `locked` to
    // canonicalize its child.
    let Some(_guard) = PermGuard::deny_all(&locked, |_| fs::canonicalize(&inside).map(|_| ()))
    else {
        skip_no_dac_enforcement("find_root_canonicalize_perm_denied_returns_typed_error");
        return;
    };

    let err = find_workspace_root(&inside).unwrap_err();
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

/// PERF-3 / TASK-1512: `[workspace.metadata]` (a sub-table of workspace)
/// must still count as declaring a workspace.
#[test]
fn content_declares_workspace_subtable() {
    let content = "[workspace.metadata]\nfoo = \"bar\"\n";
    assert!(content_declares_workspace(content));
}

/// PERF-3 / TASK-1512: a string value containing `[workspace]` must NOT
/// register as declaring a workspace — only real table headers count.
#[test]
fn content_declares_workspace_ignores_string_value() {
    let content = r#"
[package]
name = "fake"
description = """
This package has a [workspace] reference in a string.
"""
version = "0.1.0"
"#;
    assert!(!content_declares_workspace(content));
}

/// PERF-3 / TASK-1512: literal multi-line strings (''') also suppress false
/// positives.
#[test]
fn content_declares_workspace_ignores_literal_multiline_string() {
    let content = "[package]\nname = 'x'\ndoc = '''\n[workspace]\n'''\nversion = \"1.0\"\n";
    assert!(!content_declares_workspace(content));
}

/// PERF-3 / TASK-1512: a real `[workspace]` header is detected.
#[test]
fn content_declares_workspace_detects_bare_workspace() {
    let content = "[workspace]\nmembers = [\"crates/*\"]\n";
    assert!(content_declares_workspace(content));
}

/// SEC-11 / TASK-1781: a trailing comment on the table header is ordinary,
/// valid TOML. Missing it made the walk climb past the real workspace root
/// into attacker-plantable ancestors.
#[test]
fn content_declares_workspace_accepts_trailing_comment() {
    for content in [
        "[workspace] # the workspace root\nmembers = []\n",
        "[workspace]# no space before the comment\n",
        "[workspace.package]   # shared metadata\nversion = \"1.0.0\"\n",
        "  [workspace]\t# indented header with a tab-separated comment\n",
    ] {
        assert!(
            content_declares_workspace(content),
            "should detect workspace in: {content:?}"
        );
    }
}

/// SEC-11 / TASK-1781: quoted bare keys are valid TOML table keys.
#[test]
fn content_declares_workspace_accepts_quoted_key() {
    for content in [
        "[\"workspace\"]\nmembers = []\n",
        "[ \"workspace\" ]\nmembers = []\n",
        "['workspace']\nmembers = []\n",
        "[ 'workspace' . package ] # shared\nversion = \"1.0.0\"\n",
        "[\"workspace\".package]\nversion = \"1.0.0\"\n",
    ] {
        assert!(
            content_declares_workspace(content),
            "should detect workspace in: {content:?}"
        );
    }
}

/// SEC-11 / TASK-1781: the accept-side widening must not weaken the existing
/// false-positive guarantees.
#[test]
fn content_declares_workspace_rejects_non_headers() {
    for content in [
        // A `#`-commented header line is not a header.
        "# [workspace]\n[package]\nname = \"x\"\n",
        // `[workspace]` inside a triple-quoted basic string.
        "[package]\nname = \"x\"\ndesc = \"\"\"\n[workspace]\n\"\"\"\n",
        // `[workspace]` inside a triple-quoted literal string.
        "[package]\nname = 'x'\ndoc = '''\n[workspace]\n'''\n",
        // A different table whose name merely starts with `workspace`.
        "[workspaces]\nmembers = []\n",
        // An array-of-tables, not a `[workspace]` table.
        "[[workspace]]\n",
        // A quoted key that is not `workspace`.
        "[\"workspace-ish\"]\n",
        // An unterminated header is not a header.
        "[workspace\nmembers = []\n",
    ] {
        assert!(
            !content_declares_workspace(content),
            "should NOT detect workspace in: {content:?}"
        );
    }
}
