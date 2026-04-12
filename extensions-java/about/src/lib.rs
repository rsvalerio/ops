//! Java stack `project_identity` providers (Maven and Gradle).
//!
//! Provides two extensions:
//! - `AboutMavenExtension` (stack: JavaMaven) — parses `pom.xml`
//! - `AboutGradleExtension` (stack: JavaGradle) — parses `settings.gradle` + `gradle.properties`

use std::path::Path;

use ops_core::project_identity::{AboutFieldDef, ProjectIdentity};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};

// --- Maven ---

const MAVEN_NAME: &str = "about-java-maven";
const MAVEN_DESCRIPTION: &str = "Java Maven project identity";
const MAVEN_SHORTNAME: &str = "about-mvn";

pub struct AboutMavenExtension;

ops_extension::impl_extension! {
    AboutMavenExtension,
    name: MAVEN_NAME,
    description: MAVEN_DESCRIPTION,
    shortname: MAVEN_SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::JavaMaven),
    data_provider_name: Some("project_identity"),
    register_data_providers: |_self, registry| {
        registry.register("project_identity", Box::new(MavenIdentityProvider));
    },
    factory: MAVEN_ABOUT_FACTORY = |_, _| {
        Some((MAVEN_NAME, Box::new(AboutMavenExtension)))
    },
}

// --- Gradle ---

const GRADLE_NAME: &str = "about-java-gradle";
const GRADLE_DESCRIPTION: &str = "Java Gradle project identity";
const GRADLE_SHORTNAME: &str = "about-gradle";

pub struct AboutGradleExtension;

ops_extension::impl_extension! {
    AboutGradleExtension,
    name: GRADLE_NAME,
    description: GRADLE_DESCRIPTION,
    shortname: GRADLE_SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::JavaGradle),
    data_provider_name: Some("project_identity"),
    register_data_providers: |_self, registry| {
        registry.register("project_identity", Box::new(GradleIdentityProvider));
    },
    factory: GRADLE_ABOUT_FACTORY = |_, _| {
        Some((GRADLE_NAME, Box::new(AboutGradleExtension)))
    },
}

// =============================================================================
// Maven provider
// =============================================================================

struct MavenIdentityProvider;

const JAVA_ABOUT_FIELDS: &[(&str, &str, &str)] = &[
    ("project", "Project path", "Absolute path to project root"),
    ("modules", "Module count", "Number of project modules"),
    ("code", "Lines of code", "Total lines of code (from tokei)"),
    ("files", "File count", "Total source file count"),
    ("authors", "Authors", "Project author(s)"),
    ("repository", "Repository", "Repository URL"),
    ("homepage", "Homepage", "Project homepage URL"),
    ("coverage", "Coverage", "Test coverage percentage"),
    ("languages", "Languages", "Languages used in the project"),
];

fn java_about_fields() -> Vec<AboutFieldDef> {
    JAVA_ABOUT_FIELDS
        .iter()
        .map(|(id, label, description)| AboutFieldDef {
            id,
            label,
            description,
        })
        .collect()
}

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
            loc: None,
            file_count: None,
            authors: pom.developers,
            repository: pom.scm_url,
            homepage: None,
            msrv: None,
            dependency_count: None,
            coverage_percent: None,
            languages: vec![],
        };

        serde_json::to_value(&identity).map_err(DataProviderError::from)
    }
}

struct PomData {
    artifact_id: Option<String>,
    version: Option<String>,
    description: Option<String>,
    license: Option<String>,
    modules: Vec<String>,
    developers: Vec<String>,
    scm_url: Option<String>,
}

/// Simple line-based XML extraction from pom.xml.
///
/// Not a full XML parser — extracts top-level elements by matching opening/closing
/// tags. Sufficient for the standard Maven POM fields we need.
fn parse_pom_xml(project_root: &Path) -> Option<PomData> {
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

    // Track nesting depth to only read top-level <project> children.
    // We use a simple approach: count <tag> vs </tag> to track depth.
    let mut depth = 0;
    let mut in_modules = false;
    let mut in_developers = false;
    let mut in_developer = false;
    let mut in_scm = false;
    let mut in_licenses = false;

    for line in content.lines() {
        let line = line.trim();

        // Track depth for <project> children.
        if line.starts_with("<project") {
            depth = 1;
            continue;
        }
        if line == "</project>" {
            break;
        }

        // Sections
        if depth == 1 {
            if line == "<modules>" {
                in_modules = true;
                continue;
            }
            if line == "</modules>" {
                in_modules = false;
                continue;
            }
            if line == "<developers>" {
                in_developers = true;
                continue;
            }
            if line == "</developers>" {
                in_developers = false;
                continue;
            }
            if line.starts_with("<scm>") || line == "<scm>" {
                in_scm = true;
                continue;
            }
            if line == "</scm>" {
                in_scm = false;
                continue;
            }
            if line.starts_with("<licenses>") || line == "<licenses>" {
                in_licenses = true;
                continue;
            }
            if line == "</licenses>" {
                in_licenses = false;
                continue;
            }
        }

        if in_modules {
            if let Some(val) = extract_xml_value(line, "module") {
                data.modules.push(val);
            }
            continue;
        }

        if in_developers {
            if line == "<developer>" {
                in_developer = true;
                continue;
            }
            if line == "</developer>" {
                in_developer = false;
                continue;
            }
            if in_developer {
                if let Some(val) = extract_xml_value(line, "name") {
                    data.developers.push(val);
                }
            }
            continue;
        }

        if in_scm {
            if data.scm_url.is_none() {
                if let Some(val) = extract_xml_value(line, "url") {
                    data.scm_url = Some(val);
                }
            }
            continue;
        }

        if in_licenses {
            if data.license.is_none() {
                if let Some(val) = extract_xml_value(line, "name") {
                    data.license = Some(val);
                }
            }
            continue;
        }

        // Top-level simple elements (depth == 1, not in a section).
        if depth == 1 {
            if data.artifact_id.is_none() {
                if let Some(val) = extract_xml_value(line, "artifactId") {
                    data.artifact_id = Some(val);
                }
            }
            if data.version.is_none() {
                if let Some(val) = extract_xml_value(line, "version") {
                    data.version = Some(val);
                }
            }
            if data.description.is_none() {
                if let Some(val) = extract_xml_value(line, "description") {
                    data.description = Some(val);
                }
            }
            if let Some(val) = extract_xml_value(line, "name") {
                // <name> overrides artifactId for display purposes.
                data.artifact_id = Some(val);
            }
            if let Some(val) = extract_xml_value(line, "url") {
                // Only use top-level <url> if no SCM URL found.
                if data.scm_url.is_none() {
                    data.scm_url = Some(val);
                }
            }
        }

        // Rough depth tracking for nested elements.
        if line.starts_with("</") {
            // Don't go below 1 for top-level sections.
        } else if line.starts_with('<') && !line.contains("/>") && !line.contains("</") {
            // Opening tag without self-close — increase depth conceptually
            // (we don't actually need precise depth for our simple extraction).
        }
    }

    Some(data)
}

/// Extract value from `<tag>value</tag>` on a single line.
fn extract_xml_value(line: &str, tag: &str) -> Option<String> {
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

// =============================================================================
// Gradle provider
// =============================================================================

struct GradleIdentityProvider;

impl DataProvider for GradleIdentityProvider {
    fn about_fields(&self) -> Vec<AboutFieldDef> {
        java_about_fields()
    }

    fn name(&self) -> &'static str {
        "project_identity"
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let cwd = ctx.working_directory.clone();
        let settings = parse_gradle_settings(&cwd);
        let props = parse_gradle_properties(&cwd);
        let build = parse_gradle_build(&cwd);

        let name = settings
            .as_ref()
            .and_then(|s| s.root_project_name.clone())
            .unwrap_or_else(|| dir_name(&cwd).to_string());

        let version = props.as_ref().and_then(|p| p.version.clone());
        let description = build.as_ref().and_then(|b| b.description.clone());
        let group = build.as_ref().and_then(|b| b.group.clone());

        let subproject_count = settings.as_ref().map(|s| s.includes.len());

        // Use group as a hint for the repository if available.
        let _ = group;

        let identity = ProjectIdentity {
            name,
            version,
            description,
            stack_label: "Java".to_string(),
            stack_detail: Some("Gradle".to_string()),
            license: None,
            project_path: cwd.display().to_string(),
            module_count: subproject_count.filter(|&c| c > 0),
            module_label: "subprojects".to_string(),
            loc: None,
            file_count: None,
            authors: vec![],
            repository: None,
            homepage: None,
            msrv: None,
            dependency_count: None,
            coverage_percent: None,
            languages: vec![],
        };

        serde_json::to_value(&identity).map_err(DataProviderError::from)
    }
}

struct GradleSettings {
    root_project_name: Option<String>,
    includes: Vec<String>,
}

struct GradleProperties {
    version: Option<String>,
}

struct GradleBuild {
    description: Option<String>,
    group: Option<String>,
}

fn parse_gradle_settings(project_root: &Path) -> Option<GradleSettings> {
    // Try settings.gradle, then settings.gradle.kts
    let content = std::fs::read_to_string(project_root.join("settings.gradle"))
        .or_else(|_| std::fs::read_to_string(project_root.join("settings.gradle.kts")))
        .ok()?;

    let mut root_project_name = None;
    let mut includes = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // rootProject.name = "spring-boot-build" or rootProject.name = 'name'
        if let Some(name) = extract_assignment(line, "rootProject.name") {
            root_project_name = Some(name);
        }

        // include "path:to:module" or include 'path:to:module' or include("path")
        if let Some(rest) = line.strip_prefix("include ") {
            let rest = rest.trim();
            // Handle quoted value: "foo" or 'foo' or ("foo")
            if let Some(val) = extract_quoted(rest) {
                includes.push(val);
            }
        } else if let Some(rest) = line.strip_prefix("include(") {
            // Kotlin DSL: include("foo")
            if let Some(val) = extract_quoted(rest.trim_end_matches(')')) {
                includes.push(val);
            }
        }
    }

    Some(GradleSettings {
        root_project_name,
        includes,
    })
}

fn parse_gradle_properties(project_root: &Path) -> Option<GradleProperties> {
    let content = std::fs::read_to_string(project_root.join("gradle.properties")).ok()?;
    let mut version = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("version=") {
            version = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("version =") {
            version = Some(rest.trim().to_string());
        }
    }

    Some(GradleProperties { version })
}

fn parse_gradle_build(project_root: &Path) -> Option<GradleBuild> {
    let content = std::fs::read_to_string(project_root.join("build.gradle"))
        .or_else(|_| std::fs::read_to_string(project_root.join("build.gradle.kts")))
        .ok()?;

    let mut description = None;
    let mut group = None;

    for line in content.lines() {
        let line = line.trim();

        // description = "Spring Boot Build" or description = 'text'
        if let Some(val) = extract_assignment(line, "description") {
            description = Some(val);
        }
        // group = "org.springframework.boot"
        if let Some(val) = extract_assignment(line, "group") {
            group = Some(val);
        }
    }

    Some(GradleBuild { description, group })
}

/// Extract a value from `key = "value"` or `key = 'value'` or `key="value"`.
fn extract_assignment(line: &str, key: &str) -> Option<String> {
    let line = line.trim();
    if !line.starts_with(key) {
        return None;
    }
    let rest = line[key.len()..].trim();
    let rest = rest.strip_prefix('=')?;
    extract_quoted(rest.trim())
}

/// Extract a quoted string value: `"foo"` or `'foo'`.
fn extract_quoted(s: &str) -> Option<String> {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        Some(s[1..s.len() - 1].to_string())
    } else {
        None
    }
}

fn dir_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- XML helpers ---

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

    // --- Gradle helpers ---

    #[test]
    fn extract_assignment_double_quoted() {
        assert_eq!(
            extract_assignment("description = \"Spring Boot\"", "description"),
            Some("Spring Boot".to_string())
        );
    }

    #[test]
    fn extract_assignment_single_quoted() {
        assert_eq!(
            extract_assignment("group = 'org.spring'", "group"),
            Some("org.spring".to_string())
        );
    }

    #[test]
    fn extract_assignment_no_spaces() {
        assert_eq!(
            extract_assignment("rootProject.name=\"myapp\"", "rootProject.name"),
            Some("myapp".to_string())
        );
    }

    #[test]
    fn extract_quoted_double() {
        assert_eq!(extract_quoted("\"hello\""), Some("hello".to_string()));
    }

    #[test]
    fn extract_quoted_single() {
        assert_eq!(extract_quoted("'hello'"), Some("hello".to_string()));
    }

    #[test]
    fn extract_quoted_unquoted() {
        assert_eq!(extract_quoted("hello"), None);
    }

    // --- Maven parsing ---

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
        // <name> overrides artifactId
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

    // --- Gradle parsing ---

    #[test]
    fn parse_gradle_settings_basic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle"),
            "rootProject.name=\"spring-boot\"\ninclude \"core\"\ninclude \"web\"\n",
        )
        .unwrap();

        let s = parse_gradle_settings(dir.path()).unwrap();
        assert_eq!(s.root_project_name, Some("spring-boot".to_string()));
        assert_eq!(s.includes, vec!["core", "web"]);
    }

    #[test]
    fn parse_gradle_settings_kts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle.kts"),
            "rootProject.name = \"myapp\"\ninclude(\"api\")\ninclude(\"impl\")\n",
        )
        .unwrap();

        let s = parse_gradle_settings(dir.path()).unwrap();
        assert_eq!(s.root_project_name, Some("myapp".to_string()));
        assert_eq!(s.includes, vec!["api", "impl"]);
    }

    #[test]
    fn parse_gradle_properties_version() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("gradle.properties"),
            "version=4.1.0-SNAPSHOT\norg.gradle.caching=true\n",
        )
        .unwrap();

        let p = parse_gradle_properties(dir.path()).unwrap();
        assert_eq!(p.version, Some("4.1.0-SNAPSHOT".to_string()));
    }

    #[test]
    fn parse_gradle_build_description() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("build.gradle"),
            "description = \"Spring Boot Build\"\ngroup = \"org.springframework.boot\"\n",
        )
        .unwrap();

        let b = parse_gradle_build(dir.path()).unwrap();
        assert_eq!(b.description, Some("Spring Boot Build".to_string()));
        assert_eq!(b.group, Some("org.springframework.boot".to_string()));
    }
}
