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
//! treated like the bare-`*` form: the prefix is enumerated, suffix is
//! ignored. Exclusion patterns follow the same shape and filter the resolved
//! list (TASK-0389 / TASK-0400).

use std::path::Path;

use crate::manifest_io::read_optional_text;

/// Resolve workspace member globs against `root`, looking for `marker`
/// (e.g. `"package.json"`, `"pyproject.toml"`) inside each candidate
/// directory. Excludes are matched with the same prefix-only glob shape and
/// applied after expansion.
pub fn resolve_member_globs(
    members: &[String],
    excludes: &[String],
    root: &Path,
    marker: &str,
) -> Vec<(String, String)> {
    let mut resolved: Vec<(String, String)> = Vec::new();
    for member in members {
        if let Some(idx) = member.find('*') {
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
                            if let Ok(rel) = path.strip_prefix(root) {
                                resolved.push((rel.to_string_lossy().to_string(), manifest));
                            }
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
/// Multi-`*` patterns are not supported and surface a `tracing::warn` so a
/// silently-failed-closed exclude becomes visible.
fn matches_exclude(pattern: &str, candidate: &str) -> bool {
    let star_count = pattern.bytes().filter(|b| *b == b'*').count();
    if star_count == 0 {
        return pattern == candidate;
    }
    if star_count > 1 {
        tracing::warn!(
            pattern,
            "workspace exclude pattern has more than one `*`; not supported, ignoring"
        );
        return false;
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

    /// PATTERN-1 (TASK-0503): multi-`*` patterns are explicitly unsupported
    /// and must not silently drop the exclude.
    #[test]
    fn multi_star_exclude_is_ignored() {
        assert!(!matches_exclude("a/*/b/*", "a/x/b/y"));
    }

    /// Suffix-after-`*` (e.g. `prefix/*/suffix`) is documented but unsupported
    /// — the suffix part is ignored and only the prefix is enumerated. Test
    /// guards against silent breakage of the existing semantics.
    #[test]
    fn suffix_after_star_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("packages/a/package.json"), r#"{}"#);

        let resolved = resolve_member_globs(
            &["packages/*/sub".to_string()],
            &[],
            dir.path(),
            "package.json",
        );
        let names: Vec<&str> = resolved.iter().map(|(p, _)| p.as_str()).collect();
        assert_eq!(names, vec!["packages/a"]);
    }
}
