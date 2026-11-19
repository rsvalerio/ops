//! Git directory discovery.
//!
//! Walks up from a starting path to locate a repo's `.git` directory,
//! resolving worktree pointer files and rejecting symlinked `.git` entries
//! (supply-chain risk for callers that write into the returned path).

use std::path::{Component, Path, PathBuf};

use ops_core::text::{read_capped_bytes, read_capped_bytes_following, CappedReadError};

/// Maximum number of parent directories to walk while searching for `.git`.
/// Bounds the loop so a hostile cwd cannot force us to ascend to `/` repeatedly.
const FIND_GIT_DIR_MAX_DEPTH: usize = 64;

/// Maximum net `..` traversal allowed in a relative `gitdir:` pointer.
///
/// Real worktree pointers either use absolute paths or step up at most one or
/// two directories to reach the parent repo's `.git/worktrees/<name>`.
/// A pointer with deeper `..` traversal (e.g. `../../../../../etc`) is the
/// shape of a redirection attack against the hook installer, which writes
/// into the resolved path.
///
/// "Absolute pointers are what git writes" is not a
/// reason to trust them — the parser cannot tell a pointer git wrote from one
/// someone else dropped into the walk. Absolute targets are validated too;
/// see [`resolve_absolute_gitdir`] for the rule they must satisfy.
const MAX_GITDIR_PARENT_TRAVERSAL: usize = 2;

/// Name of the back-reference file git writes inside a worktree gitdir,
/// holding the absolute path of the worktree's own `.git` pointer file.
const GITDIR_BACKREFERENCE: &str = "gitdir";

/// Byte cap for the back-reference file read.
///
/// The file git writes holds a single absolute path, so 64 KiB is orders of
/// magnitude above any legitimate content while keeping a hostile (or
/// device-backed) `gitdir` file from being slurped into memory unbounded.
const MAX_GITDIR_BACKREFERENCE_BYTES: u64 = 64 * 1024;

/// Byte cap for the `.git` pointer file read.
///
/// Git writes a single `gitdir: <path>` line, so 64 KiB loses nothing real,
/// while a planted multi-gigabyte `.git` file anywhere in the ancestor walk
/// can no longer be slurped into memory by every `find_git_dir` caller.
const MAX_GITDIR_POINTER_BYTES: u64 = 64 * 1024;

/// Find the `.git` directory by walking up from the given path.
///
/// Handles three cases:
/// 1. Plain repos: `.git` is a real directory (symlinked `.git` is rejected).
/// 2. Worktrees / submodules: `.git` is a regular file with body
///    `gitdir: <path>`. The path is resolved relative to the working copy root
///    and returned.
/// 3. No `.git` entry at all: walks up to the parent, up to
///    [`FIND_GIT_DIR_MAX_DEPTH`] times.
///
/// The walk **stops at the first `.git` entry it finds**, like git itself.
/// An entry that exists but is refused — a symlink, an unreadable, oversize
/// or malformed pointer file, a pointer whose target fails containment, or
/// any other file type — ends the search with `None`. It never ascends past
/// it, because the enclosing repository does not own a nested checkout, and
/// callers such as the hook installer would otherwise write into an
/// unrelated repository.
///
/// A relative input (including `"."`) is normalised to an absolute path
/// first, via [`std::path::absolute`] — a pure lexical join onto the current
/// directory with no filesystem access and no symlink resolution — because
/// `PathBuf::pop` cannot ascend past the process working directory, so a
/// walk over the relative path itself would stop after one probe.
///
/// Symlinked `.git` entries are deliberately refused: callers like the hook
/// installer write into this directory and a redirected symlink is a
/// supply-chain risk. The returned path is canonicalised so downstream
/// consumers see a stable, real location.
///
/// There is no caller-supplied root ceiling — the depth limit serves as the
/// bound. Pass an already-canonicalised input if the caller has a stricter
/// containment requirement.
#[must_use]
pub fn find_git_dir(from: &Path) -> Option<PathBuf> {
    // `pop` returns false once a relative path has no parent, so without
    // this normalisation a relative input would truncate the walk to one
    // probe and answer "not a git repository" inside a real repo.
    let mut dir = if from.is_absolute() {
        from.to_path_buf()
    } else {
        std::path::absolute(from).unwrap_or_else(|_| from.to_path_buf())
    };
    for _ in 0..FIND_GIT_DIR_MAX_DEPTH {
        match probe_git_entry(&dir.join(".git")) {
            GitEntry::Found(found) => return Some(found),
            GitEntry::Refused => return None,
            GitEntry::Absent => {}
        }
        if !dir.pop() {
            return None;
        }
    }
    None
}

/// What one level of the [`find_git_dir`] walk found at `<dir>/.git`.
#[derive(Debug, PartialEq, Eq)]
enum GitEntry {
    /// No `.git` entry at this level: keep walking up.
    Absent,
    /// A `.git` entry resolved to this gitdir.
    Found(PathBuf),
    /// A `.git` entry exists here but was refused: the walk stops.
    Refused,
}

fn probe_git_entry(candidate: &Path) -> GitEntry {
    let meta = match std::fs::symlink_metadata(candidate) {
        Ok(meta) => meta,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return GitEntry::Absent,
        Err(err) => {
            // Something is there but cannot even be stat'ed: refuse rather
            // than guess, and never fall through to an enclosing repository.
            tracing::debug!(
                path = ?candidate.display(),
                error = ?err,
                "failed to stat .git entry; stopping the walk",
            );
            return GitEntry::Refused;
        }
    };
    let ft = meta.file_type();
    if ft.is_dir() {
        GitEntry::Found(
            std::fs::canonicalize(candidate).unwrap_or_else(|_| candidate.to_path_buf()),
        )
    } else if ft.is_file() {
        read_gitdir_pointer(candidate).map_or(GitEntry::Refused, |resolved| {
            GitEntry::Found(std::fs::canonicalize(&resolved).unwrap_or(resolved))
        })
    } else {
        // A symlinked `.git` (or a FIFO, socket, device) is never trusted for
        // writes, and it still marks this level as the repository root.
        tracing::debug!(
            path = ?candidate.display(),
            "`.git` entry is neither a directory nor a regular file; stopping the walk",
        );
        GitEntry::Refused
    }
}

/// Resolve a `.git` pointer file (worktrees / submodules) to the real gitdir.
///
/// Accepted shape: a single line of the form
/// `gitdir: <path>\n`, with no leading whitespace before the `gitdir:` token
/// — exactly the format git itself writes. This installer is the path-
/// resolution oracle for hook writes, so the parser must not be wider than
/// the format git produces:
///
/// * Leading whitespace before `gitdir:` is rejected (an indented pointer is
///   not legal git output).
/// * A file with more than one `gitdir:` line is rejected (picking one
///   would silently ignore the other, and git never writes two; the operator
///   has to fix the file).
///
/// # Containment
///
/// Both spellings of the target are held to a containment rule, because the
/// resolved path feeds a write primitive that produces an executable file git
/// runs on every commit: a *relative* target has to survive the
/// [`MAX_GITDIR_PARENT_TRAVERSAL`] cap plus a symlink-aware anchor check, and
/// an *absolute* one is never returned verbatim.
///
/// * **Relative**: textual `..` cap, then the canonical result must sit under
///   the anchor at most [`MAX_GITDIR_PARENT_TRAVERSAL`] levels above the
///   pointer's parent — floored below the filesystem root, see
///   [`anchor_ancestor`].
/// * **Absolute**: the canonical result must sit under that same anchor, carry
///   git's own back-reference, or be a substantive separate git directory —
///   see [`resolve_absolute_gitdir`].
///
/// Returns `None` (and emits a `tracing::debug!`) for any other shape.
fn read_gitdir_pointer(file: &Path) -> Option<PathBuf> {
    // The pointer is reached by walking up from the caller's starting
    // directory, which is not canonicalized: a symlinked ancestor (`/tmp` on
    // macOS, a symlinked home or projects directory) is ordinary, so the
    // strict no-symlink walk would refuse legitimate checkouts. The final
    // component was already `lstat`ed as a regular file by the probe, and the
    // following open still refuses a FIFO swapped in since then.
    let content = match capped_utf8(read_capped_bytes_following(file, MAX_GITDIR_POINTER_BYTES)) {
        Ok(c) => c,
        Err(err) => {
            // A `.git` pointer file we can't read (EACCES, EISDIR, mid-write,
            // over the byte cap, not UTF-8) is worth a diagnostic — the walk
            // would otherwise stop silently. debug! keeps it out of normal
            // logs while letting `RUST_LOG=ops_hook_common=debug` surface it.
            // Debug-format path/error so an
            // attacker-controlled `.git` pointer path cannot inject
            // newlines/ANSI escapes into operator-facing logs.
            tracing::debug!(
                path = ?file.display(),
                error = %err,
                "failed to read .git pointer file; refusing it",
            );
            return None;
        }
    };
    // Require the strict single-line shape git itself
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
        // Debug-format paths to neutralize control
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

/// The textual `max_parent_escape` cap is symlink-blind.
/// A pointer like `link/../../etc` has peak textual escape = 1 (well within
/// the cap of 2), but if `link` is a symlink, `canonicalize` follows it and
/// can land the resolved gitdir anywhere on disk. Anchor the canonical
/// resolved path to the ancestor that the textual cap permits — exactly
/// [`MAX_GITDIR_PARENT_TRAVERSAL`] levels above the pointer's parent — so any
/// canonical result that escapes that anchor (via symlink redirection) is
/// refused before downstream code writes into it.
/// The ancestor is picked with [`anchor_ancestor`], which floors the walk
/// below the filesystem root. An unconditional `nth` would make the anchor `/`
/// for any pointer whose parent is two or fewer components deep
/// (`/srv/checkout/.git`), and `starts_with("/")` holds for every path on the
/// machine — a containment gate that proves nothing.
/// A canonical anchor that is still rootlike (a shallow ancestor that
/// canonicalizes to `/`) is refused outright with a breadcrumb rather than
/// waved through as a vacuous check.
fn canonical_anchor(parent: &Path) -> Option<PathBuf> {
    let Some(anchor_raw) = anchor_ancestor(parent) else {
        tracing::debug!(
            parent = ?parent.display(),
            "gitdir pointer: canonical anchor would be the filesystem root; refusing to resolve"
        );
        return None;
    };
    // Emit a per-site breadcrumb on canonicalize failure
    // so operators chasing "ops did nothing in this repo" can distinguish
    // (a) no `.git` upstream, (b) escape rejection, and (c) a real
    // canonicalize syscall error. Without this the three failure modes
    // collapsed to the same silent `None`. Debug-format paths/errors.
    match std::fs::canonicalize(anchor_raw) {
        Ok(p) if p.parent().is_none() => {
            tracing::debug!(
                anchor_raw = ?anchor_raw.display(),
                anchor = ?p.display(),
                "gitdir pointer: anchor canonicalizes to the filesystem root; refusing to resolve"
            );
            None
        }
        Ok(p) => Some(p),
        Err(e) => {
            tracing::debug!(
                anchor_raw = ?anchor_raw.display(),
                error = ?e,
                "gitdir pointer: failed to canonicalize anchor; treating as no gitdir"
            );
            None
        }
    }
}

/// The lexical ancestor the containment check anchors to: at most
/// [`MAX_GITDIR_PARENT_TRAVERSAL`] levels above the pointer's parent, but
/// never the filesystem root itself.
///
/// Flooring here is what keeps the anchor discriminating for a shallow
/// pointer. `/srv/checkout/.git` anchors at `/srv`, not `/`, which still
/// admits every layout the cap was written for
/// (a sibling gitdir, a submodule next to the repo) while refusing a symlink
/// that jumps elsewhere on the machine. `None` means every permitted ancestor
/// *is* the root — the caller refuses to resolve rather than applying a check
/// that admits everything.
fn anchor_ancestor(parent: &Path) -> Option<&Path> {
    let mut anchor = None;
    for candidate in parent.ancestors().take(MAX_GITDIR_PARENT_TRAVERSAL + 1) {
        // `parent() == None` identifies a root (`/`, or a Windows prefix root)
        // and, for relative inputs, the empty path that terminates the walk.
        if candidate.parent().is_none() {
            break;
        }
        anchor = Some(candidate);
    }
    anchor
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

/// Validate an **absolute** `gitdir:` target.
///
/// An absolute target cannot be trusted verbatim. A `.git` *file* planted
/// anywhere in the walk `find_git_dir` performs — an unpacked archive, a
/// vendored or generated tree, a scratch directory some tool wrote — would
/// otherwise redirect the whole install, and `install_hook` writes an
/// executable script into `<resolved>/hooks/<hook>` that git runs on every
/// commit. `paths::is_accepted_git_dir` does not stand in the way: any real
/// repository on the machine satisfies it; it is a shape check, not a
/// containment boundary.
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
/// Impact stays bounded by filesystem permissions — the redirect can only
/// land where the invoking user could already write — so these rules are
/// defence in depth on top of them.
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
    // Debug-format paths, matching the relative-pointer
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

/// Decode a capped read as strict UTF-8, mirroring the bounded-read posture
/// of `MAX_GIT_CONFIG_BYTES` in `ops-git`.
///
/// Every failure mode — unreadable, a FIFO or other non-regular file, over
/// the cap, not valid UTF-8 — comes back typed so each caller can leave a
/// breadcrumb naming its own file. Both opens are `O_NONBLOCK` with a
/// regular-file check on the handle, so a named pipe planted at either path
/// is refused instead of hanging the walk.
fn capped_utf8(read: Result<Vec<u8>, CappedReadError>) -> Result<String, CappedReadError> {
    String::from_utf8(read?).map_err(|e| CappedReadError::InvalidUtf8(e.utf8_error()))
}

/// True when `gitdir/gitdir` names `pointer` — the reverse link `git worktree
/// add` writes alongside the forward `gitdir:` pointer.
fn has_gitdir_backreference(gitdir: &Path, pointer: &Path) -> bool {
    let backreference = gitdir.join(GITDIR_BACKREFERENCE);
    // `gitdir` is canonical, so its prefix holds no symlink, and git writes the
    // back-reference as a plain file: the strict no-symlink read applies.
    let recorded =
        match capped_utf8(read_capped_bytes(&backreference, MAX_GITDIR_BACKREFERENCE_BYTES)) {
            Ok(recorded) => recorded,
            Err(err) => {
                tracing::debug!(
                    path = ?backreference.display(),
                    error = %err,
                    "no usable gitdir back-reference",
                );
                return false;
            }
        };
    let Some(canonical_pointer) = canonicalize_gitdir_target(pointer) else {
        return false;
    };
    canonicalize_gitdir_target(Path::new(recorded.trim()))
        .is_some_and(|recorded| recorded == canonical_pointer)
}

/// Peak number of directories `path` ascends above its starting point
/// while being walked component-by-component. `a/../../b` peaks at 1 above
/// start, `../../etc` peaks at 2.
///
/// `peak` is tracked as a `usize` directly, so no conversion sits between
/// the walk and the traversal cap: a signed peak converted with a
/// fallback (`usize::try_from(...).unwrap_or(0)`) would report "no escape"
/// if the "peak is non-negative" invariant were ever broken — the worst
/// possible failure mode for a security gate.
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

    /// Tracing fields for git-pointer paths flow through
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

    /// A relative input is normalised to absolute before the walk
    /// (`PathBuf::pop` returns false once a relative path has no parent), so
    /// `find_git_dir(Path::new("."))` finds the repository from a
    /// subdirectory. Driven through a cwd guard from several levels below
    /// the repo root.
    #[test]
    #[serial_test::serial]
    fn find_git_dir_walks_up_from_a_relative_starting_path() {
        use crate::test_helpers::CwdGuard;

        let dir = tempfile::tempdir().expect("tempdir");
        let git = dir.path().join(".git");
        std::fs::create_dir(&git).unwrap();
        let deep = dir.path().join("a/b/c");
        std::fs::create_dir_all(&deep).unwrap();
        let expected = std::fs::canonicalize(&git).unwrap();

        let _cwd = CwdGuard::new(&deep).expect("CwdGuard");
        // "." must walk from the cwd, and a bare subdirectory name must walk
        // from the cwd-relative subdirectory — both several levels below the
        // repo root.
        assert_eq!(find_git_dir(Path::new(".")), Some(expected.clone()));
        std::fs::create_dir_all("a/b/c").unwrap();
        assert_eq!(
            find_git_dir(Path::new("a/b/c")),
            Some(expected.clone()),
            "a relative subdir must walk up to the repo root"
        );
        assert_eq!(
            find_git_dir(Path::new("..")),
            Some(expected),
            "a relative parent hop must walk up to the repo root"
        );
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
        // A symlinked .git is refused, and the walk stops there.
        assert_eq!(find_git_dir(&workspace), None);
    }

    /// A refused `.git` entry in a nested directory stops the walk: it must
    /// never ascend to the real repository enclosing it, or the hook
    /// installer writes into a repository the operator did not ask about.
    #[test]
    fn find_git_dir_stops_at_a_refused_entry_instead_of_using_the_enclosing_repo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let outer_git = dir.path().join(".git");
        std::fs::create_dir(&outer_git).unwrap();
        let outer_git = std::fs::canonicalize(&outer_git).unwrap();

        // A malformed pointer, and one whose relative target escapes the
        // traversal cap: both are refused at the nested level.
        let malformed = dir.path().join("malformed");
        std::fs::create_dir(&malformed).unwrap();
        std::fs::write(malformed.join(".git"), "not a pointer\n").unwrap();
        let escaping = dir.path().join("x/y/escaping");
        std::fs::create_dir_all(&escaping).unwrap();
        std::fs::write(escaping.join(".git"), "gitdir: ../../../../elsewhere\n").unwrap();

        for nested in [&malformed, &escaping] {
            let sub = nested.join("src");
            std::fs::create_dir(&sub).unwrap();
            assert_eq!(find_git_dir(nested), None, "{}", nested.display());
            assert_eq!(find_git_dir(&sub), None, "{}", sub.display());
        }

        #[cfg(unix)]
        {
            let symlinked = dir.path().join("symlinked");
            std::fs::create_dir(&symlinked).unwrap();
            std::os::unix::fs::symlink(&outer_git, symlinked.join(".git")).unwrap();
            assert_eq!(find_git_dir(&symlinked), None);
        }

        // Control: with no `.git` entry in between, the outer repo is found.
        let plain = dir.path().join("plain/src");
        std::fs::create_dir_all(&plain).unwrap();
        assert_eq!(find_git_dir(&plain), Some(outer_git));
    }

    /// A `.git` pointer file over [`MAX_GITDIR_POINTER_BYTES`] is refused with
    /// a debug breadcrumb instead of being read whole.
    #[test]
    fn oversize_gitdir_pointer_is_refused_with_a_breadcrumb() {
        let dir = tempfile::tempdir().expect("tempdir");
        let real = dir.path().join("real_gitdir");
        std::fs::create_dir(&real).unwrap();
        let pointer = dir.path().join(".git");
        // A well-formed pointer line followed by padding past the cap, so
        // only the size check can refuse it.
        let mut body = format!("gitdir: {}\n", real.display());
        body.push_str(&"\n".repeat(usize::try_from(MAX_GITDIR_POINTER_BYTES).unwrap()));
        std::fs::write(&pointer, body).unwrap();

        let (logged, ()) = ops_core::test_utils::capture_tracing(tracing::Level::DEBUG, || {
            assert_eq!(read_gitdir_pointer(&pointer), None);
        });
        assert!(logged.contains("failed to read .git pointer file"), "got: {logged}");
        assert!(logged.contains("byte cap"), "got: {logged}");
    }

    /// A relative `gitdir:` pointer that traverses several parents to
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
        // pointer. With the traversal bound the pointer is refused
        // and the walk stops with None.
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
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let pointer = dir.path().join(".git");
        std::fs::write(&pointer, "gitdir: /tmp/whatever\n").unwrap();
        // Make the pointer unreadable by the current user.
        std::fs::set_permissions(&pointer, std::fs::Permissions::from_mode(0o000)).unwrap();

        let (logged, ()) = ops_core::test_utils::capture_tracing(tracing::Level::DEBUG, || {
            // Walking from the workspace root finds *its* .git, not ours, so
            // probe the entry directly to exercise the read-error path.
            assert_eq!(probe_git_entry(&pointer), GitEntry::Refused);
        });

        // Restore permissions so tempdir cleanup succeeds.
        let _ = std::fs::set_permissions(&pointer, std::fs::Permissions::from_mode(0o644));

        assert!(
            logged.contains("failed to read .git pointer file"),
            "expected debug log, got: {logged}",
        );
    }

    /// A relative pointer using the Normal-then-ParentDir
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
        // walk must reject the pointer and stop with None.
        assert_eq!(find_git_dir(&pointer_parent), None);
    }

    /// The anchor must never degenerate to a path that
    /// contains every candidate target. Pinned lexically because the
    /// degenerate layouts (`/srv/checkout`, `/repo`, `/`) are not ones a test
    /// can create on a real filesystem.
    #[test]
    fn anchor_ancestor_is_floored_below_the_filesystem_root() {
        // Deep enough: the full MAX_GITDIR_PARENT_TRAVERSAL climb applies.
        assert_eq!(anchor_ancestor(Path::new("/a/b/c/d")), Some(Path::new("/a/b")));
        // Shallow: the unfloored climb would land on `/`, which contains
        // everything. Floor at the deepest non-root ancestor instead.
        assert_eq!(anchor_ancestor(Path::new("/srv/checkout")), Some(Path::new("/srv")));
        assert_eq!(anchor_ancestor(Path::new("/repo")), Some(Path::new("/repo")));
        // Nothing but the root is available — refuse rather than admit
        // every path on the machine.
        assert_eq!(anchor_ancestor(Path::new("/")), None);
    }

    /// A pointer sitting directly at the tempdir root has a shallow
    /// parent (`/tmp/.tmpXXXX`): an unfloored anchor
    /// `parent.ancestors().nth(2)` would be `/`, and
    /// `canonical_target.starts_with("/")` holds for every path on the
    /// machine — a containment gate that proves nothing. The floored
    /// anchor refuses a symlink jumping clean out of the temp tree.
    #[cfg(unix)]
    #[test]
    fn find_git_dir_rejects_out_of_tree_redirect_from_a_shallow_pointer() {
        let dir = tempfile::tempdir().expect("tempdir");

        // Sanity: this fixture really is the shallow shape the finding
        // describes — without the floor the anchor would be the root.
        let unfloored =
            dir.path().ancestors().nth(MAX_GITDIR_PARENT_TRAVERSAL).unwrap_or_else(|| dir.path());
        if unfloored.parent().is_some() {
            // TMPDIR is nested deeper than /tmp on this machine, so the
            // degenerate case cannot be reproduced here. The lexical test
            // above still pins the floor.
            return;
        }

        // A real git directory well outside the temp tree: the crate's own
        // source directory stands in for "somewhere else on the machine".
        let out_of_tree = Path::new(env!("CARGO_MANIFEST_DIR"));
        let symlink = dir.path().join("sym");
        std::os::unix::fs::symlink(out_of_tree, &symlink).unwrap();

        let pointer = dir.path().join(".git");
        // Textual escape is 0, so only the anchor check can refuse this.
        std::fs::write(&pointer, "gitdir: sym\n").unwrap();

        assert_eq!(
            read_gitdir_pointer(&pointer),
            None,
            "a shallow pointer must not get a vacuous containment anchor"
        );
    }

    /// An indented `gitdir:` line is not the shape git writes, so a leading
    /// tab or space is refused rather than trimmed away.
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

    /// A pointer with multiple `gitdir:` lines must be rejected: resolving
    /// it would silently use one line and ignore the other. Refusing forces
    /// the operator to fix the file.
    #[test]
    fn read_gitdir_pointer_rejects_multiple_gitdir_lines() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pointer = dir.path().join(".git");
        std::fs::write(&pointer, "gitdir: /attacker\ngitdir: /real\n").unwrap();
        assert!(read_gitdir_pointer(&pointer).is_none());
    }

    /// A single well-formed pointer resolves: the strict-shape contract
    /// admits the happy path.
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

    /// An absolute `gitdir:` target that is neither
    /// anchored near the pointer nor back-referenced by git must be refused,
    /// even though the planted target is a perfectly convincing repository
    /// (named `.git`, carrying a `HEAD` regular file) that
    /// `paths::is_accepted_git_dir` would wave through — the hook installer
    /// would otherwise write an executable script into it.
    #[test]
    fn find_git_dir_rejects_unanchored_absolute_pointer() {
        let dir = tempfile::tempdir().expect("tempdir");
        // The pointer sits deep enough that its canonical anchor
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

    /// `git init --separate-git-dir=<dir>` writes an
    /// absolute pointer and **no** reverse link (verified against git 2.53),
    /// so neither the anchor rule nor the back-reference rule accepts it. The
    /// substance of the target — `HEAD` plus `objects/` and `refs/` — is the
    /// only proof the layout offers, and it is enough to resolve.
    #[test]
    fn find_git_dir_accepts_external_separate_git_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Deep enough that the canonical anchor (`<tmp>/work/a/b`) cannot
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
        std::fs::write(worktree.join(".git"), format!("gitdir: {}\n", canonical.display()))
            .unwrap();

        assert_eq!(find_git_dir(&worktree), None);
    }

    /// The shape `git worktree add` writes for a worktree
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
        std::fs::write(&pointer, format!("gitdir: {}\n", canonical_gitdir.display())).unwrap();
        // The reverse link git writes at the same time.
        std::fs::write(
            gitdir.join("gitdir"),
            format!("{}\n", std::fs::canonicalize(&pointer).unwrap().display()),
        )
        .unwrap();

        assert_eq!(find_git_dir(&worktree), Some(canonical_gitdir));
    }

    /// A named pipe planted as the `<gitdir>/gitdir` back-reference is
    /// refused at the open, so `find_git_dir` returns instead of blocking
    /// until a writer appears.
    #[cfg(unix)]
    #[test]
    fn find_git_dir_refuses_fifo_backreference_without_blocking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let gitdir = dir.path().join("repo/.git/worktrees/feature");
        std::fs::create_dir_all(&gitdir).unwrap();
        std::fs::write(gitdir.join("HEAD"), "ref: refs/heads/feature\n").unwrap();
        let worktree = dir.path().join("far/away/from/the/repo/feature");
        std::fs::create_dir_all(&worktree).unwrap();
        let canonical_gitdir = std::fs::canonicalize(&gitdir).unwrap();
        std::fs::write(worktree.join(".git"), format!("gitdir: {}\n", canonical_gitdir.display()))
            .unwrap();
        let status = std::process::Command::new("mkfifo")
            .arg(gitdir.join("gitdir"))
            .status()
            .expect("run mkfifo");
        assert!(status.success(), "mkfifo failed");

        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let _ = tx.send(find_git_dir(&worktree));
        });
        let outcome = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("find_git_dir blocked on a FIFO back-reference instead of refusing it");
        handle.join().unwrap();
        // Neither anchored, back-referenced, nor a separate git dir.
        assert_eq!(outcome, None);
    }

    /// The pointer read follows symlinks (its prefix is the caller's
    /// unresolved starting directory) but still refuses a FIFO without
    /// blocking.
    #[cfg(unix)]
    #[test]
    fn read_gitdir_pointer_refuses_fifo_without_blocking() {
        let dir = tempfile::tempdir().expect("tempdir");
        let pointer = dir.path().join(".git");
        let status =
            std::process::Command::new("mkfifo").arg(&pointer).status().expect("run mkfifo");
        assert!(status.success(), "mkfifo failed");

        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let _ = tx.send(read_gitdir_pointer(&pointer));
        });
        let outcome = rx
            .recv_timeout(std::time::Duration::from_secs(10))
            .expect("read_gitdir_pointer blocked on a FIFO instead of refusing it");
        handle.join().unwrap();
        assert_eq!(outcome, None);
    }

    /// A back-reference that names *someone else's*
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
            format!("{}\n", std::fs::canonicalize(&honest_pointer).unwrap().display()),
        )
        .unwrap();

        // The attacker's pointer, elsewhere, aiming at the same gitdir.
        let attacker = dir.path().join("far/away/attacker");
        std::fs::create_dir_all(&attacker).unwrap();
        let pointer = attacker.join(".git");
        std::fs::write(
            &pointer,
            format!("gitdir: {}\n", std::fs::canonicalize(&gitdir).unwrap().display()),
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
