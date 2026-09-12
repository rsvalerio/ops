//! Gradle `project_identity` provider — parses `settings.gradle` + `gradle.properties`.
//!
//! Lexer primitives (quote-aware tokenisation, comment stripping, paren
//! matching) live in [`lexer`]; this module owns the Gradle DSL semantics built
//! on top of them and the [`GradleIdentityProvider`] impl. Mirrors the
//! maven/pom split.

mod lexer;

use std::path::Path;

use ops_about::cards::format_unit_name;
use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{AboutFieldDef, ProjectUnit};
use ops_core::text::for_each_trimmed_line;
use ops_extension::{Context, DataProvider, DataProviderError};

use super::gradle_about_fields;
use lexer::{
    brace_delta, extract_quoted, extract_quoted_list, split_at_unquoted_close_paren,
    strip_properties_comment, strip_trailing_comment,
};

pub struct GradleIdentityProvider;

impl DataProvider for GradleIdentityProvider {
    fn about_fields(&self) -> Vec<AboutFieldDef> {
        gradle_about_fields()
    }

    fn name(&self) -> &'static str {
        "project_identity"
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        provide_identity_from_manifest(ctx.working_directory(), |root| {
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

/// Parse `settings.gradle` (or its `.kts` sibling) for the root project name
/// and the `include`d subprojects.
///
/// `rootProject.name` is only accepted at brace depth 0: an assignment inside
/// a block belongs to that block, not to the root project. Duplicate
/// resolution is **first writer wins**, matching the sibling Maven parser's
/// `try_set_once` (`maven/pom.rs`), so the two parsers in this crate resolve
/// duplicates the same way.
///
/// `include` directives get two further protections, because their count feeds
/// `module_count` directly:
///
/// - **Depth-gated**, like `rootProject.name` above. An `include` inside a
///   multi-line block (`gradle.beforeSettings { … }`, an `if (…) { … }`
///   spanning lines) belongs to that block and may never execute, so counting
///   it inflates the subproject total. (A single-line conditional
///   `if (…) { include(":x") }` never matches [`parse_include_line`] at all —
///   it only accepts lines that *start* with the directive.)
/// - **Deduplicated on the normalised project path** ([`normalise_include`]):
///   Gradle treats `include` as idempotent on the project path, so
///   `include ':app'` followed by `include 'app'` is one subproject, not two.
///   The first raw spelling is kept (first writer wins); only the dedup key is
///   canonical.
fn parse_gradle_settings(project_root: &Path) -> Option<GradleSettings> {
    let mut root_project_name: Option<String> = None;
    let mut includes = Vec::new();
    let mut depth = 0_i32;

    let mut scan = |line: &str| {
        if depth == 0 {
            if root_project_name.is_none() {
                root_project_name = extract_assignment(line, "rootProject.name");
            }
            parse_include_line(line, &mut includes);
        }
        depth = depth.saturating_add(brace_delta(line)).max(0);
    };

    for_each_trimmed_line(&project_root.join("settings.gradle"), &mut scan)
        .or_else(|| for_each_trimmed_line(&project_root.join("settings.gradle.kts"), &mut scan))?;

    // Dedup on the normalised path, first writer wins.
    let mut seen = std::collections::HashSet::new();
    let mut deduped = Vec::with_capacity(includes.len());
    for include in includes {
        if seen.insert(normalise_include(&include)) {
            deduped.push(include);
        }
    }

    Some(GradleSettings {
        root_project_name,
        includes: deduped,
    })
}

/// Canonical form of a Gradle project path, used as the include dedup key.
///
/// Gradle's path separator is `:` and the leading `:` of an absolute project
/// path is optional, so `":a"` and `"a"` denote
/// the same subproject, and `/`- or `\`-separated spellings of the same path
/// (`"apps/web"` for `"apps:web"`) collapse onto the `:` form.
fn normalise_include(entry: &str) -> String {
    entry
        .trim()
        .trim_start_matches(':')
        .replace(['\\', '/'], ":")
}

/// Parse `gradle.properties` for the project `version`.
///
/// This parser resolves duplicates **last writer wins**, deliberately
/// diverging from the first-wins policy of [`parse_gradle_settings`],
/// [`parse_gradle_build`] and the Maven `try_set_once`. `gradle.properties` is
/// a `java.util.Properties` file, and `Properties::load` lets a later entry
/// override an earlier one for the same key: matching Gradle's own behaviour
/// matters more here than matching the sibling DSL parsers. A properties file
/// has no brace nesting, so the depth rule does not apply either.
fn parse_gradle_properties(project_root: &Path) -> Option<GradleProperties> {
    let mut version = None;

    for_each_trimmed_line(&project_root.join("gradle.properties"), |line| {
        if let Some(rest) = line.strip_prefix("version") {
            // .properties syntax: `key = value`, `key:value`, or `key value`
            // (we accept = and : here; trailing # / ! is a comment).
            let rest = rest.trim_start();
            let Some(rest) = rest.strip_prefix('=').or_else(|| rest.strip_prefix(':')) else {
                return;
            };
            let value = strip_properties_comment(rest).trim();
            if !value.is_empty() {
                version = Some(value.to_string());
            }
        }
    })?;

    Some(GradleProperties { version })
}

/// Parse `build.gradle` (or its `.kts` sibling) for the root project's
/// `description`.
///
/// `description` is a standard `Task` property, so a
/// `tasks.register(…) { description = … }` or `subprojects { description = … }`
/// block routinely sets it too. Only an assignment at brace depth 0 belongs to
/// the root project; nested ones are ignored. [`brace_delta`] counts braces
/// outside string literals and comments, so a `{` inside a quoted value does
/// not open a phantom block.
///
/// Duplicate resolution is **first writer wins**, matching the sibling Maven
/// parser's `try_set_once` (`maven/pom.rs`) so both parsers in this crate
/// resolve duplicates the same way.
fn parse_gradle_build(project_root: &Path) -> Option<GradleBuild> {
    let mut description: Option<String> = None;
    let mut depth = 0_i32;

    let mut scan = |line: &str| {
        if depth == 0 && description.is_none() {
            description = extract_assignment(line, "description")
                .or_else(|| extract_bare_method(line, "description"));
        }
        // Depth is updated *after* the match: a line that both assigns and
        // opens a block (`description = 'x'` … `foo {`) is still top level.
        // `max(0)` keeps a stray `}` from pushing depth negative and locking
        // out every later top-level assignment.
        depth = depth.saturating_add(brace_delta(line)).max(0);
    };

    for_each_trimmed_line(&project_root.join("build.gradle"), &mut scan)
        .or_else(|| for_each_trimmed_line(&project_root.join("build.gradle.kts"), &mut scan))?;

    Some(GradleBuild { description })
}

/// Collect the project paths from every Gradle `include` shape that fits on a
/// single line:
///
/// - `include 'a'`                     — Groovy bare
/// - `include 'a', 'b'`                — Groovy multi-arg
/// - `include("a", "b")`               — Kotlin DSL
/// - `include("a"); include("b")`      — Kotlin DSL, multiple per line
/// - `include 'core' // comment`       — trailing comments stripped first
///
/// The Kotlin form splits once on the structural `)` rather than trimming
/// trailing `)` characters, so a quoted argument containing `)` (e.g.
/// `include("legacy)module")`) is preserved, and the loop visits every
/// `include(` occurrence so chained invocations on one line all contribute.
///
/// This is the **only** point on the include path where comments are stripped,
/// and [`strip_trailing_comment`] is quote-aware, so an argument containing
/// `//` (`include('a//b')`) survives; `extract_quoted_list` deliberately does
/// not strip again.
fn parse_include_line(line: &str, includes: &mut Vec<String>) {
    let stripped = strip_trailing_comment(line).trim_end();
    if stripped.starts_with("include(") {
        let mut remaining = stripped;
        while let Some(rest) = remaining.strip_prefix("include(") {
            if let Some((args, after)) = split_at_unquoted_close_paren(rest) {
                extract_quoted_list(args, includes);
                remaining = after.trim_start().trim_start_matches(';').trim_start();
            } else {
                extract_quoted_list(rest, includes);
                break;
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
/// `// …` is deliberately not pre-stripped from `rest`: that would chop a
/// quoted URL (`description "see https://example.com"`) at the URL's `//` and
/// drop the value. `extract_quoted` terminates at the closing quote, leaving
/// any trailing `// comment` outside the result — the same invariant
/// `extract_assignment` relies on.
fn extract_bare_method(line: &str, key: &str) -> Option<String> {
    let line = line.trim();
    let rest = line.strip_prefix(key)?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest = rest.trim_start();
    extract_quoted(rest).map(str::to_string)
}

/// Gradle `project_units` provider: one [`ProjectUnit`] per `include` entry.
///
/// The identity card counts `include` entries, so the units page lists exactly
/// those subprojects; otherwise the card's "N subprojects" and the units table
/// would disagree by construction.
pub struct GradleUnitsProvider;

impl DataProvider for GradleUnitsProvider {
    fn name(&self) -> &'static str {
        "project_units"
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let units = collect_units(ctx.working_directory());
        serde_json::to_value(&units).map_err(DataProviderError::from)
    }
}

/// Build one [`ProjectUnit`] per `include` entry — the same list the
/// identity provider counts, so `units.len()` always equals its
/// `module_count`.
///
/// Gradle spells nested projects with `:` separators (`include ":app:core"`),
/// while enrichment joins `unit.path` against cwd-relative file paths, so the
/// project path is normalised to the on-disk spelling (`app/core`).
fn collect_units(cwd: &Path) -> Vec<ProjectUnit> {
    let Some(GradleSettings { includes, .. }) = parse_gradle_settings(cwd) else {
        return Vec::new();
    };
    includes
        .into_iter()
        .map(|include| {
            let path = include
                .trim_start_matches(':')
                .replace(':', std::path::MAIN_SEPARATOR_STR);
            ProjectUnit::new(format_unit_name(&path), path)
        })
        .collect()
}

#[cfg(test)]
mod tests;
