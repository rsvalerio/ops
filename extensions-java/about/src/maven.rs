//! Maven `project_identity` provider — parses `pom.xml`.

use std::path::Path;

use ops_core::project_identity::{AboutFieldDef, ProjectIdentity};
use ops_core::text::dir_name;
use ops_extension::{Context, DataProvider, DataProviderError};

use super::java_about_fields;

pub(crate) struct MavenIdentityProvider;

impl DataProvider for MavenIdentityProvider {
    fn name(&self) -> &'static str {
        "project_identity"
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        java_about_fields()
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();
        let pom = parse_pom_xml(&cwd)
            .ok_or_else(|| DataProviderError::computation_failed("could not parse pom.xml"))?;

        let identity = ProjectIdentity {
            name: pom
                .artifact_id
                .unwrap_or_else(|| dir_name(&cwd).to_string()),
            version: pom.version,
            description: pom.description,
            stack_label: "Java".to_string(),
            stack_detail: Some("Maven".to_string()),
            license: pom.license,
            project_path: cwd.display().to_string(),
            module_count: if pom.modules.is_empty() {
                None
            } else {
                Some(pom.modules.len())
            },
            module_label: "modules".to_string(),
            authors: pom.developers,
            repository: pom
                .scm_url
                .filter(|s| !s.is_empty())
                .or_else(|| ops_git::GitInfo::collect(&cwd).remote_url),
            ..Default::default()
        };

        serde_json::to_value(&identity).map_err(DataProviderError::from)
    }
}

pub(crate) struct PomData {
    pub artifact_id: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub modules: Vec<String>,
    pub developers: Vec<String>,
    pub scm_url: Option<String>,
}

/// Tracks which POM section we're currently inside.
#[derive(PartialEq)]
enum PomSection {
    TopLevel,
    Modules,
    Developers { in_developer: bool },
    Scm,
    Licenses,
}

/// Simple line-based XML extraction from pom.xml.
///
/// Not a full XML parser — extracts top-level elements by matching opening/closing
/// tags. Sufficient for the standard Maven POM fields we need.
pub(crate) fn parse_pom_xml(project_root: &Path) -> Option<PomData> {
    let content = std::fs::read_to_string(project_root.join("pom.xml")).ok()?;

    let mut data = PomData {
        artifact_id: None,
        version: None,
        description: None,
        license: None,
        modules: Vec::new(),
        developers: Vec::new(),
        scm_url: None,
    };

    let mut started = false;
    let mut section = PomSection::TopLevel;

    for line in content.lines() {
        let line = line.trim();

        if line.starts_with("<project") {
            started = true;
            continue;
        }
        if line == "</project>" {
            break;
        }
        if !started {
            continue;
        }

        // Check for section transitions at top level
        if section == PomSection::TopLevel {
            if let Some(new_section) = match_section_open(line) {
                section = new_section;
                continue;
            }
            parse_top_level(line, &mut data);
            continue;
        }

        // Handle section-specific content and closing tags
        match &mut section {
            PomSection::Modules => {
                if line == "</modules>" {
                    section = PomSection::TopLevel;
                } else if let Some(val) = extract_xml_value(line, "module") {
                    data.modules.push(val);
                }
            }
            PomSection::Developers { in_developer } => {
                if line == "</developers>" {
                    section = PomSection::TopLevel;
                } else if line == "<developer>" {
                    *in_developer = true;
                } else if line == "</developer>" {
                    *in_developer = false;
                } else if *in_developer {
                    if let Some(val) = extract_xml_value(line, "name") {
                        data.developers.push(val);
                    }
                }
            }
            PomSection::Scm => {
                if line == "</scm>" {
                    section = PomSection::TopLevel;
                } else if data.scm_url.is_none() {
                    data.scm_url = extract_xml_value(line, "url");
                }
            }
            PomSection::Licenses => {
                if line == "</licenses>" {
                    section = PomSection::TopLevel;
                } else if data.license.is_none() {
                    data.license = extract_xml_value(line, "name");
                }
            }
            PomSection::TopLevel => unreachable!(),
        }
    }

    Some(data)
}

/// Match opening tags for POM sections.
fn match_section_open(line: &str) -> Option<PomSection> {
    if line == "<modules>" {
        Some(PomSection::Modules)
    } else if line == "<developers>" {
        Some(PomSection::Developers {
            in_developer: false,
        })
    } else if line == "<scm>" || line.starts_with("<scm>") {
        Some(PomSection::Scm)
    } else if line == "<licenses>" || line.starts_with("<licenses>") {
        Some(PomSection::Licenses)
    } else {
        None
    }
}

/// Parse top-level simple elements (artifactId, version, description, name, url).
fn parse_top_level(line: &str, data: &mut PomData) {
    if data.artifact_id.is_none() {
        if let Some(val) = extract_xml_value(line, "artifactId") {
            data.artifact_id = Some(val);
        }
    }
    if data.version.is_none() {
        data.version = extract_xml_value(line, "version");
    }
    if data.description.is_none() {
        data.description = extract_xml_value(line, "description");
    }
    // <name> overrides artifactId for display purposes.
    if let Some(val) = extract_xml_value(line, "name") {
        data.artifact_id = Some(val);
    }
    // Only use top-level <url> if no SCM URL found.
    if data.scm_url.is_none() {
        data.scm_url = extract_xml_value(line, "url");
    }
}

/// Extract value from `<tag>value</tag>` on a single line.
pub(crate) fn extract_xml_value(line: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = line.find(&open) {
        if let Some(end) = line.find(&close) {
            let val_start = start + open.len();
            if val_start < end {
                return Some(line[val_start..end].trim().to_string());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_xml_value_basic() {
        assert_eq!(
            extract_xml_value("<artifactId>camel</artifactId>", "artifactId"),
            Some("camel".to_string())
        );
    }

    #[test]
    fn extract_xml_value_with_whitespace() {
        assert_eq!(
            extract_xml_value("    <version>1.0</version>  ", "version"),
            Some("1.0".to_string())
        );
    }

    #[test]
    fn extract_xml_value_no_match() {
        assert_eq!(extract_xml_value("<name>foo</name>", "version"), None);
    }

    #[test]
    fn parse_pom_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<?xml version="1.0"?>
<project>
    <artifactId>myapp</artifactId>
    <version>2.0.0</version>
    <name>My App</name>
    <description>A cool app</description>
    <modules>
        <module>core</module>
        <module>web</module>
    </modules>
    <developers>
        <developer>
            <name>Alice</name>
        </developer>
    </developers>
    <scm>
        <url>https://github.com/user/myapp</url>
    </scm>
    <licenses>
        <license>
            <name>Apache-2.0</name>
        </license>
    </licenses>
</project>"#,
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("My App".to_string()));
        assert_eq!(pom.version, Some("2.0.0".to_string()));
        assert_eq!(pom.description, Some("A cool app".to_string()));
        assert_eq!(pom.modules, vec!["core", "web"]);
        assert_eq!(pom.developers, vec!["Alice"]);
        assert_eq!(
            pom.scm_url,
            Some("https://github.com/user/myapp".to_string())
        );
        assert_eq!(pom.license, Some("Apache-2.0".to_string()));
    }

    #[test]
    fn parse_pom_minimal() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <artifactId>simple</artifactId>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.artifact_id, Some("simple".to_string()));
        assert!(pom.version.is_none());
        assert!(pom.modules.is_empty());
    }

    #[test]
    fn parse_pom_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse_pom_xml(dir.path()).is_none());
    }

    #[test]
    fn parse_pom_top_level_url_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <artifactId>mylib</artifactId>\n    <url>https://example.com</url>\n</project>",
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.scm_url, Some("https://example.com".to_string()));
    }

    #[test]
    fn parse_pom_scm_takes_precedence_over_url() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<project>
    <artifactId>mylib</artifactId>
    <scm>
        <url>https://github.com/user/mylib</url>
    </scm>
    <url>https://example.com</url>
</project>"#,
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(
            pom.scm_url,
            Some("https://github.com/user/mylib".to_string())
        );
    }

    #[test]
    fn parse_pom_multiple_developers() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<project>
    <artifactId>multi</artifactId>
    <developers>
        <developer>
            <name>Alice</name>
        </developer>
        <developer>
            <name>Bob</name>
        </developer>
    </developers>
</project>"#,
        )
        .unwrap();

        let pom = parse_pom_xml(dir.path()).unwrap();
        assert_eq!(pom.developers, vec!["Alice", "Bob"]);
    }

    #[test]
    fn maven_provider_name() {
        assert_eq!(MavenIdentityProvider.name(), "project_identity");
    }

    #[test]
    fn maven_provider_about_fields() {
        let fields = MavenIdentityProvider.about_fields();
        assert!(!fields.is_empty());
    }

    #[test]
    fn maven_provider_provide_success() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<project>
    <artifactId>testapp</artifactId>
    <version>1.0.0</version>
    <description>Test application</description>
</project>"#,
        )
        .unwrap();

        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let mut ctx = Context::new(config, dir.path().to_path_buf());
        let result = MavenIdentityProvider.provide(&mut ctx).unwrap();

        assert_eq!(result["name"], "testapp");
        assert_eq!(result["version"], "1.0.0");
        assert_eq!(result["description"], "Test application");
        assert_eq!(result["stack_label"], "Java");
        assert_eq!(result["stack_detail"], "Maven");
    }

    #[test]
    fn maven_provider_provide_no_pom() {
        let dir = tempfile::tempdir().unwrap();
        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let mut ctx = Context::new(config, dir.path().to_path_buf());
        assert!(MavenIdentityProvider.provide(&mut ctx).is_err());
    }

    #[test]
    fn maven_provider_provide_uses_dir_name_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <version>1.0</version>\n</project>",
        )
        .unwrap();

        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let mut ctx = Context::new(config, dir.path().to_path_buf());
        let result = MavenIdentityProvider.provide(&mut ctx).unwrap();

        let name = result["name"].as_str().unwrap();
        assert!(!name.is_empty());
    }

    #[test]
    fn maven_provider_provide_with_modules() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r#"<project>
    <artifactId>parent</artifactId>
    <modules>
        <module>api</module>
        <module>impl</module>
        <module>web</module>
    </modules>
</project>"#,
        )
        .unwrap();

        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let mut ctx = Context::new(config, dir.path().to_path_buf());
        let result = MavenIdentityProvider.provide(&mut ctx).unwrap();

        assert_eq!(result["module_count"], 3);
        assert_eq!(result["module_label"], "modules");
    }
}
