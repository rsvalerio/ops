//! Gradle `project_identity` provider — parses `settings.gradle` + `gradle.properties`.

use std::path::Path;

use ops_core::project_identity::{AboutFieldDef, ProjectIdentity};
use ops_core::text::{dir_name, for_each_trimmed_line};
use ops_extension::{Context, DataProvider, DataProviderError};

use super::java_about_fields;

pub(crate) struct GradleIdentityProvider;

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

        let _ = group;
        let repository = ops_git::resolve_repository_with_git_fallback(&cwd, None);

        let mut identity =
            ProjectIdentity::new(name, "Java", cwd.display().to_string(), "subprojects");
        identity.version = version;
        identity.description = description;
        identity.stack_detail = Some("Gradle".to_string());
        identity.module_count = subproject_count.filter(|&c| c > 0);
        identity.repository = repository;

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
    let mut root_project_name = None;
    let mut includes = Vec::new();

    let mut scan = |line: &str| {
        if let Some(name) = extract_assignment(line, "rootProject.name") {
            root_project_name = Some(name);
        }

        if let Some(rest) = line.strip_prefix("include ") {
            if let Some(val) = extract_quoted(rest.trim()) {
                includes.push(val);
            }
        } else if let Some(rest) = line.strip_prefix("include(") {
            if let Some(val) = extract_quoted(rest.trim_end_matches(')')) {
                includes.push(val);
            }
        }
    };

    for_each_trimmed_line(&project_root.join("settings.gradle"), &mut scan)
        .or_else(|| for_each_trimmed_line(&project_root.join("settings.gradle.kts"), &mut scan))?;

    Some(GradleSettings {
        root_project_name,
        includes,
    })
}

fn parse_gradle_properties(project_root: &Path) -> Option<GradleProperties> {
    let mut version = None;

    for_each_trimmed_line(&project_root.join("gradle.properties"), |line| {
        if let Some(rest) = line.strip_prefix("version=") {
            version = Some(rest.trim().to_string());
        } else if let Some(rest) = line.strip_prefix("version =") {
            version = Some(rest.trim().to_string());
        }
    })?;

    Some(GradleProperties { version })
}

fn parse_gradle_build(project_root: &Path) -> Option<GradleBuild> {
    let mut description = None;
    let mut group = None;

    let mut scan = |line: &str| {
        if let Some(val) = extract_assignment(line, "description") {
            description = Some(val);
        }
        if let Some(val) = extract_assignment(line, "group") {
            group = Some(val);
        }
    };

    for_each_trimmed_line(&project_root.join("build.gradle"), &mut scan)
        .or_else(|| for_each_trimmed_line(&project_root.join("build.gradle.kts"), &mut scan))?;

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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn extract_assignment_no_match() {
        assert_eq!(extract_assignment("other = \"val\"", "description"), None);
    }

    #[test]
    fn extract_assignment_no_equals() {
        assert_eq!(
            extract_assignment("description \"val\"", "description"),
            None
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
    fn parse_gradle_settings_single_quoted() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle"),
            "rootProject.name = 'myapp'\ninclude 'core'\n",
        )
        .unwrap();

        let s = parse_gradle_settings(dir.path()).unwrap();
        assert_eq!(s.root_project_name, Some("myapp".to_string()));
        assert_eq!(s.includes, vec!["core"]);
    }

    #[test]
    fn parse_gradle_settings_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse_gradle_settings(dir.path()).is_none());
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
    fn parse_gradle_properties_with_spaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gradle.properties"), "version = 2.0.0\n").unwrap();

        let p = parse_gradle_properties(dir.path()).unwrap();
        assert_eq!(p.version, Some("2.0.0".to_string()));
    }

    #[test]
    fn parse_gradle_properties_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse_gradle_properties(dir.path()).is_none());
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

    #[test]
    fn parse_gradle_build_kts() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("build.gradle.kts"),
            "description = \"Kotlin Build\"\ngroup = \"com.example\"\n",
        )
        .unwrap();

        let b = parse_gradle_build(dir.path()).unwrap();
        assert_eq!(b.description, Some("Kotlin Build".to_string()));
        assert_eq!(b.group, Some("com.example".to_string()));
    }

    #[test]
    fn parse_gradle_build_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert!(parse_gradle_build(dir.path()).is_none());
    }

    #[test]
    fn gradle_provider_name() {
        assert_eq!(GradleIdentityProvider.name(), "project_identity");
    }

    #[test]
    fn gradle_provider_about_fields() {
        let fields = GradleIdentityProvider.about_fields();
        assert!(!fields.is_empty());
    }

    #[test]
    fn gradle_provider_provide_full() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle"),
            "rootProject.name = \"mygradle\"\ninclude \"api\"\ninclude \"core\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("gradle.properties"), "version=3.2.1\n").unwrap();
        std::fs::write(
            dir.path().join("build.gradle"),
            "description = \"My Gradle Project\"\ngroup = \"com.example\"\n",
        )
        .unwrap();

        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let mut ctx = Context::new(config, dir.path().to_path_buf());
        let result = GradleIdentityProvider.provide(&mut ctx).unwrap();

        assert_eq!(result["name"], "mygradle");
        assert_eq!(result["version"], "3.2.1");
        assert_eq!(result["description"], "My Gradle Project");
        assert_eq!(result["stack_label"], "Java");
        assert_eq!(result["stack_detail"], "Gradle");
        assert_eq!(result["module_count"], 2);
        assert_eq!(result["module_label"], "subprojects");
    }

    #[test]
    fn gradle_provider_provide_minimal() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("settings.gradle"), "// empty settings\n").unwrap();

        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let mut ctx = Context::new(config, dir.path().to_path_buf());
        let result = GradleIdentityProvider.provide(&mut ctx).unwrap();

        let name = result["name"].as_str().unwrap();
        assert!(!name.is_empty());
        assert_eq!(result["stack_detail"], "Gradle");
        assert!(result["version"].is_null());
    }
}
