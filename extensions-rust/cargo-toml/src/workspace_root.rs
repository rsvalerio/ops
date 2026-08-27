//! Workspace-root discovery by ancestor walk.
//!
//! Contains [`find_workspace_root`] and its strict variant, the typed
//! [`FindWorkspaceRootError`], and the [`MAX_ANCESTOR_DEPTH`] constant.

use ops_core::text::read_capped_to_string;
use std::fs;
use std::path::{Path, PathBuf};

/// ARCH-2 / TASK-0871: typed errors for [`find_workspace_root`].
///
/// Replaces the previously synthesised `io::Error::new(NotFound, …)`, so
/// consumers (notably `is_manifest_missing` in `extensions-rust/about`)
/// can match a typed variant instead of walking the source chain looking
/// for an `io::ErrorKind::NotFound` shape that another wrapping layer
/// would mask.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FindWorkspaceRootError {
    #[error(
        "no Cargo.toml found in {start} or any parent directory (walked up to {depth} ancestors)"
    )]
    NotFound { start: PathBuf, depth: usize },
    #[error("failed to canonicalize {path}")]
    CanonicalizeFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl FindWorkspaceRootError {
    /// True when this error indicates the search walked to its bound without
    /// finding any `Cargo.toml`. Mirrors the legacy `io::ErrorKind::NotFound`
    /// signal that `is_manifest_missing` consumed.
    #[must_use]
    pub const fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }
}

/// Maximum ancestor depth walked when searching for `Cargo.toml`. Defensive
/// bound that prevents a symlink loop (or pathologically deep mount layout)
/// from spinning the discovery loop forever.
///
/// TASK-0963: exposed as `pub` so callers and tests can reference the same
/// default the high-level [`find_workspace_root`] uses, and so a future
/// caller with a legitimately deeper layout can opt into a larger bound via
/// [`find_workspace_root_with_depth`] instead of patching the crate.
pub const MAX_ANCESTOR_DEPTH: usize = 64;

/// Find the workspace root by walking up from `start` looking for Cargo.toml.
///
/// TASK-0501: prefers the *outermost* `Cargo.toml` containing `[workspace]`
/// over the first `Cargo.toml` encountered. Running from inside a member
/// crate (e.g. `cd crates/foo`) used to return the member manifest; the new
/// walk continues past member manifests until it finds the workspace root.
/// If no manifest in the chain declares `[workspace]`, the first encountered
/// `Cargo.toml` is returned — preserving the single-crate / non-workspace
/// project behaviour.
///
/// The caller's `start` is canonicalized first so symlinks under `start` are
/// resolved once up front, and the walk is capped at [`MAX_ANCESTOR_DEPTH`]
/// so a symlink-induced loop cannot hang the process.
///
/// # Symlink threat model (SEC-25 / TASK-0604 / TASK-1036)
///
/// The walk starts from a single canonicalized path; intermediate ancestors
/// are reached via [`Path::parent`] and are **not** re-canonicalized at each
/// step. Because the starting path is already resolved, its lexical parents
/// are themselves canonical *for the typical, stable filesystem case*.
///
/// The implication callers must understand:
///
/// - A symlinked ancestor that exists (or is swapped in) at walk time can
///   cause the discovered `Cargo.toml` to live outside the user's intended
///   logical path. This covers both the TOCTOU window between the initial
///   `canonicalize` and the per-ancestor `manifest_declares_workspace` read,
///   and the pre-existing case where an ancestor is itself a symlink to a
///   different filesystem location at the moment the walk runs. A malicious
///   workspace that plants a fake `Cargo.toml` 1-2 ancestors above can have
///   it picked up as the workspace root.
/// - The depth cap ([`MAX_ANCESTOR_DEPTH`]) bounds the blast radius — a
///   hostile symlink chain cannot drive an unbounded walk — but it does not
///   prevent the wrong manifest from being selected within that bound.
/// - Callers that need stronger guarantees should canonicalize each
///   candidate `Cargo.toml` themselves (e.g. via [`std::fs::canonicalize`])
///   and verify the result still lies under their expected logical root
///   before trusting it as the workspace root.
///
/// Treat the returned path as "best-effort symlink-safe" rather than
/// absolute.
///
/// # Errors
///
/// [`FindWorkspaceRootError`] if no `Cargo.toml` is found within the
/// ancestor-depth bound, or if an ancestor cannot be read or canonicalized.
pub fn find_workspace_root(start: &Path) -> Result<PathBuf, FindWorkspaceRootError> {
    find_workspace_root_with_depth(start, MAX_ANCESTOR_DEPTH)
}

/// Variant of [`find_workspace_root`] that takes the ancestor-depth bound as
/// a parameter.
///
/// TASK-0963: lets tests verify the bound without crafting a 64-deep
/// directory hierarchy, and gives callers an escape hatch if their layout
/// legitimately needs a deeper walk.
///
/// The same symlink threat model documented on [`find_workspace_root`]
/// applies here: the start path is canonicalized once and ancestors are
/// reached via [`Path::parent`] without re-canonicalization, so a symlinked
/// ancestor at walk time can cause the discovered `Cargo.toml` to live
/// outside the caller's intended logical path. The `max_depth` parameter
/// bounds the blast radius but does not prevent mis-selection within that
/// bound; callers wanting stronger guarantees should canonicalize each
/// candidate `Cargo.toml` themselves.
///
/// # Errors
///
/// [`FindWorkspaceRootError`] if no `Cargo.toml` is found within the
/// ancestor-depth bound, or if an ancestor cannot be read or canonicalized.
pub fn find_workspace_root_with_depth(
    start: &Path,
    max_depth: usize,
) -> Result<PathBuf, FindWorkspaceRootError> {
    walk_ancestors(start, max_depth, |current, cargo_toml, _start_canonical| {
        if manifest_declares_workspace(&cargo_toml) {
            return CandidateAction::AcceptWorkspace(current.to_path_buf());
        }
        CandidateAction::RecordFirst(current.to_path_buf())
    })
}

/// Strict variant of [`find_workspace_root`] that re-canonicalises each
/// candidate `Cargo.toml`'s parent before accepting it as the workspace
/// root.
///
/// Rejects (with [`tracing::warn`]) any candidate whose canonical path
/// does not lie on the canonical start's ancestor chain — i.e. a
/// symlink in the lexical walk that would otherwise redirect the
/// discovered root outside the user's intended logical path.
///
/// SEC-25 / TASK-1204: addresses the symlink-retarget gap documented on
/// [`find_workspace_root`]. The lenient walk reaches each ancestor via
/// [`Path::parent`] on the lexical path of the canonicalized start and
/// reads each candidate by its lexical path, so an attacker who can
/// write inside any reachable ancestor can plant a `Cargo.toml`
/// containing `[workspace]` and have it returned as the root — every
/// downstream provider (units, coverage, deps) then targets the wrong
/// workspace. The strict variant adds a per-candidate canonicalize step
/// so a redirected ancestor is detected and skipped.
///
/// Lenient siblings remain available for callers that explicitly opt
/// out (e.g. legacy `find_workspace_root` / `find_workspace_root_with_depth`),
/// preserving behaviour for tools that rely on the lexical walk.
///
/// # Errors
///
/// [`FindWorkspaceRootError`] if no `Cargo.toml` is found within the
/// ancestor-depth bound, or if an ancestor cannot be read or canonicalized.
///
/// The strict variant additionally rejects a candidate root that does not
/// contain `start` after canonicalization (symlink-planting defence).
pub fn find_workspace_root_strict(start: &Path) -> Result<PathBuf, FindWorkspaceRootError> {
    find_workspace_root_strict_with_depth(start, MAX_ANCESTOR_DEPTH)
}

/// Variant of [`find_workspace_root_strict`] with an explicit ancestor-depth
/// bound. Exposed for tests and for callers whose layout legitimately needs
/// a deeper walk.
///
/// # Errors
///
/// [`FindWorkspaceRootError`] if no `Cargo.toml` is found within the
/// ancestor-depth bound, or if an ancestor cannot be read or canonicalized.
///
/// The strict variant additionally rejects a candidate root that does not
/// contain `start` after canonicalization (symlink-planting defence).
pub fn find_workspace_root_strict_with_depth(
    start: &Path,
    max_depth: usize,
) -> Result<PathBuf, FindWorkspaceRootError> {
    walk_ancestors(
        start,
        max_depth,
        |current, cargo_toml, start_canonical| match fs::canonicalize(current) {
            Ok(canonical_parent) => {
                if start_canonical.starts_with(&canonical_parent) {
                    if manifest_declares_workspace(&cargo_toml) {
                        return CandidateAction::AcceptWorkspace(canonical_parent);
                    }
                    CandidateAction::RecordFirst(canonical_parent)
                } else {
                    tracing::warn!(
                        cargo_toml = ?cargo_toml.display(),
                        lexical_parent = ?current.display(),
                        canonical_parent = ?canonical_parent.display(),
                        canonical_start = ?start_canonical.display(),
                        "SEC-25 / TASK-1204: candidate Cargo.toml's canonical parent escapes the canonical start's ancestor chain; rejecting"
                    );
                    CandidateAction::Skip
                }
            }
            Err(e) => {
                tracing::warn!(
                    path = ?current.display(),
                    error = ?e,
                    "TASK-1204: failed to canonicalize candidate manifest's parent; skipping ancestor"
                );
                CandidateAction::Skip
            }
        },
    )
}

/// Action returned by the per-candidate check closure in [`walk_ancestors`].
enum CandidateAction {
    /// The candidate is the workspace root — return it immediately.
    AcceptWorkspace(PathBuf),
    /// The candidate is a valid Cargo.toml but not a workspace root; record it
    /// as the first-seen fallback if none has been recorded yet.
    RecordFirst(PathBuf),
    /// Skip this candidate entirely (e.g. symlink escapes the ancestor chain).
    Skip,
}

/// Shared ancestor-walk core for both lenient and strict workspace-root
/// discovery. Canonicalises `start`, walks up to `max_depth` ancestors,
/// stats each `Cargo.toml`, and delegates per-candidate decisions to `check`.
fn walk_ancestors(
    start: &Path,
    max_depth: usize,
    check: impl Fn(&Path, PathBuf, &Path) -> CandidateAction,
) -> Result<PathBuf, FindWorkspaceRootError> {
    let start_canonical = match fs::canonicalize(start) {
        Ok(p) => p,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(
                start = ?start.display(),
                "find_workspace_root: start path is unreachable (canonicalize NotFound); reporting NotFound"
            );
            return Err(FindWorkspaceRootError::NotFound {
                start: start.to_path_buf(),
                depth: max_depth,
            });
        }
        Err(source) => {
            return Err(FindWorkspaceRootError::CanonicalizeFailed {
                path: start.to_path_buf(),
                source,
            });
        }
    };
    let mut current = start_canonical.as_path();
    let mut first_cargo_toml: Option<PathBuf> = None;
    for _ in 0..max_depth {
        let cargo_toml = current.join("Cargo.toml");
        let exists = match cargo_toml.try_exists() {
            Ok(b) => b,
            Err(e) => {
                tracing::warn!(
                    path = ?cargo_toml.display(),
                    error = ?e,
                    "Cargo.toml stat failed (non-NotFound IO error); skipping ancestor"
                );
                false
            }
        };
        if exists {
            match check(current, cargo_toml, &start_canonical) {
                CandidateAction::AcceptWorkspace(root) => return Ok(root),
                CandidateAction::RecordFirst(path) => {
                    if first_cargo_toml.is_none() {
                        first_cargo_toml = Some(path);
                    }
                }
                CandidateAction::Skip => {}
            }
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    if let Some(root) = first_cargo_toml {
        return Ok(root);
    }
    Err(FindWorkspaceRootError::NotFound {
        start: start.to_path_buf(),
        depth: max_depth,
    })
}

/// True iff the manifest at `path` contains a top-level `[workspace]` table
/// header. Read errors return false — the walk will keep looking and ultimately
/// fall back to the first Cargo.toml seen.
///
/// PERF-3 / TASK-1512: avoids a full `toml::Value` parse (which allocates the
/// entire AST only to check for one key). Instead performs a line-level scan
/// that recognises TOML table headers of the form `[workspace]` or
/// `[workspace.<anything>]`. Lines inside multi-line strings (triple-quoted
/// basic or literal strings) are excluded so a string value containing the
/// substring `[workspace]` does not produce a false positive.
fn manifest_declares_workspace(path: &Path) -> bool {
    let content = match read_capped_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
        Err(e) => {
            tracing::debug!(
                path = ?path.display(),
                error = ?e,
                "Cargo.toml unreadable during workspace walk; skipping candidate"
            );
            return false;
        }
    };
    content_declares_workspace(&content)
}

/// Line-level scan for a top-level `[workspace]` or `[workspace.*]` header.
///
/// Skips lines that fall inside triple-quoted multi-line strings (`"""` or
/// `'''`). A bare `[workspace]` on a line that is not inside such a string is
/// enough to declare the manifest as workspace-bearing.
pub fn content_declares_workspace(content: &str) -> bool {
    let mut in_multiline_string = false;
    let mut multiline_delim: &str = "\"\"\"";

    for line in content.lines() {
        let trimmed = line.trim();

        if in_multiline_string {
            if trimmed.contains(multiline_delim) {
                in_multiline_string = false;
            }
            continue;
        }

        if !trimmed.starts_with('[') {
            let basic_count = trimmed.matches("\"\"\"").count();
            let literal_count = trimmed.matches("'''").count();
            if basic_count % 2 == 1 {
                in_multiline_string = true;
                multiline_delim = "\"\"\"";
                continue;
            }
            if literal_count % 2 == 1 {
                in_multiline_string = true;
                multiline_delim = "'''";
                continue;
            }
            continue;
        }

        if let Some(inside) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            if inside.starts_with('[') {
                continue;
            }
            let key = inside.trim();
            if key == "workspace" || key.starts_with("workspace.") {
                return true;
            }
        }
    }
    false
}
