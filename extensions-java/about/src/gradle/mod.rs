//! Gradle `project_identity` provider — parses `settings.gradle` + `gradle.properties`.
//!
//! FN-1 / TASK-0847: lexer primitives (quote-aware tokenisation, comment
//! stripping, paren matching) live in [`lexer`]; this module owns the
//! Gradle DSL semantics on top of them and the [`GradleIdentityProvider`]
//! impl. Mirrors the maven/pom split.

mod lexer;

use std::path::Path;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::AboutFieldDef;
use ops_core::text::for_each_trimmed_line;
use ops_extension::{Context, DataProvider, DataProviderError};

use super::java_about_fields;
use lexer::{
    extract_quoted, extract_quoted_list, split_at_unquoted_close_paren, strip_properties_comment,
    strip_trailing_comment,
};

pub(crate) struct GradleIdentityProvider;

impl DataProvider for GradleIdentityProvider {
    fn about_fields(&self) -> Vec<AboutFieldDef> {
        java_about_fields()
    }

    fn name(&self) -> &'static str {
        "project_identity"
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        provide_identity_from_manifest(ctx.working_directory.as_path(), |root| {
            let settings = parse_gradle_settings(root);
            let props = parse_gradle_properties(root);
            let build = parse_gradle_build(root);

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

            ParsedManifest::build(|m| {
                m.name = name;
                m.version = version;
                m.description = description;
                m.stack_label = "Java";
                m.stack_detail = Some("Gradle".to_string());
                m.module_label = "subprojects";
                m.module_count = module_count;
            })
        })
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
        parse_include_line(line, &mut includes);
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

/// READ-2 / TASK-0817: handles every Gradle `include` shape on a single line:
///
/// - `include 'a'`                     — Groovy bare
/// - `include 'a', 'b'`                — Groovy multi-arg
/// - `include("a", "b")`               — Kotlin DSL
/// - `include("a"); include("b")`      — Kotlin DSL, multiple per line
/// - `include 'core' // comment`       — trailing comments stripped first
///
/// PATTERN-1 / TASK-0619: split once on the structural `)` rather than
/// `trim_end_matches(')')`, so a quoted argument containing `)` (e.g.
/// `include("legacy)module")`) is preserved.
///
/// PATTERN-1 / TASK-0687: iterate over every `include(` occurrence so chained
/// Kotlin DSL invocations on the same line don't silently drop all but one
/// call.
fn parse_include_line(line: &str, includes: &mut Vec<String>) {
    let stripped = strip_trailing_comment(line).trim_end();
    if stripped.starts_with("include(") {
        let mut remaining = stripped;
        while let Some(rest) = remaining.strip_prefix("include(") {
            match split_at_unquoted_close_paren(rest) {
                Some((args, after)) => {
                    extract_quoted_list(args, includes);
                    remaining = after.trim_start().trim_start_matches(';').trim_start();
                }
                None => {
                    extract_quoted_list(rest, includes);
                    break;
                }
            }
        }
    } else if let Some(rest) = stripped.strip_prefix("include ") {
        extract_quoted_list(rest, includes);
    }
}

/// Extract a value from `key = "value"` or `key = 'value'` or `key="value"`.
/// Requires a word boundary after `key` (next char is `=` or whitespace) so
/// `descriptionText = …` does not match `description`. Trailing `// ...`
/// comments are silently ignored: [`extract_quoted`] terminates at the
/// closing quote, leaving any post-quote text out of the result
/// (`parse_gradle_settings_root_project_name_with_inline_comment` pins this).
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
///
/// READ-2 / TASK-0647: do NOT pre-strip `// …` from `rest` — that chops a
/// quoted URL (`description "see https://example.com"`) at the URL's `//`
/// and silently drops the value. `extract_quoted` terminates at the closing
/// quote, leaving any trailing `// comment` outside the result, which is
/// the same invariant `extract_assignment` relies on.
fn extract_bare_method(line: &str, key: &str) -> Option<String> {
    let line = line.trim();
    let rest = line.strip_prefix(key)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    extract_quoted(rest).map(str::to_string)
}

#[cfg(test)]
mod tests;
