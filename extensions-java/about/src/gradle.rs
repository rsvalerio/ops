//! Gradle `project_identity` provider — parses `settings.gradle` + `gradle.properties`.

use std::path::Path;

use ops_about::identity::{build_identity_value, ParsedManifest};
use ops_core::project_identity::AboutFieldDef;
use ops_core::text::for_each_trimmed_line;
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

        let (name, module_count) = match settings {
            Some(GradleSettings {
                root_project_name,
                includes,
            }) => {
                let count = (!includes.is_empty()).then_some(includes.len());
                (root_project_name, count)
            }
            None => (None, None),
        };
        let version = props.and_then(|GradleProperties { version }| version);
        let description = build.and_then(|GradleBuild { description }| description);

        build_identity_value(
            ParsedManifest {
                name,
                version,
                description,
                stack_label: "Java",
                stack_detail: Some("Gradle".to_string()),
                module_label: "subprojects",
                module_count,
                ..ParsedManifest::default()
            },
            &cwd,
        )
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
}

fn parse_gradle_settings(project_root: &Path) -> Option<GradleSettings> {
    let mut root_project_name = None;
    let mut includes = Vec::new();

    let mut scan = |line: &str| {
        if let Some(name) = extract_assignment(line, "rootProject.name") {
            root_project_name = Some(name);
        }

        // include 'a'         (Groovy bare)
        // include 'a', 'b'    (Groovy multi-arg)
        // include("a", "b")   (Kotlin DSL)
        // include 'core' // comment
        let after_include = line
            .strip_prefix("include(")
            .map(|r| r.trim_end_matches(|c: char| c.is_whitespace() || c == ')'))
            .or_else(|| line.strip_prefix("include ").map(strip_trailing_comment));
        if let Some(rest) = after_include {
            extract_quoted_list(rest, &mut includes);
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
        if let Some(rest) = line.strip_prefix("version") {
            // .properties syntax: `key = value`, `key:value`, or `key value`
            // (we accept = and : here; trailing # / ! is a comment).
            let rest = rest.trim_start();
            let rest = match rest.strip_prefix('=').or_else(|| rest.strip_prefix(':')) {
                Some(r) => r,
                None => return,
            };
            let value = strip_properties_comment(rest).trim();
            if !value.is_empty() {
                version = Some(value.to_string());
            }
        }
    })?;

    Some(GradleProperties { version })
}

fn parse_gradle_build(project_root: &Path) -> Option<GradleBuild> {
    let mut description = None;

    let mut scan = |line: &str| {
        if let Some(val) = extract_assignment(line, "description") {
            description = Some(val);
        } else if let Some(val) = extract_bare_method(line, "description") {
            description = Some(val);
        }
    };

    for_each_trimmed_line(&project_root.join("build.gradle"), &mut scan)
        .or_else(|| for_each_trimmed_line(&project_root.join("build.gradle.kts"), &mut scan))?;

    Some(GradleBuild { description })
}

/// Extract a value from `key = "value"` or `key = 'value'` or `key="value"`.
/// Requires a word boundary after `key` (next char is `=` or whitespace) so
/// `descriptionText = …` does not match `description`.
fn extract_assignment(line: &str, key: &str) -> Option<String> {
    let line = line.trim();
    let rest = line.strip_prefix(key)?;
    if !rest.starts_with('=') && !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=')?;
    extract_quoted(rest.trim()).map(str::to_string)
}

/// Extract a value from the Groovy bare-method form `key "value"` or
/// `key 'value'` (no `=`). Rejects `keyTask { … }` and similar by requiring
/// whitespace after `key` and a quoted value immediately after.
fn extract_bare_method(line: &str, key: &str) -> Option<String> {
    let line = line.trim();
    let rest = line.strip_prefix(key)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    extract_quoted(strip_trailing_comment(rest)).map(str::to_string)
}

/// Extract a quoted string value: `"foo"` or `'foo'`.
fn extract_quoted(s: &str) -> Option<&str> {
    let s = s.trim();
    let (open, rest) = if let Some(r) = s.strip_prefix('"') {
        ('"', r)
    } else if let Some(r) = s.strip_prefix('\'') {
        ('\'', r)
    } else {
        return None;
    };
    let end = rest.find(open)?;
    Some(&rest[..end])
}

/// Extract every quoted token from a comma-separated list of values:
/// `'a', 'b', "c"`. Pushes each unquoted token into `out`.
fn extract_quoted_list(s: &str, out: &mut Vec<String>) {
    let mut rest = strip_trailing_comment(s).trim();
    while !rest.is_empty() {
        let Some(quote) = rest.chars().next().filter(|c| *c == '"' || *c == '\'') else {
            return;
        };
        let after = &rest[1..];
        let Some(end) = after.find(quote) else {
            return;
        };
        out.push(after[..end].to_string());
        rest = after[end + 1..].trim_start();
        if let Some(next) = rest.strip_prefix(',') {
            rest = next.trim_start();
        } else {
            break;
        }
    }
}

/// Strip a trailing `// ...` Groovy/Kotlin comment from a line fragment.
fn strip_trailing_comment(s: &str) -> &str {
    match s.find("//") {
        Some(i) => &s[..i],
        None => s,
    }
}

/// Strip a trailing `# ...` or `! ...` java.util.Properties comment.
fn strip_properties_comment(s: &str) -> &str {
    let cut = s
        .find('#')
        .into_iter()
        .chain(s.find('!'))
        .min()
        .unwrap_or(s.len());
    &s[..cut]
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
    fn extract_assignment_no_equals_does_not_match() {
        // bare-method form is handled by extract_bare_method, not extract_assignment
        assert_eq!(
            extract_assignment("description \"val\"", "description"),
            None
        );
    }

    #[test]
    fn extract_assignment_word_boundary() {
        assert_eq!(
            extract_assignment("rootProject.nameOverride = \"x\"", "rootProject.name"),
            None
        );
        assert_eq!(
            extract_assignment("descriptionText = \"x\"", "description"),
            None
        );
    }

    #[test]
    fn extract_bare_method_basic() {
        assert_eq!(
            extract_bare_method("description \"My project\"", "description"),
            Some("My project".to_string())
        );
        assert_eq!(
            extract_bare_method("description 'My project'", "description"),
            Some("My project".to_string())
        );
    }

    #[test]
    fn extract_bare_method_rejects_block() {
        assert_eq!(
            extract_bare_method("descriptionTask { foo() }", "description"),
            None
        );
    }

    #[test]
    fn extract_quoted_double() {
        assert_eq!(extract_quoted("\"hello\""), Some("hello"));
    }

    #[test]
    fn extract_quoted_single() {
        assert_eq!(extract_quoted("'hello'"), Some("hello"));
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
    fn parse_gradle_settings_multi_arg_groovy() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle"),
            "include 'a', 'b', 'c'\n",
        )
        .unwrap();

        let s = parse_gradle_settings(dir.path()).unwrap();
        assert_eq!(s.includes, vec!["a", "b", "c"]);
    }

    #[test]
    fn parse_gradle_settings_multi_arg_kotlin() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle.kts"),
            "include(\"a\", \"b\")\n",
        )
        .unwrap();

        let s = parse_gradle_settings(dir.path()).unwrap();
        assert_eq!(s.includes, vec!["a", "b"]);
    }

    #[test]
    fn parse_gradle_settings_inline_comment() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("settings.gradle"),
            "include 'core' // primary module\n",
        )
        .unwrap();

        let s = parse_gradle_settings(dir.path()).unwrap();
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
    fn parse_gradle_properties_colon_separator() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gradle.properties"), "version : 1.2\n").unwrap();

        let p = parse_gradle_properties(dir.path()).unwrap();
        assert_eq!(p.version, Some("1.2".to_string()));
    }

    #[test]
    fn parse_gradle_properties_inline_comment() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("gradle.properties"),
            "version=1.2 # release\n",
        )
        .unwrap();

        let p = parse_gradle_properties(dir.path()).unwrap();
        assert_eq!(p.version, Some("1.2".to_string()));
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
    }

    #[test]
    fn parse_gradle_build_bare_method() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("build.gradle"),
            "description \"Bare method form\"\n",
        )
        .unwrap();

        let b = parse_gradle_build(dir.path()).unwrap();
        assert_eq!(b.description, Some("Bare method form".to_string()));
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
