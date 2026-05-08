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

/// Resolve workspace member globs against `root`, looking for `marker`
/// (e.g. `"package.json"`, `"pyproject.toml"`) inside each candidate
/// directory. Excludes are matched with the same single-`*`-per-pattern glob
/// shape and applied after expansion.
///
/// PATTERN-1 (TASK-1052): an exclude pattern with more than one `*` is
/// unsupported and fails **closed** — the candidate is dropped (treated as
/// matching) and a `tracing::warn` is emitted, rather than the previous
/// fail-open behaviour that silently let the unit through.
pub fn resolve_member_globs(
    members: &[String],
    excludes: &[String],
    root: &Path,
    marker: &str,
) -> Vec<(String, String)> {
    let mut resolved: Vec<(String, String)> = Vec::new();
    for member in members {
        // PATTERN-1 (TASK-1071): reject `..` traversal in member values before
        // any I/O. Workspace config is operator-controlled so the impact is
        // low, but `root.join(member)` is otherwise the only surface where a
        // `../sibling` entry escapes the workspace root. Aligns with the
        // SEC-13 dot-only-segment work in `git/src/remote.rs`.
        if Path::new(member)
            .components()
            .any(|c| matches!(c, Component::ParentDir))
        {
            tracing::warn!(member, "workspace member contains `..` traversal; skipping");
            continue;
        }
        if let Some(idx) = member.find('*') {
            // PATTERN-1 (TASK-1069): the original implementation found the
            // first `*` and treated everything before it as the prefix,
            // silently ignoring any suffix after it. That meant `**/foo`
            // (prefix `""`) brute-force-scanned the workspace root, and
            // `prefix/*/suffix` silently flattened to `prefix/*`. Reject
            // both shapes explicitly with a `tracing::warn` so the
            // divergence is observable rather than silent.
            let suffix = &member[idx + 1..];
            let is_recursive = member.contains("**");
            let suffix_is_trivial = suffix.is_empty() || !suffix.contains('/');
            if is_recursive || !suffix_is_trivial {
                tracing::warn!(
                    member,
                    "workspace member glob shape unsupported (only single trailing `*` per segment); skipping"
                );
                continue;
            }
            let prefix = &member[..idx];
            let parent = root.join(prefix);
            // ERR-1 (TASK-0517): a read_dir error here used to silently
            // produce "No project units found". Log at warn so a permissions
            // or missing-prefix issue is visible without changing the
            // best-effort behavior that lets the rest of the globs resolve.
            match std::fs::read_dir(&parent) {
                Ok(entries) => {
                    // ERR-1 (TASK-0942): replace `entries.flatten()` with an
                    // explicit match so a per-entry IO error (EACCES on a
                    // sibling member, EIO, ...) is visible at warn level
                    // rather than silently disappearing into "no project
                    // units found". Mirrors the policy the surrounding
                    // `read_dir` arm already adopted in TASK-0517.
                    for entry in entries {
                        let entry = match entry {
                            Ok(e) => e,
                            Err(e) => {
                                tracing::warn!(
                                    member,
                                    parent = ?parent.display(),
                                    error = ?e,
                                    "workspace glob entry unreadable; skipping"
                                );
                                continue;
                            }
                        };
                        let path = entry.path();
                        if !path.is_dir() {
                            continue;
                        }
                        if let Some(manifest) = try_read_manifest(&path, marker) {
                            // ERR-1 (TASK-1070): a `strip_prefix` failure
                            // here used to silently drop a successfully-read
                            // manifest — typically when `root` and
                            // `entry.path()` disagree on symlink resolution
                            // (common on macOS via `/var` vs
                            // `/private/var`). Try canonicalising both
                            // sides as a fallback, log a tracing breadcrumb
                            // either way, and fall back to the absolute
                            // path so the unit is not silently lost.
                            let rel_string = match path.strip_prefix(root) {
                                Ok(rel) => rel.to_string_lossy().to_string(),
                                Err(_) => {
                                    let canonical_rel =
                                        std::fs::canonicalize(root).ok().and_then(|root_canon| {
                                            std::fs::canonicalize(&path).ok().and_then(|p_canon| {
                                                p_canon
                                                    .strip_prefix(&root_canon)
                                                    .ok()
                                                    .map(|r| r.to_string_lossy().to_string())
                                            })
                                        });
                                    match canonical_rel {
                                        Some(rel) => {
                                            tracing::debug!(
                                                root = ?root.display(),
                                                path = ?path.display(),
                                                "workspace strip_prefix failed; recovered via canonicalize"
                                            );
                                            rel
                                        }
                                        None => {
                                            tracing::warn!(
                                                root = ?root.display(),
                                                path = ?path.display(),
                                                "workspace strip_prefix failed and canonicalize did not recover; falling back to absolute path so manifest is not silently dropped"
                                            );
                                            path.to_string_lossy().to_string()
                                        }
                                    }
                                }
                            };
                            resolved.push((rel_string, manifest));
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // ERR-7 (TASK-0665): Debug-format the path so embedded
                    // newlines / ANSI escapes cannot forge log lines.
                    tracing::debug!(
                        member,
                        parent = ?parent.display(),
                        "workspace glob prefix does not exist; skipping"
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        member,
                        parent = ?parent.display(),
                        error = ?e,
                        "workspace glob prefix unreadable; member skipped"
                    );
                }
            }
        } else if let Some(manifest) = try_read_manifest(&root.join(member), marker) {
            resolved.push((member.clone(), manifest));
        }
    }
    if !excludes.is_empty() {
        resolved.retain(|(m, _)| !excludes.iter().any(|pat| matches_exclude(pat, m)));
    }
    resolved.sort_by(|a, b| a.0.cmp(&b.0));
    resolved.dedup_by(|a, b| a.0 == b.0);
    resolved
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
    let idx = pattern
        .find('*')
        .expect("star_count == 1 implies a `*` exists");
    let prefix = &pattern[..idx];
    let suffix = &pattern[idx + 1..];
    let Some(rest) = candidate.strip_prefix(prefix) else {
        return false;
    };
    let Some(middle) = rest.strip_suffix(suffix) else {
        return false;
    };
    !middle.is_empty() && !middle.contains('/')
}

/// Manifest-level identity fields surfaced by the units providers. Replaces
/// the old positional `(Option<String>, Option<String>, Option<String>)` so
/// argument-order errors at call sites become compile errors.
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
    /// still resolve normally. The accompanying tracing::warn is exercised
    /// by the read_dir failure path; pinning the value-level contract here
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

    /// PATTERN-1 (TASK-1069): non-trivial suffix-after-`*` (e.g.
    /// `prefix/*/suffix`) is now explicitly skipped rather than silently
    /// flattened onto the prefix. The valid sibling member must still load
    /// to confirm the skip is per-pattern, not whole-call.
    #[test]
    fn suffix_after_star_is_skipped_with_warning() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("packages/a/package.json"), r#"{}"#);
        write(&dir.path().join("apps/web/package.json"), r#"{}"#);

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
        write(&dir.path().join("a/package.json"), r#"{}"#);
        write(&dir.path().join("b/package.json"), r#"{}"#);

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
