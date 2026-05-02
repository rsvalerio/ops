//! `package.json` parsing and the npm-shorthand URL / person normalisers.

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Default)]
#[non_exhaustive]
pub(crate) struct PackageJson {
    pub(crate) name: Option<String>,
    pub(crate) version: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) license: Option<String>,
    pub(crate) homepage: Option<String>,
    pub(crate) repository: Option<String>,
    pub(crate) authors: Vec<String>,
    pub(crate) engines_node: Option<String>,
    pub(crate) has_packagemanager: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawPackage {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    license: Option<LicenseField>,
    homepage: Option<String>,
    repository: Option<RepositoryField>,
    author: Option<PersonField>,
    #[serde(default)]
    contributors: Vec<PersonField>,
    engines: Option<Engines>,
    #[serde(rename = "packageManager")]
    package_manager: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LicenseField {
    Text(String),
    Object { r#type: Option<String> },
}

/// npm's package.json `repository` field. The object form supports a
/// `directory` property that points at a sub-path inside the repository, used
/// by monorepos to distinguish member packages that share one root URL.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RepositoryField {
    Text(String),
    Object {
        url: Option<String>,
        /// Sub-path within the repository (npm-supported, used by monorepos
        /// like babel/react-router). Surfaced as a `/tree/HEAD/<directory>`
        /// suffix on the normalised URL so the About card distinguishes
        /// member packages that share a repository root.
        directory: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum PersonField {
    Text(String),
    Object {
        name: Option<String>,
        email: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct Engines {
    node: Option<String>,
}

pub(crate) fn parse_package_json(project_root: &Path) -> Option<PackageJson> {
    let path = project_root.join("package.json");
    let content = ops_about::manifest_io::read_optional_text(&path, "package.json")?;
    let raw: RawPackage = match serde_json::from_str(&content) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(
                path = ?path.display(),
                error = %e,
                recovery = "default-identity",
                "failed to parse package.json"
            );
            return None;
        }
    };

    // PERF-2 / TASK-0819: bound is `1 (author) + contributors.len()`; allocate
    // once instead of growing through repeated `push`.
    let mut authors = Vec::with_capacity(1 + raw.contributors.len());
    if let Some(a) = raw.author {
        if let Some(s) = format_person(a) {
            authors.push(s);
        }
    }
    for c in raw.contributors {
        if let Some(s) = format_person(c) {
            authors.push(s);
        }
    }

    Some(PackageJson {
        name: trim_nonempty(raw.name),
        version: trim_nonempty(raw.version),
        description: trim_nonempty(raw.description),
        // ERR-2 / TASK-0813: trim+drop-empty for license text and object
        // forms, mirroring the pattern applied to name/version/description
        // and to pyproject's normalize_license — a whitespace-only license
        // value should not render as a blank About bullet.
        license: raw.license.and_then(|l| match l {
            LicenseField::Text(s) => trim_nonempty(Some(s)),
            LicenseField::Object { r#type } => trim_nonempty(r#type),
        }),
        homepage: trim_nonempty(raw.homepage),
        repository: raw.repository.and_then(|r| match r {
            RepositoryField::Text(s) => Some(normalize_repo_url(&s)),
            RepositoryField::Object { url, directory } => url.map(|u| {
                let base = normalize_repo_url(&u);
                match trim_nonempty(directory) {
                    Some(dir) => append_tree_directory(&base, &dir),
                    None => base,
                }
            }),
        }),
        authors,
        // ERR-2 / TASK-0814: trim+drop-empty so a whitespace-only `engines.node`
        // does not render as `Node    · …` in `build_stack_detail`.
        engines_node: raw.engines.and_then(|e| trim_nonempty(e.node)),
        has_packagemanager: raw.package_manager,
    })
}

/// ERR-2 (TASK-0563): trim then drop empties, mirroring how `description`
/// has been normalised since the field landed. Used for `name`, `version`,
/// `homepage` so `package.json` with whitespace-only fields falls through to
/// the dir-name fallback rather than rendering blank About cards.
pub(crate) fn trim_nonempty(value: Option<String>) -> Option<String> {
    value
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn format_person(p: PersonField) -> Option<String> {
    // ERR-2 (TASK-0566): trim and re-check empty so whitespace-only authors do
    // not render as empty bullets in the About card.
    match p {
        PersonField::Text(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        PersonField::Object { name, email } => {
            let name = trim_nonempty(name);
            let email = trim_nonempty(email);
            match (name, email) {
                (Some(n), Some(e)) => Some(format!("{n} <{e}>")),
                (Some(n), None) => Some(n),
                (None, Some(e)) => Some(format!("<{e}>")),
                (None, None) => None,
            }
        }
    }
}

/// Normalize shorthand repository URLs used by npm:
/// - `github:user/repo` → `https://github.com/user/repo`
/// - `git+https://…` / `git://…` → stripped scheme
/// - `git+ssh://git@host[:port]/path.git` / `ssh://git@host/path.git` →
///   `https://host/path` (user-info, port hint, and `.git` are stripped so the
///   identity is browsable in the About card).
fn normalize_repo_url(raw: &str) -> String {
    /// (shorthand prefix, host) for npm hostname shortcuts.
    const HOST_PREFIXES: &[(&str, &str)] = &[
        ("github:", "github.com"),
        ("gitlab:", "gitlab.com"),
        ("bitbucket:", "bitbucket.org"),
    ];

    let s = raw.trim();
    for (prefix, host) in HOST_PREFIXES {
        if let Some(rest) = s.strip_prefix(prefix) {
            return format!("https://{host}/{rest}");
        }
    }
    if let Some(rest) = s
        .strip_prefix("git+ssh://")
        .or_else(|| s.strip_prefix("ssh://"))
    {
        return ssh_to_https(rest);
    }
    if let Some(rest) = s.strip_prefix("git+") {
        return rest.trim_end_matches(".git").to_string();
    }
    if let Some(rest) = s.strip_prefix("git://") {
        return format!("https://{}", rest.trim_end_matches(".git"));
    }
    s.trim_end_matches(".git").to_string()
}

/// Convert the body of an `ssh://` (or `git+ssh://`) URL to its `https://`
/// equivalent: drop the `git@` user-info, replace an scp-form `host:path`
/// separator with `/`, and strip any trailing `.git` suffix. A numeric port
/// (e.g. `host:22/path`) is preserved verbatim.
///
/// PATTERN-1 / TASK-0692: distinguish a numeric port from an scp-form path
/// whose first segment merely begins with a digit (e.g. `host:42-archive/x`)
/// by requiring **all** characters before the next `/` to be digits — a
/// `host:42/foo` is a port, `host:42-archive/x` is an scp-form path.
fn ssh_to_https(rest: &str) -> String {
    let no_user = rest.strip_prefix("git@").unwrap_or(rest);
    let trimmed = no_user.trim_end_matches(".git");
    let body = match trimmed.split_once(':') {
        Some((host, path)) if !is_numeric_port_prefix(path) => {
            format!("{host}/{path}")
        }
        _ => trimmed.to_string(),
    };
    format!("https://{body}")
}

/// Append `/tree/HEAD/<directory>` to a base repository URL so monorepo
/// member packages render distinguishable links. Strips a leading `./` from
/// the directory and canonicalises slashes.
///
/// SEC-14 / TASK-0811: any path component equal to `..` (or any leading
/// absolute slash) is dropped before the suffix is built. An adversarial
/// `package.json` can otherwise emit a directory like `../../../etc/passwd`,
/// which the previous implementation passed through verbatim and produced a
/// traversal-shaped URL rendered into About cards / markdown / HTML. Empty
/// segments and `.` segments are also collapsed for the same reason. If
/// every component is filtered out, the directory suffix is omitted and the
/// base URL is returned unchanged.
fn append_tree_directory(base: &str, directory: &str) -> String {
    let normalized = directory.trim().trim_start_matches("./").replace('\\', "/");
    let cleaned = normalized
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
        .collect::<Vec<_>>()
        .join("/");
    if cleaned.is_empty() {
        return base.to_string();
    }
    let trimmed_base = base.trim_end_matches('/');
    format!("{trimmed_base}/tree/HEAD/{cleaned}")
}

fn is_numeric_port_prefix(path: &str) -> bool {
    let port = path.split('/').next().unwrap_or("");
    !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ERR-7 (TASK-0818): manifest paths flow through `tracing::warn!` via
    /// the `?` formatter so embedded newlines or ANSI escapes cannot forge
    /// multi-line log records. Pin the value-level escape without requiring
    /// a tracing-subscriber dev-dep.
    #[test]
    fn package_json_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/package.json");
        let rendered = format!("{:?}", p.display());
        assert!(!rendered.contains('\n'));
        assert!(!rendered.contains('\u{1b}'));
        assert!(rendered.contains("\\n"));
    }

    #[test]
    fn format_person_email_only_wraps_in_brackets() {
        let p = PersonField::Object {
            name: None,
            email: Some("a@example.com".to_string()),
        };
        assert_eq!(format_person(p), Some("<a@example.com>".to_string()));
    }

    #[test]
    fn format_person_name_and_email() {
        let p = PersonField::Object {
            name: Some("Alice".to_string()),
            email: Some("a@example.com".to_string()),
        };
        assert_eq!(format_person(p), Some("Alice <a@example.com>".to_string()));
    }

    #[test]
    fn format_person_name_only() {
        let p = PersonField::Object {
            name: Some("Alice".to_string()),
            email: None,
        };
        assert_eq!(format_person(p), Some("Alice".to_string()));
    }

    #[test]
    fn format_person_empty_text() {
        assert_eq!(format_person(PersonField::Text(String::new())), None);
    }

    #[test]
    fn format_person_whitespace_text_is_dropped() {
        assert_eq!(format_person(PersonField::Text("   ".into())), None);
    }

    #[test]
    fn format_person_whitespace_object_components_dropped() {
        let p = PersonField::Object {
            name: Some("   ".into()),
            email: Some("\t".into()),
        };
        assert_eq!(format_person(p), None);
    }

    #[test]
    fn parse_package_json_whitespace_only_license_text_dropped() {
        // ERR-2 / TASK-0813
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"x","license":"   "}"#,
        )
        .expect("write");
        let pkg = parse_package_json(dir.path()).expect("parse");
        assert_eq!(pkg.license, None);
    }

    #[test]
    fn parse_package_json_whitespace_only_license_object_type_dropped() {
        // ERR-2 / TASK-0813
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"x","license":{"type":"\t"}}"#,
        )
        .expect("write");
        let pkg = parse_package_json(dir.path()).expect("parse");
        assert_eq!(pkg.license, None);
    }

    #[test]
    fn parse_package_json_whitespace_only_engine_node_dropped() {
        // ERR-2 / TASK-0814
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"x","engines":{"node":"  "}}"#,
        )
        .expect("write");
        let pkg = parse_package_json(dir.path()).expect("parse");
        assert_eq!(pkg.engines_node, None);
    }

    #[test]
    fn parse_package_json_trims_whitespace_only_name_to_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"   ","version":"\t","homepage":"  "}"#,
        )
        .expect("write");
        let pkg = parse_package_json(dir.path()).expect("parse");
        assert_eq!(pkg.name, None);
        assert_eq!(pkg.version, None);
        assert_eq!(pkg.homepage, None);
    }

    #[test]
    fn normalize_git_ssh_to_https() {
        assert_eq!(
            normalize_repo_url("git+ssh://git@github.com/o/r.git"),
            "https://github.com/o/r"
        );
    }

    #[test]
    fn normalize_ssh_scp_form_url() {
        assert_eq!(
            normalize_repo_url("ssh://git@gitlab.com:owner/r.git"),
            "https://gitlab.com/owner/r"
        );
    }

    #[test]
    fn normalize_git_https_unchanged_path() {
        assert_eq!(
            normalize_repo_url("git+https://github.com/o/r.git"),
            "https://github.com/o/r"
        );
    }

    #[test]
    fn repository_object_with_directory_appends_tree_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{
                "name": "@scope/foo",
                "version": "1.0.0",
                "repository": {
                    "type": "git",
                    "url": "https://github.com/example/mono.git",
                    "directory": "packages/foo"
                }
            }"#,
        )
        .unwrap();

        let parsed = parse_package_json(dir.path()).expect("parsed");
        assert_eq!(
            parsed.repository.as_deref(),
            Some("https://github.com/example/mono/tree/HEAD/packages/foo")
        );
    }

    #[test]
    fn repository_object_without_directory_matches_text_form() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{
                "name": "@scope/foo",
                "version": "1.0.0",
                "repository": {
                    "type": "git",
                    "url": "https://github.com/example/mono.git"
                }
            }"#,
        )
        .unwrap();

        let parsed = parse_package_json(dir.path()).expect("parsed");
        assert_eq!(
            parsed.repository.as_deref(),
            Some("https://github.com/example/mono")
        );
    }

    #[test]
    fn normalize_ssh_scp_form_with_digit_prefixed_owner() {
        assert_eq!(
            normalize_repo_url("ssh://git@github.com:42-archive/x.git"),
            "https://github.com/42-archive/x"
        );
    }

    #[test]
    fn normalize_ssh_with_numeric_port_keeps_port() {
        assert_eq!(
            normalize_repo_url("ssh://git@host:22/path.git"),
            "https://host:22/path"
        );
    }

    /// SEC-14 / TASK-0811: a `directory` that escapes the repository root via
    /// `..` segments must be sanitized — the URL is rendered into About cards
    /// (and downstream markdown/HTML), so a traversal-shaped suffix is a
    /// real surface for path-shape attacks.
    #[test]
    fn append_tree_directory_strips_leading_parent_segments() {
        assert_eq!(
            append_tree_directory("https://github.com/o/r", "../foo"),
            "https://github.com/o/r/tree/HEAD/foo"
        );
    }

    #[test]
    fn append_tree_directory_strips_internal_parent_segments() {
        assert_eq!(
            append_tree_directory("https://github.com/o/r", "a/../b"),
            "https://github.com/o/r/tree/HEAD/a/b"
        );
    }

    #[test]
    fn append_tree_directory_strips_absolute_leading_slash() {
        assert_eq!(
            append_tree_directory("https://github.com/o/r", "/absolute"),
            "https://github.com/o/r/tree/HEAD/absolute"
        );
    }

    #[test]
    fn append_tree_directory_drops_when_only_parent_components() {
        assert_eq!(
            append_tree_directory("https://github.com/o/r", "../../.."),
            "https://github.com/o/r"
        );
    }

    #[test]
    fn append_tree_directory_pure_traversal_etc_passwd_is_neutralised() {
        // The motivating case from the SEC-14 finding: an adversarial
        // package.json must not produce a URL whose path component contains
        // `../../etc/passwd` style traversal.
        let url = append_tree_directory("https://github.com/o/r", "../../../../etc/passwd");
        assert!(!url.contains(".."), "url still contains ..: {url}");
        assert_eq!(url, "https://github.com/o/r/tree/HEAD/etc/passwd");
    }

    #[test]
    fn normalize_github_shorthand() {
        assert_eq!(
            normalize_repo_url("github:owner/repo"),
            "https://github.com/owner/repo"
        );
    }
}
