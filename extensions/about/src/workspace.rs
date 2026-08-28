//! Shared `[workspace].members` glob expansion for stack `project_units`
//! providers (Node, Python, ...).
//!
//! Returns `(member_path, manifest_contents)` tuples so callers do not need
//! to re-open the manifest. The single read avoids the SEC-25 TOCTOU window
//! where a symlink swap between an `exists()` probe and a later open could
//! redirect the read.
//!
//! Pattern shape supported is the simple `prefix/*` case Cargo / yarn / npm /
//! uv all use in practice. Multi-segment globs (`**`, `prefix/*/suffix`) are
//! **not supported** and are skipped with a `tracing::warn` per
//! TASK-1069 — the previous behaviour silently flattened them onto the
//! prefix, producing either a brute-force scan of the workspace root
//! (`**/foo`) or dropped patterns (`prefix/*/suffix`) with no breadcrumb.
//! Exclusion patterns follow the same single-`*`-per-segment shape and
//! filter the resolved list (TASK-0389 / TASK-0400).

use std::path::{Component, Path};

use crate::manifest_io::read_optional_text;

/// Resolve workspace member globs against `root`, looking for `marker` (e.g.
/// `"package.json"`, `"pyproject.toml"`) inside each candidate directory.
///
/// Excludes are matched with the same single-`*`-per-pattern glob shape and
/// applied after expansion.
///
/// PATTERN-1 (TASK-1052): an exclude pattern with more than one `*` is
/// unsupported and fails **closed** — the candidate is dropped (treated as
/// matching) and a `tracing::warn` is emitted, rather than the previous
/// fail-open behaviour that silently let the unit through.
///
/// FN-1 / TASK-1743: the body used to inline five concerns — member-value
/// validation, glob-shape validation, directory enumeration with per-entry
/// error classification, relative-path derivation, and post-processing —
/// across 136 lines nested six deep under a bare
/// `#[allow(clippy::too_many_lines)]`. Each concern now has a name; what is
/// left here is the orchestration: validate the member, expand it, filter the
/// excludes, sort and dedup.
pub fn resolve_member_globs(
    members: &[String],
    excludes: &[String],
    root: &Path,
    marker: &str,
) -> Vec<(String, String)> {
    let mut resolver = Resolver::new(root, marker);
    let mut resolved: Vec<(String, String)> = Vec::new();
    for member in members {
        if let Some(escape) = member_escape(member) {
            tracing::warn!(
                member,
                escape = escape.reason(),
                "workspace member escapes the workspace root; skipping"
            );
            continue;
        }
        match classify_member_pattern(member) {
            MemberPattern::Unsupported => tracing::warn!(
                member,
                "workspace member glob shape unsupported (only a whole-segment trailing `*`, e.g. `packages/*`); skipping"
            ),
            MemberPattern::SegmentGlob(prefix) => {
                resolved.extend(resolver.expand_segment_glob(member, prefix));
            }
            MemberPattern::Literal(literal) => {
                if let Some(manifest) = try_read_manifest(&root.join(literal), marker) {
                    resolved.push((literal.to_string(), manifest));
                }
            }
        }
    }
    if !excludes.is_empty() {
        resolved.retain(|(m, _)| !excludes.iter().any(|pat| matches_exclude(pat, m)));
    }
    resolved.sort_by(|a, b| a.0.cmp(&b.0));
    resolved.dedup_by(|a, b| a.0 == b.0);
    resolved
}

/// How a `[workspace].members` value escapes the workspace root.
///
/// See [`member_escape`] for the containment invariant these represent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberEscape {
    /// A `..` component walks out of the root.
    ParentTraversal,
    /// An absolute (or drive-prefixed) path. `Path::join` *discards* the base
    /// when the joined path is absolute, so the root is not merely escaped —
    /// it is ignored entirely.
    Absolute,
}

impl MemberEscape {
    /// Stable log token naming the escape, for the warn at the call site.
    const fn reason(self) -> &'static str {
        match self {
            Self::ParentTraversal => "parent-traversal",
            Self::Absolute => "absolute-path",
        }
    }
}

/// Whether `member` breaks the containment invariant, and how.
///
/// **Containment invariant**: `root.join(member)` resolves inside `root` only
/// when `member` is *relative* **and** free of `..` components. Both halves
/// are enforced here, before any I/O:
///
/// - **`..`** (PATTERN-1 / TASK-1071): `root.join("../sibling")` walks out of
///   the workspace root. Aligns with the SEC-13 dot-only-segment work in
///   `git/src/remote.rs`.
/// - **absolute** (SEC-14 / TASK-1726): an absolute member has no
///   `ParentDir` component — `/etc/foo` decomposes to `RootDir, Normal,
///   Normal` — so the `..` check alone let it through, and
///   `root.join("/etc/foo")` is `/etc/foo`, discarding `root`. That made the
///   *simpler* of the two escapes the open one: a checked-in manifest could
///   direct reads anywhere on the filesystem and echo the contents into
///   rendered `about` output. Windows drive prefixes (`C:\…`, and the
///   drive-relative `C:foo`) are rejected for the same reason.
///
/// Workspace config is operator-authored, so the impact is bounded — but the
/// guard exists precisely so a reviewer can conclude members cannot escape
/// `root`, and that conclusion has to be true.
///
/// Pure and filesystem-free: it inspects the string's path components only.
fn member_escape(member: &str) -> Option<MemberEscape> {
    Path::new(member).components().find_map(|c| match c {
        Component::ParentDir => Some(MemberEscape::ParentTraversal),
        Component::RootDir | Component::Prefix(_) => Some(MemberEscape::Absolute),
        Component::CurDir | Component::Normal(_) => None,
    })
}

/// The glob shape of a member value, once [`member_escape`] has cleared it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MemberPattern<'a> {
    /// No `*` at all: a literal member directory, joined straight onto the
    /// root.
    Literal(&'a str),
    /// A supported whole-segment trailing `*`. The payload is the directory
    /// prefix to enumerate — `""` for a bare `*`, `"packages/"` for
    /// `packages/*`.
    SegmentGlob(&'a str),
    /// Carries a `*` in a position this module does not support.
    Unsupported,
}

/// Classify the glob shape of `member`.
///
/// PATTERN-1 (TASK-1069): the original implementation found the first `*` and
/// treated everything before it as the prefix, silently ignoring any suffix.
/// That made `**/foo` (prefix `""`) brute-force-scan the workspace root and
/// flattened `prefix/*/suffix` to `prefix/*`.
///
/// PATTERN-1 (TASK-1069 follow-up): the `*` must span a *whole* path segment
/// (`*`, `packages/*`). The earlier check only rejected a suffix containing
/// `/`, so a partial-segment pattern like `packages/*-internal` or
/// `packages/foo*` passed and was then expanded as a bare `read_dir(prefix)`
/// — the text around the `*` was silently dropped and every sibling directory
/// matched.
///
/// Pure and filesystem-free, so the shape rules are unit-testable on their own.
fn classify_member_pattern(member: &str) -> MemberPattern<'_> {
    let Some((prefix, suffix)) = member.split_once('*') else {
        return MemberPattern::Literal(member);
    };
    let is_recursive = member.contains("**");
    let is_full_segment = suffix.is_empty() && (prefix.is_empty() || prefix.ends_with('/'));
    if is_recursive || !is_full_segment {
        return MemberPattern::Unsupported;
    }
    MemberPattern::SegmentGlob(prefix)
}

/// Per-resolution filesystem state, so the expansion helpers can share the
/// root, the marker filename, and the memoised canonical root without
/// threading five parameters through each other.
struct Resolver<'a> {
    root: &'a Path,
    marker: &'a str,
    /// PERF-3 / TASK-1149: lazily canonicalize `root` once across the whole
    /// resolution. The recovery path (run when `strip_prefix(root)` misses,
    /// e.g. macOS `/var` ↔ `/private/var` or any symlinked workspace root)
    /// re-canonicalised the *same* root for every directory entry, turning a
    /// one-shot fallback into O(N) syscalls on monorepos with hundreds of
    /// members.
    ///
    root_canonical: RootCanonical,
}

/// Memo state for the canonicalised workspace root (PERF-3 / TASK-1149).
///
/// A named enum rather than `Option<Option<PathBuf>>`: all three states are
/// meaningful and the nested-option spelling made which `None` meant what a
/// matter of counting layers.
enum RootCanonical {
    /// Not attempted — no `strip_prefix` has missed yet, so the syscall has
    /// not been paid for.
    Unattempted,
    /// `canonicalize(root)` succeeded.
    Resolved(std::path::PathBuf),
    /// `canonicalize(root)` failed. The strip cannot succeed without a valid
    /// canonical root, so recovery goes straight to the absolute-path
    /// fallback. Cached so the failing syscall is not repeated per entry.
    Failed,
}

impl<'a> Resolver<'a> {
    const fn new(root: &'a Path, marker: &'a str) -> Self {
        Self {
            root,
            marker,
            root_canonical: RootCanonical::Unattempted,
        }
    }

    /// Enumerate `<root>/<prefix>` and collect a
    /// `(relative_path, manifest_contents)` pair for every child directory
    /// holding `marker`.
    fn expand_segment_glob(&mut self, member: &str, prefix: &str) -> Vec<(String, String)> {
        let parent = self.root.join(prefix);
        let Some(entries) = open_glob_parent(member, &parent) else {
            return Vec::new();
        };
        let mut resolved = Vec::new();
        for entry in entries {
            let Some(path) = glob_child_dir(member, &parent, entry) else {
                continue;
            };
            let Some(manifest) = try_read_manifest(&path, self.marker) else {
                continue;
            };
            resolved.push((self.relative_path(&path), manifest));
        }
        resolved
    }

    /// Render `path` relative to the workspace root.
    ///
    /// ERR-1 (TASK-1070): a `strip_prefix` failure here used to silently drop
    /// a successfully-read manifest — typically when `root` and
    /// `entry.path()` disagree on symlink resolution (common on macOS via
    /// `/var` vs `/private/var`). Canonicalise both sides as a fallback, log
    /// a breadcrumb either way, and fall back to the absolute path so the
    /// unit is not silently lost.
    fn relative_path(&mut self, path: &Path) -> String {
        let root = self.root;
        if let Ok(rel) = path.strip_prefix(root) {
            return rel.to_string_lossy().to_string();
        }
        if matches!(self.root_canonical, RootCanonical::Unattempted) {
            self.root_canonical =
                std::fs::canonicalize(root).map_or(RootCanonical::Failed, RootCanonical::Resolved);
        }
        let root_canon = match &self.root_canonical {
            RootCanonical::Resolved(canonical) => Some(canonical.as_path()),
            RootCanonical::Unattempted | RootCanonical::Failed => None,
        };
        recover_relative_path(path, root, root_canon)
    }
}

/// Open the directory a segment glob expands over.
///
/// ERR-1 (TASK-0517): a `read_dir` error here used to silently produce "No
/// project units found". Log at warn so a permissions or missing-prefix issue
/// is visible, without changing the best-effort behaviour that lets the rest
/// of the globs resolve. A missing prefix is routine (an optional
/// `packages/` directory) and stays at debug.
fn open_glob_parent(member: &str, parent: &Path) -> Option<std::fs::ReadDir> {
    match std::fs::read_dir(parent) {
        Ok(entries) => Some(entries),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // ERR-7 (TASK-0665): Debug-format the path so embedded newlines /
            // ANSI escapes cannot forge log lines.
            tracing::debug!(
                member,
                parent = ?parent.display(),
                "workspace glob prefix does not exist; skipping"
            );
            None
        }
        Err(e) => {
            tracing::warn!(
                member,
                parent = ?parent.display(),
                error = ?e,
                "workspace glob prefix unreadable; member skipped"
            );
            None
        }
    }
}

/// Accept one `read_dir` item as a candidate member directory.
///
/// ERR-1 (TASK-0942): the per-entry `Result` is matched explicitly rather
/// than `flatten()`ed, so an IO error on one entry (EACCES on a sibling
/// member, EIO, ...) is visible at warn level instead of disappearing into
/// "no project units found". Mirrors the policy `open_glob_parent` adopted in
/// TASK-0517. Non-directories are skipped silently — they are ordinary files
/// sitting beside the members, not an error.
fn glob_child_dir(
    member: &str,
    parent: &Path,
    entry: std::io::Result<std::fs::DirEntry>,
) -> Option<std::path::PathBuf> {
    let entry = match entry {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!(
                member,
                parent = ?parent.display(),
                error = ?e,
                "workspace glob entry unreadable; skipping"
            );
            return None;
        }
    };
    let path = entry.path();
    path.is_dir().then_some(path)
}

/// ERR-1 (TASK-1070): recover a workspace-relative path when
/// `path.strip_prefix(root)` misses, which happens when `root` and the entry
/// path disagree on symlink resolution (commonly macOS `/var` vs
/// `/private/var`). Falls back to the absolute path so a
/// successfully-read manifest is never silently dropped.
///
/// `root_canon` is the caller's memoised canonicalised root (PERF-3 /
/// TASK-1149), or `None` when canonicalising the root itself failed — the
/// strip cannot succeed without it, so recovery goes straight to the fallback.
///
/// Split out of the loop body so the caller's `map_or_else` keeps the common
/// `strip_prefix` success on one line instead of trailing 25 lines of recovery.
fn recover_relative_path(path: &Path, root: &Path, root_canon: Option<&Path>) -> String {
    let canonical_rel = root_canon.and_then(|root_canon| {
        std::fs::canonicalize(path).ok().and_then(|p_canon| {
            p_canon
                .strip_prefix(root_canon)
                .ok()
                .map(|r| r.to_string_lossy().to_string())
        })
    });
    canonical_rel.map_or_else(
        || {
            tracing::warn!(
                root = ?root.display(),
                path = ?path.display(),
                "workspace strip_prefix failed and canonicalize did not recover; falling back to absolute path so manifest is not silently dropped"
            );
            path.to_string_lossy().to_string()
        },
        |rel| {
            tracing::debug!(
                root = ?root.display(),
                path = ?path.display(),
                "workspace strip_prefix failed; recovered via canonicalize"
            );
            rel
        },
    )
}

fn try_read_manifest(dir: &Path, marker: &str) -> Option<String> {
    let path = dir.join(marker);
    read_optional_text(&path, marker)
}

/// PATTERN-1 (TASK-0503): exclude patterns now support a single `*` anywhere
/// in the final path segment — `prefix*`, `*suffix`, `prefix*suffix`, and
/// bare `*`. The `*` matches any non-empty run of characters that does not
/// cross a `/`, mirroring Cargo / yarn / npm single-segment glob semantics.
///
/// PATTERN-1 (TASK-1052): multi-`*` patterns are unsupported and now fail
/// **closed** — the candidate is treated as matching (i.e. excluded) so a
/// typo like `packages/*-internal-*` does not silently leak the unit into
/// published output. A `tracing::warn` is still emitted so operators can
/// see and fix the pattern; the fail-closed default is the safer wrong
/// answer (over-restrictive) versus the previous fail-open behaviour
/// (under-restrictive) that shipped private modules until someone noticed.
fn matches_exclude(pattern: &str, candidate: &str) -> bool {
    let star_count = pattern.bytes().filter(|b| *b == b'*').count();
    if star_count == 0 {
        return pattern == candidate;
    }
    if star_count > 1 {
        tracing::warn!(
            pattern,
            "workspace exclude pattern has more than one `*`; not supported, treating as match (fail-closed)"
        );
        return true;
    }
    let Some((prefix, suffix)) = pattern.split_once('*') else {
        // Unreachable: `star_count == 0` returned above. Falling back to the
        // exact-match rule keeps the helper total instead of panicking.
        return pattern == candidate;
    };
    let Some(rest) = candidate.strip_prefix(prefix) else {
        return false;
    };
    let Some(middle) = rest.strip_suffix(suffix) else {
        return false;
    };
    !middle.is_empty() && !middle.contains('/')
}

/// Manifest-level identity fields surfaced by the units providers.
///
/// Replaces the old positional `(Option<String>, Option<String>,
/// Option<String>)` so argument-order errors at call sites become compile
/// errors.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PackageMetadata {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
}

/// DUP-3 (TASK-0620): shared `(name, version, description)` projection
/// shared by Node `package.json` and Python `pyproject.toml` units providers.
///
/// Calls `parse` to produce the raw fields from the manifest contents. On
/// parser error, logs at warn with the manifest path and returns
/// `PackageMetadata::default()` — matching the swallow-and-warn shape
/// established by TASK-0440. Description is trimmed and empty values are
/// filtered out.
pub fn parse_package_metadata<E, F>(path: &Path, content: &str, parse: F) -> PackageMetadata
where
    E: std::fmt::Display + std::fmt::Debug,
    F: FnOnce(&str) -> Result<PackageMetadata, E>,
{
    match parse(content) {
        Ok(meta) => PackageMetadata {
            name: meta.name,
            version: meta.version,
            description: meta
                .description
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        },
        Err(e) => {
            tracing::warn!(
                path = ?path.display(),
                error = ?e,
                "failed to parse package manifest",
            );
            PackageMetadata::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &std::path::Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn expands_simple_glob_with_marker() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("packages/a/package.json"),
            r#"{"name":"a"}"#,
        );
        write(
            &dir.path().join("packages/b/package.json"),
            r#"{"name":"b"}"#,
        );
        write(&dir.path().join("packages/no-pkg/README.md"), "");

        let resolved =
            resolve_member_globs(&["packages/*".to_string()], &[], dir.path(), "package.json");
        let names: Vec<&str> = resolved.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(names, vec!["packages/a", "packages/b"]);
    }

    /// PATTERN-1 (TASK-1069 follow-up): a partial-segment wildcard is not a
    /// supported shape. It used to pass validation and then expand as a bare
    /// `read_dir("packages")`, so `packages/*-internal` silently matched every
    /// sibling — including `packages/other`, which does not end in `-internal`.
    /// Rejecting the pattern outright is what keeps the sibling out.
    #[test]
    fn rejects_partial_segment_glob_instead_of_matching_every_sibling() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("packages/foo-internal/package.json"),
            r#"{"name":"foo-internal"}"#,
        );
        write(
            &dir.path().join("packages/other/package.json"),
            r#"{"name":"other"}"#,
        );

        let resolved = resolve_member_globs(
            &["packages/*-internal".to_string()],
            &[],
            dir.path(),
            "package.json",
        );
        let names: Vec<&str> = resolved.iter().map(|(p, _)| p.as_str()).collect();
        assert!(
            !names.contains(&"packages/other"),
            "non-matching sibling must not leak in: {names:?}"
        );
        assert!(
            names.is_empty(),
            "unsupported glob shape must resolve to nothing: {names:?}"
        );
    }

    /// The trailing-`*`-inside-a-segment shape (`packages/foo*`) is rejected
    /// for the same reason: it used to `read_dir("packages/foo")`, a directory
    /// that generally does not exist, so it silently resolved to nothing while
    /// looking like a working prefix filter.
    #[test]
    fn rejects_prefix_glob_inside_a_segment() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("packages/foobar/package.json"),
            r#"{"name":"foobar"}"#,
        );

        write(
            &dir.path().join("packages/foo/nested/package.json"),
            r#"{"name":"nested"}"#,
        );

        let resolved = resolve_member_globs(
            &["packages/foo*".to_string()],
            &[],
            dir.path(),
            "package.json",
        );
        let names: Vec<&str> = resolved.iter().map(|(p, _)| p.as_str()).collect();
        assert!(
            !names.contains(&"packages/foo/nested"),
            "must not enumerate children of the literal prefix: {names:?}"
        );
        assert!(names.is_empty(), "partial-segment glob must be rejected");
    }

    #[test]
    fn passthrough_non_glob_member_with_manifest() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("services/api/pyproject.toml"),
            "[project]\nname=\"api\"\n",
        );

        let resolved = resolve_member_globs(
            &["services/api".to_string()],
            &[],
            dir.path(),
            "pyproject.toml",
        );
        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].0, "services/api");
    }

    #[test]
    fn excludes_filter_resolved_members() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["keep", "internal-tools"] {
            write(
                &dir.path().join(format!("packages/{name}/package.json")),
                r#"{"name":"x"}"#,
            );
        }

        let resolved = resolve_member_globs(
            &["packages/*".to_string()],
            &["packages/internal-*".to_string()],
            dir.path(),
            "package.json",
        );
        let names: Vec<&str> = resolved.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(names, vec!["packages/keep"]);
    }

    /// ERR-1 (TASK-0517): an unreadable glob-prefix directory must not
    /// crash; resolution returns empty for that member while other globs
    /// still resolve normally. The accompanying `tracing::warn` is exercised
    /// by the `read_dir` failure path; pinning the value-level contract here
    /// keeps the test free of a tracing-subscriber dev-dep.
    #[cfg(unix)]
    #[test]
    fn unreadable_glob_prefix_yields_no_panic_and_empty() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let parent = dir.path().join("packages");
        std::fs::create_dir(&parent).unwrap();
        // Drop read permissions so read_dir fails with PermissionDenied.
        let mut perms = std::fs::metadata(&parent).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&parent, perms).unwrap();

        let resolved =
            resolve_member_globs(&["packages/*".to_string()], &[], dir.path(), "package.json");

        // Restore so tempdir cleanup works.
        let mut restore = std::fs::metadata(&parent).unwrap().permissions();
        restore.set_mode(0o755);
        std::fs::set_permissions(&parent, restore).unwrap();

        assert!(resolved.is_empty());
    }

    /// ERR-1: an unreadable manifest (permission denied) must drop the unit
    /// out of the resolved listing. The previous `.ok()` shape coerced every
    /// IO failure to `NotFound`, silently producing "no project units".
    #[cfg(unix)]
    #[test]
    fn unreadable_manifest_is_skipped_not_silent() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("services/api/pyproject.toml");
        write(&manifest, "[project]\nname=\"api\"\n");
        // Drop read permissions on the manifest itself so read_to_string
        // fails with PermissionDenied (not NotFound).
        let mut perms = std::fs::metadata(&manifest).unwrap().permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&manifest, perms).unwrap();

        let resolved = resolve_member_globs(
            &["services/api".to_string()],
            &[],
            dir.path(),
            "pyproject.toml",
        );

        // Restore so tempdir cleanup works.
        let mut restore = std::fs::metadata(&manifest).unwrap().permissions();
        restore.set_mode(0o644);
        std::fs::set_permissions(&manifest, restore).unwrap();

        assert!(
            resolved.is_empty(),
            "unreadable manifest must be skipped (not falsely included)"
        );
    }

    /// PERF-3 / TASK-1149: when the workspace root is reached through a
    /// symlink, `path.strip_prefix(root)` misses for every entry and the
    /// recovery path canonicalises both sides. The root canonicalisation
    /// is hoisted out of the per-entry loop and cached on first miss, so
    /// a many-entry tree pays one root canonicalize, not N.
    ///
    /// We can't observe syscall counts portably; instead we exercise a
    /// 200-entry symlinked-root tree and assert every member resolves
    /// without the recovery path silently dropping units. Combined with
    /// the structural lazy-init in `resolve_member_globs`, this pins the
    /// behaviour the AC asks for.
    #[cfg(unix)]
    #[test]
    fn symlinked_root_with_many_entries_resolves_via_cached_canonicalize() {
        let dir = tempfile::tempdir().unwrap();
        let real_root = dir.path().join("real");
        std::fs::create_dir(&real_root).unwrap();
        for i in 0..200 {
            write(
                &real_root.join(format!("packages/m{i:03}/package.json")),
                r#"{"name":"x"}"#,
            );
        }
        let link_root = dir.path().join("link");
        std::os::unix::fs::symlink(&real_root, &link_root).unwrap();

        let resolved = resolve_member_globs(
            &["packages/*".to_string()],
            &[],
            // Pass the symlinked root so strip_prefix misses on each entry
            // (entry.path() resolves through canonicalised parent on some
            // platforms) and the recovery path is exercised.
            &link_root,
            "package.json",
        );
        // 200 members must all resolve; the recovery path's cached
        // canonicalize must not lose entries.
        assert_eq!(
            resolved.len(),
            200,
            "all 200 symlinked-root entries should resolve"
        );
    }

    /// PATTERN-1 (TASK-0503): `prefix*suffix` excludes match a single
    /// non-`/`-spanning segment middle.
    #[test]
    fn prefix_star_suffix_exclude_matches_single_segment() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["internal-x-tool", "internal-y-tool", "keep"] {
            write(
                &dir.path().join(format!("packages/{name}/package.json")),
                r#"{"name":"x"}"#,
            );
        }

        let resolved = resolve_member_globs(
            &["packages/*".to_string()],
            &["packages/internal-*-tool".to_string()],
            dir.path(),
            "package.json",
        );
        let names: Vec<&str> = resolved.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(names, vec!["packages/keep"]);
    }

    /// PATTERN-1 (TASK-0503): bare `*` only matches a single path segment, so
    /// nested multi-segment members are left in place.
    #[test]
    fn bare_star_exclude_only_matches_single_segment() {
        assert!(matches_exclude("*", "foo"));
        assert!(!matches_exclude("*", "packages/foo"));
        assert!(!matches_exclude("*", ""));
    }

    /// PATTERN-1 (TASK-1052): multi-`*` patterns are explicitly unsupported
    /// and now fail **closed** — `matches_exclude` returns true so the
    /// candidate is dropped rather than silently leaked. The accompanying
    /// `tracing::warn` is exercised but not asserted here to avoid pulling
    /// in a tracing-subscriber dev-dep just for this case.
    #[test]
    fn multi_star_exclude_fails_closed() {
        assert!(matches_exclude("a/*/b/*", "a/x/b/y"));
        assert!(matches_exclude("packages/*-internal-*", "packages/foo"));
    }

    /// PATTERN-1 (TASK-1052): end-to-end — a multi-`*` exclude pattern must
    /// drop the matching candidate from `resolve_member_globs` rather than
    /// fail open and ship it. Mirrors the typo case `packages/*-internal-*`
    /// from the task description.
    #[test]
    fn multi_star_exclude_drops_candidate_in_resolve() {
        let dir = tempfile::tempdir().unwrap();
        for name in ["keep", "x-internal-y"] {
            write(
                &dir.path().join(format!("packages/{name}/package.json")),
                r#"{"name":"x"}"#,
            );
        }

        let resolved = resolve_member_globs(
            &["packages/*".to_string()],
            &["packages/*-internal-*".to_string()],
            dir.path(),
            "package.json",
        );
        let names: Vec<&str> = resolved.iter().map(|(p, _)| p.as_str()).collect();
        // Fail-closed: the multi-`*` pattern matches everything, so even
        // `keep` is dropped — the typo is loud rather than silent.
        assert!(
            names.is_empty(),
            "multi-`*` exclude must fail closed (drop candidates), got {names:?}"
        );
    }

    /// PATTERN-1 (TASK-1071): a non-glob member value containing `..` must be
    /// rejected before any I/O — `root.join("../sibling")` would otherwise
    /// escape the workspace root. The valid sibling member `packages/foo`
    /// continues to resolve, confirming the check only fires on `ParentDir`
    /// components. The accompanying `tracing::warn` is exercised but not
    /// asserted here to avoid pulling in a tracing-subscriber dev-dep.
    #[test]
    fn parent_dir_member_is_rejected_sibling_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        // Create a "sibling" manifest one level above `root` that a `..`
        // traversal would otherwise reach.
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();
        write(
            &dir.path().join("sibling/package.json"),
            r#"{"name":"escape"}"#,
        );
        // Valid in-root member must still load.
        write(&root.join("packages/foo/package.json"), r#"{"name":"foo"}"#);

        let resolved = resolve_member_globs(
            &["../sibling".to_string(), "packages/foo".to_string()],
            &[],
            &root,
            "package.json",
        );
        let names: Vec<&str> = resolved.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            names,
            vec!["packages/foo"],
            "`..` traversal must be rejected; sibling member must still load"
        );
    }

    /// SEC-14 / TASK-1726: an **absolute** member decomposes to `RootDir,
    /// Normal, …` with no `ParentDir` component, so the `..` guard alone let
    /// it through — and `root.join("/abs/path")` discards `root` entirely.
    /// Pin that an absolute member resolves to nothing while a valid relative
    /// sibling in the same call still loads.
    #[test]
    fn absolute_member_is_rejected_sibling_still_loads() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        // A tempdir sibling of `root`, holding a valid marker file, that an
        // absolute member would otherwise read straight out of.
        let outside = dir.path().join("outside");
        write(&outside.join("package.json"), r#"{"name":"escape"}"#);
        assert!(outside.is_absolute(), "the escape target must be absolute");

        // Valid in-root member must still load.
        write(&root.join("packages/foo/package.json"), r#"{"name":"foo"}"#);

        let resolved = resolve_member_globs(
            &[
                outside.to_string_lossy().into_owned(),
                "packages/foo".to_string(),
            ],
            &[],
            &root,
            "package.json",
        );
        let names: Vec<&str> = resolved.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            names,
            vec!["packages/foo"],
            "an absolute member must be rejected; the relative sibling must still load"
        );
    }

    /// SEC-14 / TASK-1726: the glob branch joins `root.join(prefix)`, so it
    /// has the same absolute-path hole. An absolute glob must not enumerate
    /// the directory it names.
    #[test]
    fn absolute_glob_member_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("root");
        std::fs::create_dir_all(&root).unwrap();

        let outside = dir.path().join("outside");
        write(&outside.join("secret/package.json"), r#"{"name":"escape"}"#);

        let pattern = format!("{}/*", outside.to_string_lossy());
        let resolved = resolve_member_globs(&[pattern], &[], &root, "package.json");
        assert!(
            resolved.is_empty(),
            "an absolute glob must not enumerate outside the root, got {resolved:?}"
        );
    }

    /// FN-1 / TASK-1743: member-value validation is now a pure predicate, so
    /// the containment invariant is testable without touching the filesystem.
    #[test]
    fn member_escape_classifies_both_escapes() {
        // Relative, no `..`: contained.
        assert_eq!(member_escape("packages/foo"), None);
        assert_eq!(member_escape("packages/*"), None);
        assert_eq!(member_escape("./packages/foo"), None);
        assert_eq!(member_escape(""), None);

        // `..` anywhere in the value walks out of the root.
        assert_eq!(member_escape(".."), Some(MemberEscape::ParentTraversal));
        assert_eq!(
            member_escape("../sibling"),
            Some(MemberEscape::ParentTraversal)
        );
        assert_eq!(
            member_escape("packages/../../etc"),
            Some(MemberEscape::ParentTraversal)
        );

        // Absolute: `Path::join` would discard the root outright.
        assert_eq!(member_escape("/etc"), Some(MemberEscape::Absolute));
        assert_eq!(member_escape("/etc/foo"), Some(MemberEscape::Absolute));
        assert_eq!(member_escape("/"), Some(MemberEscape::Absolute));
    }

    /// FN-1 / TASK-1743: glob-shape validation is likewise pure, so every
    /// supported and unsupported shape is pinned without a tempdir.
    #[test]
    fn classify_member_pattern_covers_every_shape() {
        assert_eq!(
            classify_member_pattern("packages/foo"),
            MemberPattern::Literal("packages/foo")
        );
        assert_eq!(
            classify_member_pattern("packages/*"),
            MemberPattern::SegmentGlob("packages/")
        );
        // A bare `*` enumerates the root itself.
        assert_eq!(classify_member_pattern("*"), MemberPattern::SegmentGlob(""));

        // Partial-segment globs would silently drop the text around the `*`.
        assert_eq!(
            classify_member_pattern("packages/foo*"),
            MemberPattern::Unsupported
        );
        assert_eq!(
            classify_member_pattern("packages/*-internal"),
            MemberPattern::Unsupported
        );
        // Multi-segment and recursive shapes.
        assert_eq!(
            classify_member_pattern("packages/*/sub"),
            MemberPattern::Unsupported
        );
        assert_eq!(
            classify_member_pattern("**/foo"),
            MemberPattern::Unsupported
        );
        assert_eq!(classify_member_pattern("**"), MemberPattern::Unsupported);
    }

    /// PATTERN-1 (TASK-1069): non-trivial suffix-after-`*` (e.g.
    /// `prefix/*/suffix`) is now explicitly skipped rather than silently
    /// flattened onto the prefix. The valid sibling member must still load
    /// to confirm the skip is per-pattern, not whole-call.
    #[test]
    fn suffix_after_star_is_skipped_with_warning() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("packages/a/package.json"), r"{}");
        write(&dir.path().join("apps/web/package.json"), r"{}");

        let resolved = resolve_member_globs(
            &["packages/*/sub".to_string(), "apps/*".to_string()],
            &[],
            dir.path(),
            "package.json",
        );
        let names: Vec<&str> = resolved.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(
            names,
            vec!["apps/web"],
            "non-trivial suffix glob must be skipped, sibling must still load"
        );
    }

    /// PATTERN-1 (TASK-1069): a recursive `**` member must be skipped, not
    /// brute-force-scanned over the entire workspace root.
    #[test]
    fn double_star_member_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        // Populate top-level dirs that the pre-fix behaviour would have
        // brute-force enumerated when prefix collapsed to `""`.
        write(&dir.path().join("a/package.json"), r"{}");
        write(&dir.path().join("b/package.json"), r"{}");

        let resolved =
            resolve_member_globs(&["**/foo".to_string()], &[], dir.path(), "package.json");
        assert!(
            resolved.is_empty(),
            "`**/foo` must be skipped (not brute-force-enumerated), got {resolved:?}"
        );
    }

    /// ERR-1 (TASK-1070): a `strip_prefix` mismatch caused by symlinked
    /// roots must not silently drop the manifest. macOS `/var` ->
    /// `/private/var` is the canonical example: callers pass the
    /// non-canonical root and `read_dir` yields canonical entry paths.
    /// The fallback canonicalises both sides (or, failing that, uses the
    /// absolute path) so the unit survives.
    #[cfg(unix)]
    #[test]
    fn symlinked_root_does_not_drop_manifest() {
        let dir = tempfile::tempdir().unwrap();
        // Create the real workspace under `real/` and a symlink `link` to
        // it. We pass the symlinked path as `root`, but `read_dir` follows
        // the symlink and yields entries rooted at the canonical target,
        // so `entry.path().strip_prefix(symlink_root)` would otherwise fail.
        let real_root = dir.path().join("real");
        let symlink_root = dir.path().join("link");
        std::fs::create_dir_all(&real_root).unwrap();
        write(
            &real_root.join("packages/a/package.json"),
            r#"{"name":"a"}"#,
        );
        std::os::unix::fs::symlink(&real_root, &symlink_root).unwrap();

        // Read via the symlinked root *with* the symlink resolved on the
        // entries side — emulate the macOS `/var` -> `/private/var`
        // mismatch by canonicalising the parent that read_dir walks.
        // We achieve this by passing `symlink_root` directly and relying
        // on the implementation's canonicalize-fallback to recover.
        let resolved = resolve_member_globs(
            &["packages/*".to_string()],
            &[],
            &symlink_root,
            "package.json",
        );

        // Either path: a successful strip_prefix (no mismatch) or a
        // recovered relative path via the fallback. What must NOT happen
        // is the manifest being silently dropped.
        assert_eq!(
            resolved.len(),
            1,
            "symlinked root must not silently drop the resolved manifest, got {resolved:?}"
        );
        // The recovered name should still end with `packages/a` regardless
        // of whether strip_prefix succeeded or the absolute-path fallback
        // was used.
        assert!(
            resolved[0].0.ends_with("packages/a"),
            "expected resolved name to end with `packages/a`, got {:?}",
            resolved[0].0
        );
    }
}
