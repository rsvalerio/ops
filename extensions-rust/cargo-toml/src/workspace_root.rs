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
/// discovered root outside the user's intended logical path — and any
/// candidate whose parent cannot be canonicalized at all.
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
/// # Scope of the guarantee (TASK-1785 / TASK-2026)
///
/// The strict variant enforces **two** independent checks per candidate:
///
/// 1. The candidate directory's canonical path must lie on the canonical
///    start's ancestor chain.
/// 2. The candidate `Cargo.toml`'s **own** canonical path must sit directly
///    inside that canonical directory — i.e. the manifest must not be a
///    symlink (or reachable through one) that redirects the read into
///    another tree.
///
/// TASK-2026 recorded the decision behind check 2. Check 1 alone is a
/// tautology on a quiescent filesystem: the shared walk canonicalizes
/// `start` once and reaches every ancestor via [`Path::parent`], and every
/// lexical ancestor of a canonical path is itself canonical, so
/// `canonicalize(current) == current` and the ancestor-chain test can only
/// fail if an ancestor is swapped for a symlink *during* the walk (a TOCTOU
/// race no test can construct deterministically). Check 2 is the arm that
/// actually rejects an attacker-plantable manifest on a quiescent
/// filesystem, and it is driven by
/// `find_root_strict_rejects_symlinked_manifest_that_lenient_accepts`.
///
/// The alternative — re-anchoring discovery to the caller's *pre-canonical*
/// `start` — was rejected: it would reject every legitimate working
/// directory reached through a symlink, which is a routine layout. Two
/// consequences of that choice callers must not mistake for a stronger
/// guarantee:
///
/// - A symlink *inside the caller's own `start` path* is resolved before
///   the walk begins, so the strict variant walks the resolved chain and
///   accepts what it finds there — exactly like the lenient variant. It
///   does **not** re-anchor discovery to the caller's pre-canonical,
///   logical path.
/// - A symlinked *directory* ancestor above the start is likewise already
///   resolved; only a symlinked `Cargo.toml` (check 2) and a mid-walk
///   ancestor swap (check 1) are rejected.
///
/// The per-candidate decision is factored into [`strict_candidate_action`]
/// so every rejection arm is directly testable without racing the
/// filesystem.
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
/// contain `start` after canonicalization, and any candidate whose
/// `Cargo.toml` resolves outside that directory (symlink-planting defence,
/// TASK-2026).
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
/// contain `start` after canonicalization, and any candidate whose
/// `Cargo.toml` resolves outside that directory (symlink-planting defence,
/// TASK-2026).
pub fn find_workspace_root_strict_with_depth(
    start: &Path,
    max_depth: usize,
) -> Result<PathBuf, FindWorkspaceRootError> {
    walk_ancestors(start, max_depth, |current, cargo_toml, start_canonical| {
        strict_candidate_action(current, &cargo_toml, start_canonical, &|p| {
            fs::canonicalize(p)
        })
    })
}

/// The strict variant's per-candidate decision, with the canonicalizer
/// injected.
///
/// TASK-1785: the directory rejection arms — an off-chain canonical parent
/// and a failed canonicalize — are unreachable through a quiescent
/// filesystem (see "Scope of the guarantee" on
/// [`find_workspace_root_strict`]), so they were previously untested despite
/// being the entire reason the strict variant exists. Taking `canonicalize`
/// as a parameter lets tests drive them deterministically instead of racing
/// a symlink swap.
///
/// TASK-2026: adds the manifest-path check that *is* reachable on a
/// quiescent filesystem — a `Cargo.toml` whose own canonical path does not
/// sit directly inside the canonical candidate directory is a planted
/// symlink into another tree and is skipped.
pub fn strict_candidate_action(
    current: &Path,
    cargo_toml: &Path,
    start_canonical: &Path,
    canonicalize: &dyn Fn(&Path) -> std::io::Result<PathBuf>,
) -> CandidateAction {
    match canonicalize(current) {
        Ok(canonical_parent) => {
            if start_canonical.starts_with(&canonical_parent) {
                if !manifest_is_contained(cargo_toml, &canonical_parent, canonicalize) {
                    return CandidateAction::Skip;
                }
                if manifest_declares_workspace(cargo_toml) {
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
    }
}

/// SEC-25 / TASK-2026: true iff the candidate `Cargo.toml` resolves to a
/// file that lives directly inside `canonical_parent`.
///
/// Unlike the ancestor-chain check this is *not* a tautology: the walk only
/// ever canonicalizes directories, so a `Cargo.toml` that is itself a
/// symlink into an attacker tree is read through that symlink and its
/// `[workspace]` table is trusted. Resolving the manifest path and demanding
/// its canonical parent equal the canonical candidate directory rejects that
/// planted manifest without affecting any ordinary one.
///
/// A canonicalize failure is treated as "not contained": the walk has
/// already established the file exists, so a failure here means the path
/// changed underneath us or is unreadable, neither of which should be
/// trusted as a workspace root.
fn manifest_is_contained(
    cargo_toml: &Path,
    canonical_parent: &Path,
    canonicalize: &dyn Fn(&Path) -> std::io::Result<PathBuf>,
) -> bool {
    let canonical_manifest = match canonicalize(cargo_toml) {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(
                path = ?cargo_toml.display(),
                error = ?e,
                "SEC-25 / TASK-2026: failed to canonicalize candidate Cargo.toml; skipping ancestor"
            );
            return false;
        }
    };
    if canonical_manifest.parent() == Some(canonical_parent) {
        return true;
    }
    tracing::warn!(
        cargo_toml = ?cargo_toml.display(),
        canonical_manifest = ?canonical_manifest.display(),
        canonical_parent = ?canonical_parent.display(),
        "SEC-25 / TASK-2026: candidate Cargo.toml resolves outside its own directory (planted symlink); rejecting"
    );
    false
}

/// Action returned by the per-candidate check closure in [`walk_ancestors`].
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub enum CandidateAction {
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
/// header. A missing manifest returns false — the walk will keep looking and
/// ultimately fall back to the first Cargo.toml seen.
///
/// SEC-11 / TASK-1781: any *other* read failure (permission denied, the
/// `read_capped_to_string` byte cap, a non-UTF-8 manifest) is logged at
/// `warn` level rather than `debug`, so a legitimately large or unreadable
/// workspace root that gets silently skipped is visible in the default log
/// output instead of being indistinguishable from "no workspace declared".
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
            tracing::warn!(
                path = ?path.display(),
                error = ?e,
                "SEC-11 / TASK-1781: Cargo.toml unreadable during workspace walk (not NotFound); treating as 'no workspace declared' and continuing to climb"
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
///
/// SEC-11 / TASK-1781: the header is located by scanning for the closing `]`
/// that is not inside a quoted key, so a trailing comment
/// (`[workspace] # the root`) no longer defeats the match, and the first
/// dotted key segment is unquoted before comparison so `["workspace"]` and
/// `[ 'workspace'.package ]` are recognised too. A false negative here is
/// security-relevant: the walk simply keeps climbing past the real root and
/// into attacker-plantable ancestors (see the threat model on
/// [`find_workspace_root`]).
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

        if let Some(inside) = table_header_body(trimmed) {
            // `[[array.of.tables]]` — not a plain table header.
            if inside.starts_with('[') {
                continue;
            }
            if first_key_is_workspace(inside) {
                return true;
            }
        }
    }
    false
}

/// Given a trimmed line that starts with `[`, return the text between that
/// bracket and the first `]` that lies outside a quoted key, or `None` when
/// the line has no closing bracket.
///
/// Anything after the closing bracket (whitespace, a `# comment`) is ignored,
/// which is what makes `[workspace] # root` match.
fn table_header_body(trimmed: &str) -> Option<&str> {
    let rest = trimmed.strip_prefix('[')?;
    let mut in_basic = false;
    let mut in_literal = false;
    let mut escaped = false;
    for (i, c) in rest.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        match c {
            // Inside a basic string, `\` escapes the next character (notably `\"`).
            '\\' if in_basic => escaped = true,
            '"' if !in_literal => in_basic = !in_basic,
            '\'' if !in_basic => in_literal = !in_literal,
            ']' if !in_basic && !in_literal => return rest.get(..i),
            _ => {}
        }
    }
    None
}

/// True iff the first dotted key of a table header is `workspace`, ignoring
/// surrounding whitespace and basic/literal quoting.
fn first_key_is_workspace(inside: &str) -> bool {
    let s = inside.trim_start();
    let mut chars = s.chars();
    let Some(quote) = chars.next().filter(|c| *c == '"' || *c == '\'') else {
        // Bare key: runs to the first `.` (or the end of the header).
        let key = s.split('.').next().unwrap_or(s).trim_end();
        return key == "workspace";
    };
    // Quoted key: everything up to the matching close quote. Escapes only
    // apply to basic (double-quoted) strings. `chars` is already positioned
    // just past the opening quote.
    let mut key = String::new();
    while let Some(c) = chars.next() {
        if c == '\\' && quote == '"' {
            match chars.next() {
                Some(escaped) => key.push(escaped),
                None => return false,
            }
        } else if c == quote {
            return key == "workspace";
        } else {
            key.push(c);
        }
    }
    false
}
