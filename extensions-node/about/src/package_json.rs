//! `package.json` parsing and the npm-shorthand URL / person normalisers.

use std::path::Path;

use serde::Deserialize;

use super::repo_url::{append_tree_directory, normalize_repo_url};

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
    // DUP-3 (TASK-0931): route the read through the shared cache so the
    // sister `workspace_member_globs` site does not pay for a second IO +
    // re-parse on the same `package.json`. Mirrors the Python
    // manifest_cache pattern (TASK-0816).
    let path = project_root.join("package.json");
    let content = crate::manifest_cache::package_json_text(project_root)?;
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

// ARCH-1 / TASK-0848: normalize_repo_url, ssh_to_https, append_tree_directory,
// is_numeric_port_prefix moved to `super::repo_url` so the SEC-14
// path-sanitisation surface lives in its own module with its own test target.

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

    // ARCH-1 / TASK-0848: unit tests for normalize_repo_url and
    // append_tree_directory live next to the implementation in
    // `super::repo_url`. The parse-orchestrator integration tests below
    // exercise the same code via the parse_package_json entry point.

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
}
