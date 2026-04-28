//! `package.json` parsing and the npm-shorthand URL / person normalisers.

use std::path::Path;

use serde::Deserialize;

#[derive(Debug, Default)]
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RepositoryField {
    Text(String),
    Object { url: Option<String> },
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
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::debug!(path = %path.display(), error = %e, "failed to read package.json");
            }
            return None;
        }
    };
    let raw: RawPackage = match serde_json::from_str(&content) {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!(path = %path.display(), error = %e, "failed to parse package.json");
            return None;
        }
    };

    let mut out = PackageJson {
        name: raw.name,
        version: raw.version,
        description: raw
            .description
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        license: raw.license.and_then(|l| match l {
            LicenseField::Text(s) => Some(s),
            LicenseField::Object { r#type } => r#type,
        }),
        homepage: raw.homepage.filter(|s| !s.is_empty()),
        repository: raw.repository.and_then(|r| match r {
            RepositoryField::Text(s) => Some(normalize_repo_url(&s)),
            RepositoryField::Object { url } => url.map(|u| normalize_repo_url(&u)),
        }),
        engines_node: raw.engines.and_then(|e| e.node),
        has_packagemanager: raw.package_manager,
        ..PackageJson::default()
    };

    let mut authors = Vec::new();
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
    out.authors = authors;

    Some(out)
}

fn format_person(p: PersonField) -> Option<String> {
    match p {
        PersonField::Text(s) => Some(s).filter(|s| !s.is_empty()),
        PersonField::Object { name, email } => match (name, email) {
            (Some(n), Some(e)) => Some(format!("{n} <{e}>")),
            (Some(n), None) => Some(n),
            (None, Some(e)) => Some(format!("<{e}>")),
            (None, None) => None,
        },
    }
}

/// Normalize shorthand repository URLs used by npm:
/// - `github:user/repo` → `https://github.com/user/repo`
/// - `git+https://…` / `git://…` → stripped scheme
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
    if let Some(rest) = s.strip_prefix("git+") {
        return rest.trim_end_matches(".git").to_string();
    }
    if let Some(rest) = s.strip_prefix("git://") {
        return format!("https://{}", rest.trim_end_matches(".git"));
    }
    s.trim_end_matches(".git").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
