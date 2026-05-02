//! Tests for the Gradle DSL parser. FN-1 / TASK-0847 split the lexer
//! primitives into the sibling `lexer` module; tests reach into both via
//! `super::*` (lexer items are pub(super) so they remain visible here).

use super::lexer::{extract_quoted, extract_quoted_list};
use super::*;

#[test]
fn extract_quoted_list_bails_on_bare_token() {
    let mut out = Vec::new();
    extract_quoted_list(r#""core", noise"#, &mut out);
    assert_eq!(out, vec!["core".to_string()]);
}

#[test]
fn extract_quoted_list_bails_on_unbalanced_quote() {
    let mut out = Vec::new();
    extract_quoted_list(r#""core", "dangling"#, &mut out);
    assert_eq!(out, vec!["core".to_string()]);
}

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
fn parse_gradle_settings_kotlin_chained_includes_same_line() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.gradle.kts"),
        "include(\"a\"); include(\"b\")\n",
    )
    .unwrap();

    let s = parse_gradle_settings(dir.path()).unwrap();
    assert_eq!(s.includes, vec!["a", "b"]);
}

#[test]
fn parse_gradle_settings_kotlin_quoted_arg_with_paren() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.gradle.kts"),
        "include(\"legacy)module\")\n",
    )
    .unwrap();

    let s = parse_gradle_settings(dir.path()).unwrap();
    assert_eq!(s.includes, vec!["legacy)module"]);
}

#[test]
fn parse_gradle_settings_root_project_name_with_inline_comment() {
    // The quote-bounded extract_quoted closes at the second `"`, so any
    // trailing `// comment` is silently discarded. Pin that contract here
    // so a future caller can't regress to a greedy match without breaking
    // a test.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("settings.gradle"),
        "rootProject.name = \"myapp\" // primary\n",
    )
    .unwrap();
    let s = parse_gradle_settings(dir.path()).unwrap();
    assert_eq!(s.root_project_name, Some("myapp".to_string()));
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
fn parse_gradle_properties_value_contains_bang() {
    // READ-2 / TASK-0812: `!` inside the value (no preceding whitespace)
    // is part of the value, not a comment introducer.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("gradle.properties"), "version=1.0!beta\n").unwrap();
    let p = parse_gradle_properties(dir.path()).unwrap();
    assert_eq!(p.version, Some("1.0!beta".to_string()));
}

#[test]
fn parse_gradle_properties_value_contains_hash() {
    // READ-2 / TASK-0812: `#` inside the value (no preceding whitespace)
    // is part of the value, not a comment introducer.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("gradle.properties"),
        "version=1.0#snapshot\n",
    )
    .unwrap();
    let p = parse_gradle_properties(dir.path()).unwrap();
    assert_eq!(p.version, Some("1.0#snapshot".to_string()));
}

#[test]
fn parse_gradle_properties_real_comment_line_skipped() {
    // A `# ...` comment line never matches `strip_prefix("version")`, so
    // a real comment cannot leak through as a version value.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("gradle.properties"),
        "# version=2.0\nversion=3.0\n",
    )
    .unwrap();
    let p = parse_gradle_properties(dir.path()).unwrap();
    assert_eq!(p.version, Some("3.0".to_string()));
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
fn parse_gradle_build_bare_method_url_in_description() {
    // READ-2 / TASK-0647: a `//` inside the quoted value (URL) must not
    // be stripped as a trailing comment.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("build.gradle"),
        "description \"see https://example.com\"\n",
    )
    .unwrap();

    let b = parse_gradle_build(dir.path()).unwrap();
    assert_eq!(b.description, Some("see https://example.com".to_string()));
}

#[test]
fn parse_gradle_build_bare_method_trailing_comment_ignored() {
    // READ-2 / TASK-0647: `extract_quoted` terminates at the closing
    // quote, so any trailing `// ...` is naturally outside the result.
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("build.gradle"),
        "description \"My project\" // primary\n",
    )
    .unwrap();

    let b = parse_gradle_build(dir.path()).unwrap();
    assert_eq!(b.description, Some("My project".to_string()));
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

    let mut ctx = Context::test_context(dir.path().to_path_buf());
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

    let mut ctx = Context::test_context(dir.path().to_path_buf());
    let result = GradleIdentityProvider.provide(&mut ctx).unwrap();

    let name = result["name"].as_str().unwrap();
    assert!(!name.is_empty());
    assert_eq!(result["stack_detail"], "Gradle");
    assert!(result["version"].is_null());
}
