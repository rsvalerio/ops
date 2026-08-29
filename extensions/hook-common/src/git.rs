//! Git directory discovery.
//!
//! Walks up from a starting path to locate a repo's `.git` directory,
//! resolving worktree pointer files and rejecting symlinked `.git` entries
//! (supply-chain risk for callers that write into the returned path).

use std::path::{Component, Path, PathBuf};

/// Maximum number of parent directories to walk while searching for `.git`.
/// Bounds the loop so a hostile cwd cannot force us to ascend to `/` repeatedly.
const FIND_GIT_DIR_MAX_DEPTH: usize = 64;

/// SEC-14: maximum net `..` traversal allowed in a relative `gitdir:` pointer.
///
/// Real worktree pointers either use absolute paths or step up at most one or
/// two directories to reach the parent repo's `.git/worktrees/<name>`.
/// A pointer with deeper `..` traversal (e.g. `../../../../../etc`) is the
/// shape of a redirection attack against the hook installer, which writes
/// into the resolved path.
///
/// SEC-14 / TASK-1890: "absolute pointers are what git writes" is not a
/// reason to trust them — the parser cannot tell a pointer git wrote from one
/// someone else dropped into the walk. Absolute targets are validated too;
/// see [`resolve_absolute_gitdir`] for the rule they must satisfy.
const MAX_GITDIR_PARENT_TRAVERSAL: usize = 2;

/// Name of the back-reference file git writes inside a worktree gitdir,
/// holding the absolute path of the worktree's own `.git` pointer file.
const GITDIR_BACKREFERENCE: &str = "gitdir";

/// SEC-33: byte cap for the back-reference file read.
///
/// The file git writes holds a single absolute path, so 64 KiB is orders of
/// magnitude above any legitimate content while keeping a hostile (or
/// device-backed) `gitdir` file from being slurped into memory unbounded.
const MAX_GITDIR_BACKREFERENCE_BYTES: u64 = 64 * 1024;

/// Find the `.git` directory by walking up from the given path.
///
/// Handles three cases:
/// 1. Plain repos: `.git` is a real directory (symlinked `.git` is rejected).
/// 2. Worktrees / submodules: `.git` is a regular file with body
///    `gitdir: <path>`. The path is resolved relative to the working copy root
///    and returned.
/// 3. Otherwise walks up to the parent, up to [`FIND_GIT_DIR_MAX_DEPTH`] times.
///
/// Symlinked `.git` entries are deliberately skipped: callers like the hook
/// installer write into this directory and a redirected symlink is a
/// supply-chain risk. The returned path is canonicalised so downstream
/// consumers see a stable, real location.
///
/// There is no caller-supplied root ceiling — the depth limit serves as the
/// bound. Pass an already-canonicalised input if the caller has a stricter
/// containment requirement.
#[must_use]
pub fn find_git_dir(from: &Path) -> Option<PathBuf> {
    let mut dir = from.to_path_buf();
    for _ in 0..FIND_GIT_DIR_MAX_DEPTH {
        if let Some(found) = probe_git_entry(&dir.join(".git")) {
            return Some(found);
        }
        if !dir.pop() {
            return None;
        }
    }
    None
}

fn probe_git_entry(candidate: &Path) -> Option<PathBuf> {
    let meta = std::fs::symlink_metadata(candidate).ok()?;
    let ft = meta.file_type();
    // Symlinked .git is skipped silently — never trust it for writes.
    if ft.is_dir() {
        Some(std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.to_path_buf()))
    } else if ft.is_file() {
        let resolved = read_gitdir_pointer(candidate)?;
        Some(std::fs::canonicalize(&resolved).unwrap_or(resolved))
    } else {
        None
    }
}

/// Resolve a `.git` pointer file (worktrees / submodules) to the real gitdir.
///
/// Accepted shape (PATTERN-1 / TASK-1245): a single line of the form
/// `gitdir: <path>\n`, with no leading whitespace before the `gitdir:` token
/// — exactly the format git itself writes. This installer is the path-
/// resolution oracle for hook writes, so the parser must not be wider than
/// the format git produces:
///
/// * Leading whitespace before `gitdir:` is rejected (an indented pointer is
///   not legal git output).
/// * A file with more than one `gitdir:` line is rejected (the second line
///   is silently shadowed by the first under the previous parser; an
///   attacker who can append to the pointer could redirect resolution).
///
/// # Containment
///
/// Both spellings of the target are held to a containment rule — SEC-14 /
/// TASK-1890 closed the asymmetry where a *relative* target had to survive
/// the [`MAX_GITDIR_PARENT_TRAVERSAL`] cap plus a symlink-aware anchor check
/// while an *absolute* one was returned verbatim, straight into a write
/// primitive that produces an executable file git runs on every commit.
///
/// * **Relative**: textual `..` cap, then the canonical result must sit under
///   the anchor exactly [`MAX_GITDIR_PARENT_TRAVERSAL`] levels above the
///   pointer's parent.
/// * **Absolute**: the canonical result must sit under that same anchor, carry
///   git's own back-reference, or be a substantive separate git directory —
///   see [`resolve_absolute_gitdir`].
///
/// Returns `None` (and emits a `tracing::debug!`) for any other shape.
fn read_gitdir_pointer(file: &Path) -> Option<PathBuf> {
    let content = match std::fs::read_to_string(file) {
        Ok(c) => c,
        Err(err) => {
            // A `.git` pointer file we can't read (EACCES, EISDIR, mid-write,
            // etc.) is worth a diagnostic — the walker would otherwise fall
            // through to the parent silently. debug! keeps it out of normal
            // logs while letting `RUST_LOG=ops_hook_common=debug` surface it.
            // ERR-7 (TASK-0937): Debug-format path/error so an
            // attacker-controlled `.git` pointer path cannot inject
            // newlines/ANSI escapes into operator-facing logs.
            tracing::debug!(
                path = ?file.display(),
                error = ?err,
                "failed to read .git pointer file; skipping",
            );
            return None;
        }
    };
    // PATTERN-1 (TASK-1245): require the strict single-line shape git itself
    // writes — no leading whitespace, no second `gitdir:` line. Trim *trailing*
    // whitespace per line (covers `\r` from CRLF endings) but reject any line
    // whose pre-`gitdir:` portion is non-empty.
    let mut hits = content.lines().filter_map(|l| {
        let trimmed_end = l.trim_end();
        let rest = trimmed_end.strip_prefix("gitdir:")?;
        Some(rest)
    });
    let rest = hits.next()?;
    if hits.next().is_some() {
        tracing::debug!(
            path = ?file.display(),
            "gitdir pointer has multiple `gitdir:` lines; refusing to disambiguate"
        );
        return None;
    }
    let target = Path::new(rest.trim());
    let parent = file.parent()?;
    if target.is_absolute() {
        return resolve_absolute_gitdir(file, parent, target);
    }
    if max_parent_escape(target) > MAX_GITDIR_PARENT_TRAVERSAL {
        return None;
    }
    let joined = parent.join(target);
    let anchor = canonical_anchor(parent)?;
    let canonical_target = canonicalize_gitdir_target(&joined)?;
    if !canonical_target.starts_with(&anchor) {
        // ERR-7 (TASK-0937): Debug-format paths to neutralize control
        // characters and ANSI escapes in worktree-root rejection logs.
        tracing::debug!(
            anchor = ?anchor.display(),
            target = ?canonical_target.display(),
            "gitdir pointer escapes worktree-root anchor; rejecting",
        );
        return None;
    }
    Some(canonical_target)
}

/// SEC-14 / TASK-0788: the textual `max_parent_escape` cap is symlink-blind.
/// A pointer like `link/../../etc` has peak textual escape = 1 (well within
/// the cap of 2), but if `link` is a symlink, `canonicalize` follows it and
/// can land the resolved gitdir anywhere on disk. Anchor the canonical
/// resolved path to the ancestor that the textual cap permits — exactly
/// [`MAX_GITDIR_PARENT_TRAVERSAL`] levels above the pointer's parent — so any
/// canonical result that escapes that anchor (via symlink redirection) is
/// refused before downstream code writes into it.
fn canonical_anchor(parent: &Path) -> Option<PathBuf> {
    let anchor_raw = parent
        .ancestors()
        .nth(MAX_GITDIR_PARENT_TRAVERSAL)
        .unwrap_or(parent);
    // ERR-1 / TASK-1004: emit a per-site breadcrumb on canonicalize failure
    // so operators chasing "ops did nothing in this repo" can distinguish
    // (a) no `.git` upstream, (b) SEC-14 escape rejection, and (c) a real
    // canonicalize syscall error. Without this the three failure modes
    // collapsed to the same silent `None`. Debug-format paths/errors per
    // the ERR-7 (TASK-0937) sweep.
    match std::fs::canonicalize(anchor_raw) {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::debug!(
                anchor_raw = ?anchor_raw.display(),
                error = ?e,
                "gitdir pointer: failed to canonicalize SEC-14 anchor; treating as no gitdir"
            );
            None
        }
    }
}

fn canonicalize_gitdir_target(target: &Path) -> Option<PathBuf> {
    match std::fs::canonicalize(target) {
        Ok(p) => Some(p),
        Err(e) => {
            tracing::debug!(
                target = ?target.display(),
                error = ?e,
                "gitdir pointer: failed to canonicalize gitdir target; treating as no gitdir"
            );
            None
        }
    }
}

/// SEC-14 / TASK-1890: validate an **absolute** `gitdir:` target.
///
/// Previously an absolute target was returned verbatim: no `..` cap, no
/// canonicalization, no containment. A `.git` *file* planted anywhere in the
/// walk `find_git_dir` performs — an unpacked archive, a vendored or
/// generated tree, a scratch directory some tool wrote — therefore redirected
/// the whole install, and `install_hook` writes an executable script into
/// `<resolved>/hooks/<hook>` that git runs on every commit. The only check in
/// the way was `paths::is_accepted_git_dir`, which any real repository on the
/// machine satisfies; it is a shape check, not a containment boundary.
///
/// An absolute target is accepted when, after canonicalization, it satisfies
/// **either**:
///
/// 1. **The same anchor as a relative pointer** — it sits under the ancestor
///    [`MAX_GITDIR_PARENT_TRAVERSAL`] levels above the pointer's parent. This
///    covers submodules and worktrees living next to the repo, and gives the
///    absolute spelling exactly the scrutiny the relative one already got.
/// 2. **Git's own back-reference** — `<target>/gitdir` exists and names this
///    very pointer file. `git worktree add` writes that pair in both
///    directions, so a target that points back at us is one git linked to
///    this worktree, not one somebody redirected us to. This keeps genuinely
///    distant worktrees (`git worktree add /elsewhere/wt`) working, which a
///    containment rule alone would break.
/// 3. **The `--separate-git-dir` layout** — the target is a substantive git
///    directory in its own right (`HEAD` regular file plus `objects/` and
///    `refs/` directories). `git init --separate-git-dir=/elsewhere/repo.git`
///    writes an absolute pointer and, unlike `git worktree add`, writes **no**
///    reverse link at all (verified against git 2.53): there is nothing in
///    either direction to prove the pair beyond the target being a real
///    repository. Rules 1 and 2 both reject that layout, so without this
///    branch `find_git_dir` returns `None` for every separate-git-dir
///    checkout whose gitdir sits more than [`MAX_GITDIR_PARENT_TRAVERSAL`]
///    levels away.
///
/// Rule 3 is a substance check, not a containment proof — an attacker who can
/// plant a `.git` file in the walk can point it at any real repository on the
/// machine. What still stands behind it is
/// `paths::canonical_git_dir`, which the installer runs on the resolved path
/// and which additionally requires the directory be named `.git` or sit at
/// `<repo>/.git/worktrees/<name>`.
///
/// Realistic impact of the old shape was bounded by filesystem permissions —
/// the redirect can only land where the invoking user could already write —
/// so this is defence in depth. The asymmetry is what made it a finding: two
/// spellings of the same input got very different scrutiny.
fn resolve_absolute_gitdir(pointer: &Path, parent: &Path, target: &Path) -> Option<PathBuf> {
    let canonical_target = canonicalize_gitdir_target(target)?;
    if canonical_anchor(parent).is_some_and(|anchor| canonical_target.starts_with(&anchor)) {
        return Some(canonical_target);
    }
    if has_gitdir_backreference(&canonical_target, pointer) {
        return Some(canonical_target);
    }
    if is_separate_git_dir(&canonical_target) {
        return Some(canonical_target);
    }
    // ERR-7 (TASK-0937): Debug-format paths, matching the relative-pointer
    // refusal paths.
    tracing::debug!(
        pointer = ?pointer.display(),
        target = ?canonical_target.display(),
        "absolute gitdir pointer is not anchored, back-referenced, or a separate git dir; rejecting",
    );
    None
}

/// True when `dir` has the substance of a repository git directory: a `HEAD`
/// regular file plus `objects/` and `refs/` directories.
///
/// This is what `git init --separate-git-dir=<dir>` produces, and it is the
/// only signal available for that layout — git writes no reverse link for it.
/// A directory that merely *exists* at the pointed-to path does not qualify.
fn is_separate_git_dir(dir: &Path) -> bool {
    dir.join("HEAD").is_file() && dir.join("objects").is_dir() && dir.join("refs").is_dir()
}

/// Read at most `cap` bytes of `path` as UTF-8, mirroring the bounded-read
/// posture of `MAX_GIT_CONFIG_BYTES` in `ops-git`.
///
/// Returns `None` for every failure mode the callers already treat as "no
/// usable content": unreadable, over the cap, or not valid UTF-8.
fn read_capped_to_string(path: &Path, cap: u64) -> Option<String> {
    use std::io::Read as _;

    let mut file = std::fs::File::open(path).ok()?;
    let mut bytes = Vec::new();
    // Read one byte past the cap so an oversize file is distinguishable from
    // one that lands exactly on it.
    let limit = cap.saturating_add(1);
    (&mut file).take(limit).read_to_end(&mut bytes).ok()?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > cap {
        tracing::debug!(
            path = ?path.display(),
            cap,
            "file exceeds byte cap; refusing to parse",
        );
        return None;
    }
    String::from_utf8(bytes).ok()
}

/// True when `gitdir/gitdir` names `pointer` — the reverse link `git worktree
/// add` writes alongside the forward `gitdir:` pointer.
fn has_gitdir_backreference(gitdir: &Path, pointer: &Path) -> bool {
    let Some(recorded) = read_capped_to_string(
        &gitdir.join(GITDIR_BACKREFERENCE),
        MAX_GITDIR_BACKREFERENCE_BYTES,
    ) else {
        return false;
    };
    let Some(canonical_pointer) = canonicalize_gitdir_target(pointer) else {
        return false;
    };
    canonicalize_gitdir_target(Path::new(recorded.trim()))
        .is_some_and(|recorded| recorded == canonical_pointer)
}

/// SEC-14: peak number of directories `path` ascends above its starting point
/// while being walked component-by-component. `a/../../b` peaks at 1 above
/// start, `../../etc` peaks at 2.
///
/// ERR-5 / TASK-0889: track `peak` as `usize` directly so the SEC-14
/// traversal cap cannot be silently fooled by a future refactor that
/// breaks the "peak is non-negative" invariant. The previous shape used
/// `i64` plus `usize::try_from(...).unwrap_or(0)`, whose unreachable
/// fallback would have reported "no escape" for an invariant breach —
/// the worst possible failure mode for a security gate.
fn max_parent_escape(path: &Path) -> usize {
    let mut depth: i64 = 0;
    let mut peak: usize = 0;
    for c in path.components() {
        match c {
            Component::ParentDir => {
                // One step per path component, so the `i64` cannot saturate.
                depth = depth.saturating_sub(1);
                if depth < 0 {
                    let escape = usize::try_from(depth.unsigned_abs()).unwrap_or(usize::MAX);
                    if escape > peak {
                        peak = escape;
                    }
                }
            }
            // One step per path component, so the `i64` cannot saturate.
            Component::Normal(_) => depth = depth.saturating_add(1),
            Component::CurDir | Component::RootDir | Component::Prefix(_) => {}
        }
    }
    peak
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ERR-7 (TASK-0937): tracing fields for git-pointer paths flow through
    /// the `?` formatter so embedded newlines or ANSI escapes cannot forge
    /// log lines. Pin the value-level escape without a tracing-subscriber
    /// dev-dep — mirrors `manifest_io::path_display_debug_escapes_*`.
    #[test]
    fn git_pointer_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/.git");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    #[test]
    fn find_git_dir_in_current() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = dir.path().join(".git");
        std::fs::create_dir(&git).unwrap();
        let expected = std::fs::canonicalize(&git).unwrap();
        assert_eq!(find_git_dir(dir.path()), Some(expected));
    }

    #[test]
    fn find_git_dir_in_parent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let git = dir.path().join(".git");
        std::fs::create_dir(&git).unwrap();
        let sub = dir.path().join("sub");
        std::fs::create_dir(&sub).unwrap();
        let expected = std::fs::canonicalize(&git).unwrap();
        assert_eq!(find_git_dir(&sub), Some(expected));
    }

    #[test]
    fn find_git_dir_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = find_git_dir(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn find_git_dir_resolves_worktree_pointer_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let real_gitdir = dir.path().join("worktrees/feature");
        std::fs::create_dir_all(&real_gitdir).unwrap();
        let worktree = dir.path().join("checkout");
        std::fs::create_dir(&worktree).unwrap();
        let pointer = worktree.join(".git");
        std::fs::write(&pointer, format!("gitdir: {}\n", real_gitdir.display())).unwrap();
        let expected = std::fs::canonicalize(&real_gitdir).unwrap();
        assert_eq!(find_git_dir(&worktree), Some(expected));
    }

    #[cfg(unix)]
    #[test]
    fn find_git_dir_skips_symlinked_dot_git() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outside = dir.path().join("attacker_repo");
        std::fs::create_dir(&outside).unwrap();
        let workspace = dir.path().join("workspace");
        std::fs::create_dir(&workspace).unwrap();
        std::os::unix::fs::symlink(&outside, workspace.join(".git")).unwrap();
        // Symlinked .git is skipped; with no real .git anywhere, the walk fails.
        assert_eq!(find_git_dir(&workspace), None);
    }

    /// SEC-14: a relative `gitdir:` pointer that traverses several parents to
    /// land on something like `/etc/passwd` must be rejected, even if the
    /// attacker plants a HEAD file in the resolved target so `looks_like_git_dir`
    /// would otherwise accept it.
    #[test]
    fn find_git_dir_rejects_excessive_parent_traversal_in_pointer() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Build a deep-enough chain so `../../../<target>` actually resolves
        // to a real planted directory inside the tempdir.
        let chain = dir.path().join("a/b/c");
        std::fs::create_dir_all(&chain).unwrap();
        let target = dir.path().join("etc_passwd");
        std::fs::create_dir(&target).unwrap();
        // Plant a HEAD file so a downstream looks_like_git_dir check would
        // otherwise accept the redirected target.
        std::fs::write(target.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        let pointer = chain.join(".git");
        std::fs::write(&pointer, "gitdir: ../../../etc_passwd\n").unwrap();

        // No real .git anywhere in the ancestor chain — only the planted
        // pointer. With the SEC-14 traversal bound the pointer is refused
        // and the walk falls through to None.
        assert_eq!(find_git_dir(&chain), None);
    }

    #[test]
    fn max_parent_escape_counts_peak_traversal() {
        assert_eq!(max_parent_escape(Path::new("../actual")), 1);
        assert_eq!(max_parent_escape(Path::new("../../../etc")), 3);
        // Net 1 step up but peak is 2.
        assert_eq!(max_parent_escape(Path::new("../../foo/bar")), 2);
        // No escape — `a/..` cancels out.
        assert_eq!(max_parent_escape(Path::new("a/../b")), 0);
    }

    #[cfg(unix)]
    #[test]
    fn unreadable_gitdir_pointer_is_logged_at_debug() {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::sync::{Arc, Mutex};

        #[derive(Clone)]
        struct VecWriter(Arc<Mutex<Vec<u8>>>);
        impl Write for VecWriter {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().write(buf)
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for VecWriter {
            type Writer = Self;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        let dir = tempfile::tempdir().expect("tempdir");
        let pointer = dir.path().join(".git");
        std::fs::write(&pointer, "gitdir: /tmp/whatever\n").unwrap();
        // Make the pointer unreadable by the current user.
        std::fs::set_permissions(&pointer, std::fs::Permissions::from_mode(0o000)).unwrap();

        let buf = Arc::new(Mutex::new(Vec::<u8>::new()));
        let writer = VecWriter(Arc::clone(&buf));
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(writer)
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            // Walking from the workspace root finds *its* .git, not ours, so
            // probe the entry directly to exercise the read-error path.
            assert!(probe_git_entry(&pointer).is_none());
        });

        // Restore permissions so tempdir cleanup succeeds.
        let _ = std::fs::set_permissions(&pointer, std::fs::Permissions::from_mode(0o644));

        let logged = String::from_utf8(buf.lock().unwrap().clone()).unwrap();
        assert!(
            logged.contains("failed to read .git pointer file"),
            "expected debug log, got: {logged}",
        );
    }

    /// SEC-14 / TASK-0788: a relative pointer using the Normal-then-ParentDir
    /// cancellation pattern (`link/../../target`) has peak textual escape = 1
    /// and slips past `MAX_GITDIR_PARENT_TRAVERSAL`. If `link` is a symlink to
    /// a sibling directory outside the worktree-root anchor, `canonicalize`
    /// follows it and the resolved gitdir lands outside the anchor. The
    /// post-canonicalize containment check rejects the pointer before the
    /// hook installer would write into the redirected target.
    #[cfg(unix)]
    #[test]
    fn find_git_dir_rejects_symlink_redirect_through_cancellation_pattern() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Pointer is nested deep enough that nth(MAX_GITDIR_PARENT_TRAVERSAL)
        // anchors *inside* the tempdir rather than at /tmp, so a symlink that
        // jumps to a tempdir-level sibling provably escapes the anchor.
        let pointer_parent = dir.path().join("w/a/b/c/d");
        std::fs::create_dir_all(&pointer_parent).unwrap();
        // Sibling escape target one level below tempdir root, outside the
        // anchor (which is `<tempdir>/w/a/b`).
        let escape_target = dir.path().join("escape_target");
        std::fs::create_dir(&escape_target).unwrap();
        // Plant a HEAD so a downstream `looks_like_git_dir` check would
        // otherwise accept the redirected target.
        std::fs::write(escape_target.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        // Symlink that, once followed by canonicalize, redirects upward
        // through the cancellation pattern.
        let symlink = pointer_parent.join("sym");
        std::os::unix::fs::symlink(dir.path(), &symlink).unwrap();

        let pointer = pointer_parent.join(".git");
        // Peak textual escape: sym→depth 1, ..→0, escape_target→1 (peak 0).
        // Passes the cap; only the canonicalize-aware containment check can
        // refuse it.
        std::fs::write(&pointer, "gitdir: sym/../escape_target\n").unwrap();

        // No real .git anywhere up the chain — only the planted pointer. The
        // walk must reject the pointer and fall through to None.
        assert_eq!(find_git_dir(&pointer_parent), None);
    }

    /// PATTERN-1 (TASK-1245): an indented `gitdir:` line is not the shape git
    /// writes — refuse it. Previously a leading tab/space slipped through
    /// `strip_prefix("gitdir:")` and resolved as if the file were well-formed.
    #[test]
    fn read_gitdir_pointer_rejects_indented_line() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pointer = dir.path().join(".git");
        std::fs::write(&pointer, "\tgitdir: /tmp/real\n").unwrap();
        assert!(read_gitdir_pointer(&pointer).is_none());

        let pointer2 = dir.path().join(".git2");
        std::fs::write(&pointer2, "  gitdir: /tmp/real\n").unwrap();
        assert!(read_gitdir_pointer(&pointer2).is_none());
    }

    /// PATTERN-1 (TASK-1245): a hand-edited pointer with multiple `gitdir:`
    /// lines must be rejected — the previous shape returned the *first*
    /// match, so an attacker who could append a second line could not
    /// shadow a legitimate first line, but a hand-edit that left two
    /// `gitdir:` lines in place would silently use one and ignore the
    /// other. Refusing forces the operator to fix the file.
    #[test]
    fn read_gitdir_pointer_rejects_multiple_gitdir_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pointer = dir.path().join(".git");
        std::fs::write(&pointer, "gitdir: /attacker\ngitdir: /real\n").unwrap();
        assert!(read_gitdir_pointer(&pointer).is_none());
    }

    /// PATTERN-1 (TASK-1245): single legitimate well-formed pointer still
    /// resolves — the strict-shape contract did not regress the happy path.
    #[test]
    fn read_gitdir_pointer_accepts_single_well_formed_line() {
        let dir = tempfile::tempdir().expect("tempdir");
        let real = dir.path().join("real_gitdir");
        std::fs::create_dir(&real).unwrap();
        let pointer = dir.path().join(".git");
        std::fs::write(&pointer, format!("gitdir: {}\n", real.display())).unwrap();
        let resolved = read_gitdir_pointer(&pointer).expect("resolved");
        assert!(resolved.ends_with("real_gitdir"));
    }

    /// SEC-14 / TASK-1890: an absolute `gitdir:` target that is neither
    /// anchored near the pointer nor back-referenced by git must be refused,
    /// even though the planted target is a perfectly convincing repository
    /// (named `.git`, carrying a `HEAD` regular file) that
    /// `paths::is_accepted_git_dir` would wave through. Before the fix the
    /// absolute branch returned the target verbatim and the hook installer
    /// wrote an executable script into it.
    #[test]
    fn find_git_dir_rejects_unanchored_absolute_pointer() {
        let dir = tempfile::tempdir().expect("tempdir");
        // The pointer sits deep enough that its SEC-14 anchor
        // (`<tmp>/work/a/b`) does not contain the planted target.
        let worktree = dir.path().join("work/a/b/c/checkout");
        std::fs::create_dir_all(&worktree).unwrap();
        let planted = dir.path().join("elsewhere/.git");
        std::fs::create_dir_all(&planted).unwrap();
        std::fs::write(planted.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let pointer = worktree.join(".git");
        let absolute = std::fs::canonicalize(&planted).unwrap();
        std::fs::write(&pointer, format!("gitdir: {}\n", absolute.display())).unwrap();

        // No real `.git` anywhere up the chain — only the planted pointer.
        assert_eq!(find_git_dir(&worktree), None);
    }

    /// SEC-14 / TASK-1890: `git init --separate-git-dir=<dir>` writes an
    /// absolute pointer and **no** reverse link (verified against git 2.53),
    /// so neither the anchor rule nor the back-reference rule accepts it and
    /// `find_git_dir` used to return `None` for the whole checkout. The
    /// substance of the target — `HEAD` plus `objects/` and `refs/` — is the
    /// only proof the layout offers, and it is enough to resolve.
    #[test]
    fn find_git_dir_accepts_external_separate_git_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Deep enough that the SEC-14 anchor (`<tmp>/work/a/b`) cannot
        // contain the external gitdir.
        let worktree = dir.path().join("work/a/b/c/checkout");
        std::fs::create_dir_all(&worktree).unwrap();
        let external = dir.path().join("elsewhere/repo.git");
        std::fs::create_dir_all(external.join("objects")).unwrap();
        std::fs::create_dir_all(external.join("refs")).unwrap();
        std::fs::write(external.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        // No `<gitdir>/gitdir` back-reference: that is the point of the case.
        assert!(!external.join("gitdir").exists());

        let canonical_external = std::fs::canonicalize(&external).unwrap();
        std::fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", canonical_external.display()),
        )
        .unwrap();

        assert_eq!(find_git_dir(&worktree), Some(canonical_external));
    }

    /// The separate-git-dir allowance is a *substance* check, not an open
    /// door: an out-of-scope absolute target that is merely a directory (or a
    /// directory carrying a lone `HEAD`) still fails all three rules.
    #[test]
    fn find_git_dir_rejects_out_of_scope_absolute_target_without_git_substance() {
        let dir = tempfile::tempdir().expect("tempdir");
        let worktree = dir.path().join("work/a/b/c/checkout");
        std::fs::create_dir_all(&worktree).unwrap();
        let planted = dir.path().join("elsewhere/.git");
        std::fs::create_dir_all(&planted).unwrap();
        // A convincing name and a HEAD, but no `objects/` and no `refs/`.
        std::fs::write(planted.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let canonical = std::fs::canonicalize(&planted).unwrap();
        std::fs::write(
            worktree.join(".git"),
            format!("gitdir: {}\n", canonical.display()),
        )
        .unwrap();

        assert_eq!(find_git_dir(&worktree), None);
    }

    /// SEC-14 / TASK-1890: the shape `git worktree add` writes for a worktree
    /// far from its repository — an absolute forward pointer plus the
    /// `<gitdir>/gitdir` back-reference — still resolves. A containment rule
    /// on its own would have broken this, which is why the back-reference is
    /// the second accepted proof.
    #[test]
    fn find_git_dir_accepts_absolute_pointer_with_git_backreference() {
        let dir = tempfile::tempdir().expect("tempdir");
        let gitdir = dir.path().join("repo/.git/worktrees/feature");
        std::fs::create_dir_all(&gitdir).unwrap();
        std::fs::write(gitdir.join("HEAD"), "ref: refs/heads/feature\n").unwrap();
        // Deep enough that the anchor rule alone cannot accept it.
        let worktree = dir.path().join("far/away/from/the/repo/feature");
        std::fs::create_dir_all(&worktree).unwrap();
        let pointer = worktree.join(".git");

        let canonical_gitdir = std::fs::canonicalize(&gitdir).unwrap();
        std::fs::write(
            &pointer,
            format!("gitdir: {}\n", canonical_gitdir.display()),
        )
        .unwrap();
        // The reverse link git writes at the same time.
        std::fs::write(
            gitdir.join("gitdir"),
            format!("{}\n", std::fs::canonicalize(&pointer).unwrap().display()),
        )
        .unwrap();

        assert_eq!(find_git_dir(&worktree), Some(canonical_gitdir));
    }

    /// SEC-14 / TASK-1890: a back-reference that names *someone else's*
    /// pointer proves nothing — the planted pointer must not ride another
    /// worktree's link.
    #[test]
    fn find_git_dir_rejects_absolute_pointer_with_foreign_backreference() {
        let dir = tempfile::tempdir().expect("tempdir");
        let gitdir = dir.path().join("repo/.git/worktrees/feature");
        std::fs::create_dir_all(&gitdir).unwrap();
        std::fs::write(gitdir.join("HEAD"), "ref: refs/heads/feature\n").unwrap();
        let honest = dir.path().join("far/away/honest");
        std::fs::create_dir_all(&honest).unwrap();
        let honest_pointer = honest.join(".git");
        std::fs::write(&honest_pointer, "gitdir: whatever\n").unwrap();
        std::fs::write(
            gitdir.join("gitdir"),
            format!(
                "{}\n",
                std::fs::canonicalize(&honest_pointer).unwrap().display()
            ),
        )
        .unwrap();

        // The attacker's pointer, elsewhere, aiming at the same gitdir.
        let attacker = dir.path().join("far/away/attacker");
        std::fs::create_dir_all(&attacker).unwrap();
        let pointer = attacker.join(".git");
        std::fs::write(
            &pointer,
            format!(
                "gitdir: {}\n",
                std::fs::canonicalize(&gitdir).unwrap().display()
            ),
        )
        .unwrap();

        assert_eq!(find_git_dir(&attacker), None);
    }

    #[test]
    fn find_git_dir_relative_pointer() {
        let dir = tempfile::tempdir().expect("tempdir");
        let worktree = dir.path().join("checkout");
        std::fs::create_dir_all(worktree.join("../actual_gitdir")).unwrap();
        let pointer = worktree.join(".git");
        std::fs::write(&pointer, "gitdir: ../actual_gitdir\n").unwrap();
        let result = find_git_dir(&worktree).expect("should resolve");
        assert!(result.ends_with("actual_gitdir"));
    }
}
