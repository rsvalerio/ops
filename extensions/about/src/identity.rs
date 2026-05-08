//! Shared `project_identity` builder used by stack `*IdentityProvider`s.
//!
//! Centralises the parse-then-build-`ProjectIdentity` skeleton each stack
//! provider previously copied (TASK-0387). Stacks parse their own manifest
//! shape, project the result onto [`ParsedManifest`], and call
//! [`build_identity_value`] which fills in the canonical fields, applies
//! the git-remote repository fallback, and serialises to JSON.

use std::path::Path;

use ops_core::project_identity::{LanguageStat, ProjectIdentity};
use ops_core::text::dir_name;
use ops_extension::DataProviderError;

/// Stack-agnostic projection of a parsed manifest, ready to be turned into
/// a [`ProjectIdentity`] via [`build_identity_value`]. Fields with no
/// equivalent in a given stack should remain at their `Default` value.
///
/// `#[non_exhaustive]`: out-of-crate construction must go through
/// [`ParsedManifest::build`] so new identity fields can be added without
/// breaking existing stack providers.
#[derive(Debug, Default)]
#[non_exhaustive]
pub struct ParsedManifest {
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub license: Option<String>,
    pub authors: Vec<String>,
    pub homepage: Option<String>,
    pub repository: Option<String>,
    pub stack_label: &'static str,
    pub stack_detail: Option<String>,
    pub module_label: &'static str,
    pub module_count: Option<usize>,
    pub loc: Option<i64>,
    pub file_count: Option<i64>,
    pub msrv: Option<String>,
    pub dependency_count: Option<usize>,
    pub coverage_percent: Option<f64>,
    pub languages: Vec<LanguageStat>,
}

impl ParsedManifest {
    /// Build a `ParsedManifest` by mutating a default value through a closure.
    ///
    /// Out-of-crate stack providers cannot use struct-literal syntax once the
    /// type is `#[non_exhaustive]`, so this helper provides a stable
    /// construction path that ignores fields the stack does not populate.
    pub fn build(f: impl FnOnce(&mut Self)) -> Self {
        let mut m = Self::default();
        f(&mut m);
        m
    }
}

/// Run a stack's manifest parser against the working directory and serialise
/// the result via [`build_identity_value`]. Captures the
/// `let cwd = ...; let parsed = parser(&cwd); build_identity_value(parsed, &cwd)`
/// scaffold every stack `*IdentityProvider::provide` was duplicating
/// (DUP-1 / TASK-0484).
///
/// Stack providers that need to merge data from multiple manifests (e.g. the
/// Go provider, which combines `go.mod` and `go.work`) can still call
/// [`build_identity_value`] directly; this helper covers the common
/// single-parser case.
pub fn provide_identity_from_manifest<F>(
    cwd: &Path,
    parser: F,
) -> Result<serde_json::Value, DataProviderError>
where
    F: FnOnce(&Path) -> ParsedManifest,
{
    build_identity_value(parser(cwd), cwd)
}

/// Build a [`ProjectIdentity`] JSON value from a [`ParsedManifest`].
///
/// Falls back to the working-directory name when `name` is absent and
/// applies the git-remote repository fallback when no manifest-supplied
/// repository URL is present.
///
/// ERR-1 / TASK-1103: rejects a non-UTF-8 `cwd` with a typed
/// [`DataProviderError::ComputationFailed`] rather than letting
/// `Path::display` smuggle `U+FFFD` replacement bytes into the
/// `project_root` JSON field. This mirrors the strict
/// [`ops_duckdb::DbError::NonUtf8Path`] policy adopted in TASK-0928 for
/// `upsert_data_source`: any path persisted into a downstream consumer
/// (DuckDB row, JSON identity payload, audit log) must round-trip
/// faithfully, so the two paths now share the same fail-fast contract.
pub fn build_identity_value(
    manifest: ParsedManifest,
    cwd: &Path,
) -> Result<serde_json::Value, DataProviderError> {
    let ParsedManifest {
        name,
        version,
        description,
        license,
        authors,
        homepage,
        repository,
        stack_label,
        stack_detail,
        module_label,
        module_count,
        loc,
        file_count,
        msrv,
        dependency_count,
        coverage_percent,
        languages,
    } = manifest;

    // ERR-1 / TASK-1103: reject non-UTF-8 cwd up front. `Path::display`
    // would otherwise replace each invalid byte with `U+FFFD`, silently
    // corrupting the `project_root` field of every downstream identity
    // JSON. See module-level / fn-level docs for the shared contract
    // with `upsert_data_source`'s `NonUtf8Path`.
    let project_root = cwd.to_str().ok_or_else(|| {
        DataProviderError::computation_failed(format!(
            "project_root path is not valid UTF-8: {}",
            cwd.display()
        ))
    })?;

    let name = name.unwrap_or_else(|| dir_name(cwd).to_string());
    let repository = ops_git::resolve_repository_with_git_fallback(cwd, repository);

    let mut identity = ProjectIdentity::new(name, stack_label, project_root, module_label);
    identity.version = version;
    identity.description = description;
    identity.stack_detail = stack_detail;
    identity.license = license;
    identity.authors = authors;
    identity.repository = repository;
    identity.homepage = homepage;
    identity.module_count = module_count;
    identity.loc = loc;
    identity.file_count = file_count;
    identity.msrv = msrv;
    identity.dependency_count = dependency_count;
    identity.coverage_percent = coverage_percent;
    identity.languages = languages;

    serde_json::to_value(&identity).map_err(DataProviderError::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ERR-1 / TASK-1103: a non-UTF-8 `cwd` must fail fast with a typed
    /// [`DataProviderError::ComputationFailed`] rather than silently
    /// shipping `U+FFFD`-mangled bytes into the `project_root` JSON
    /// field. Mirrors the `upsert_data_source` `NonUtf8Path` test in
    /// `ops-duckdb`.
    #[test]
    #[cfg(unix)]
    fn build_identity_value_rejects_non_utf8_cwd() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let bad_cwd = Path::new(OsStr::from_bytes(b"/ws/\xff/proj"));
        let manifest = ParsedManifest::build(|m| {
            m.stack_label = "rust";
            m.module_label = "crate";
        });

        let err = build_identity_value(manifest, bad_cwd)
            .expect_err("non-UTF-8 cwd must yield a typed error");
        assert!(matches!(err, DataProviderError::ComputationFailed(_)));
    }
}
