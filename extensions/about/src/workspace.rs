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
            if let Ok(entries) = std::fs::read_dir(&parent) {
                for entry in entries.flatten() {
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
    std::fs::read_to_string(dir.join(marker)).ok()
}

fn matches_exclude(pattern: &str, candidate: &str) -> bool {
    if let Some(idx) = pattern.find('*') {
        let prefix = &pattern[..idx];
        let after_star = &pattern[idx + 1..];
        if after_star.is_empty() {
            if let Some(rest) = candidate.strip_prefix(prefix) {
                !rest.is_empty() && !rest.contains('/')
            } else {
                false
            }
        } else {
            false
        }
    } else {
        pattern == candidate
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
