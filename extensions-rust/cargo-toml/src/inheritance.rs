//! Workspace inheritance resolution for Cargo.toml manifests.
//!
//! Resolves `{ workspace = true }` fields and dependencies by merging from the
//! workspace section. Called by [`CargoToml::resolve_inheritance`] and
//! [`CargoToml::resolve_package_inheritance`].

use std::collections::BTreeMap;

use crate::types::{
    CargoToml, DepSpec, DetailedDepSpec, InheritableField, InheritableString, InheritableVec,
    PublishSpec, ReadmeSpec,
};

/// Error during workspace inheritance resolution.
#[derive(Debug, Clone, thiserror::Error)]
#[non_exhaustive]
pub enum InheritanceError {
    /// Dependency marked as `workspace = true` but not found in workspace.
    #[error("dependency '{name}' (in [{section}]) not found in workspace.dependencies")]
    MissingWorkspaceDependency { name: String, section: &'static str },
}

impl CargoToml {
    /// Merges inherited dependencies (workspace = true) with workspace definitions.
    ///
    /// After calling this, all dependencies with `workspace = true` will have
    /// their values filled from `workspace.dependencies`.
    ///
    /// # Errors
    ///
    /// [`InheritanceError`] if a field declares `workspace = true` but the
    /// workspace table has no corresponding entry to inherit from.
    pub fn resolve_inheritance(&mut self) -> Result<(), InheritanceError> {
        let Some(ws) = &self.workspace else {
            return Ok(());
        };

        let ws_deps = &ws.dependencies;

        resolve_deps_inheritance(&mut self.dependencies, ws_deps, "dependencies")?;
        resolve_deps_inheritance(&mut self.dev_dependencies, ws_deps, "dev-dependencies")?;
        resolve_deps_inheritance(&mut self.build_dependencies, ws_deps, "build-dependencies")?;

        Ok(())
    }

    /// Resolves package fields inherited from `workspace.package`.
    ///
    /// After calling this, all package fields with `{ workspace = true }` will have
    /// their values filled from `workspace.package`.
    pub fn resolve_package_inheritance(&mut self) {
        let Some(pkg) = &mut self.package else {
            return;
        };
        let Some(ws) = &self.workspace else {
            return;
        };
        let Some(ws_pkg) = &ws.package else {
            return;
        };

        // Each line below routes one inheritable field through its matching
        // resolver. Adding a new inheritable field is one line here plus a
        // counterpart in `WorkspacePackage` — no risk of touching three
        // places to add a single field.
        resolve_string_field(&mut pkg.version, ws_pkg.version.as_ref());
        resolve_string_field(&mut pkg.edition, ws_pkg.edition.as_ref());
        resolve_string_field(&mut pkg.rust_version, ws_pkg.rust_version.as_ref());
        resolve_string_field(&mut pkg.description, ws_pkg.description.as_ref());
        resolve_string_field(&mut pkg.documentation, ws_pkg.documentation.as_ref());
        resolve_string_field(&mut pkg.homepage, ws_pkg.homepage.as_ref());
        resolve_string_field(&mut pkg.repository, ws_pkg.repository.as_ref());
        resolve_string_field(&mut pkg.license, ws_pkg.license.as_ref());

        resolve_vec_field(&mut pkg.keywords, &ws_pkg.keywords);
        resolve_vec_field(&mut pkg.categories, &ws_pkg.categories);

        if matches!(
            &pkg.authors,
            InheritableField::Inherited { workspace: true }
        ) {
            pkg.authors = InheritableField::Value(ws_pkg.authors.clone());
        }

        resolve_optional_string(&mut pkg.license_file, ws_pkg.license_file.as_ref());
        resolve_readme(&mut pkg.readme, ws_pkg.readme.as_ref());
        resolve_publish(&mut pkg.publish, &ws_pkg.publish);
    }
}

/// Resolve a `field.workspace = true` reference by copying from the
/// matching workspace value.
///
/// `field.workspace = false` is **permissively ignored**: cargo itself
/// rejects this shape (it is parseable as TOML but not valid Cargo
/// semantics), but ops-cargo-toml treats the field as if it were absent so
/// downstream tooling can still introspect malformed-but-readable
/// manifests. See `inheritance::tests::resolve_string_field_workspace_false_is_ignored`.
pub fn resolve_string_field(field: &mut InheritableString, ws_value: Option<&String>) {
    if matches!(field, InheritableField::Inherited { workspace: true }) {
        if let Some(v) = ws_value {
            *field = InheritableField::Value(v.clone());
        }
    }
}

/// Like [`resolve_string_field`] but for `Vec<String>` fields. Substitutes the
/// workspace value verbatim (cloning) when the local field is in the
/// `Inherited { workspace: true }` state.
///
/// TASK-0961: `WorkspacePackage::keywords`/`categories` are plain `Vec<String>`
/// (serde defaults to empty), so an absent workspace `keywords` table is
/// indistinguishable from `keywords = []`. Treat an empty workspace value as
/// "not declared" and leave the member field as `Inherited`, so member intent
/// is not silently overwritten with a forced empty Vec.
pub fn resolve_vec_field(field: &mut InheritableVec, ws_value: &[String]) {
    if matches!(field, InheritableField::Inherited { workspace: true }) && !ws_value.is_empty() {
        *field = InheritableField::Value(ws_value.to_vec());
    }
}

/// Resolve `license-file = { workspace = true }` against the workspace's
/// `license-file`. Mirrors [`resolve_string_field`] but for `Option<InheritableString>`.
pub fn resolve_optional_string(field: &mut Option<InheritableString>, ws_value: Option<&String>) {
    if let Some(inner) = field {
        resolve_string_field(inner, ws_value);
    }
}

/// Resolve `readme = { workspace = true }` against the workspace's `readme`.
pub fn resolve_readme(field: &mut Option<ReadmeSpec>, ws_value: Option<&ReadmeSpec>) {
    if matches!(field, Some(ReadmeSpec::Inherited { workspace: true })) {
        if let Some(v) = ws_value {
            *field = Some(v.clone());
        }
    }
}

/// Resolve `publish = { workspace = true }` against the workspace's `publish`.
///
/// **Fail-closed rule (SEC-31 / TASK-1789).** `WorkspacePackage::publish` is
/// `#[serde(default)]` over [`PublishSpec`], whose default is
/// [`PublishSpec::None`] — "no `publish` key", which
/// [`PublishSpec::is_publishable`] maps to `Some(true)`. An undeclared
/// workspace `publish` is therefore indistinguishable from an explicit
/// "publishable to any registry", and substituting it would rewrite the
/// member's unresolved `Inherited` into an open default — the exact signal
/// loss TASK-1196 introduced `Option<bool>` to prevent (cargo itself hard-
/// errors on this manifest shape).
///
/// So, like [`resolve_string_field`] / [`resolve_vec_field`] /
/// [`resolve_readme`], this resolver substitutes only when the workspace
/// actually declared the field; otherwise the member field stays
/// `Inherited` and `is_publishable()` keeps returning `None`.
pub fn resolve_publish(field: &mut PublishSpec, ws_value: &PublishSpec) {
    if matches!(field, PublishSpec::Inherited { workspace: true })
        && !matches!(ws_value, PublishSpec::None)
    {
        *field = ws_value.clone();
    }
}

fn resolve_deps_inheritance(
    deps: &mut BTreeMap<String, DepSpec>,
    ws_deps: &BTreeMap<String, DepSpec>,
    section: &'static str,
) -> Result<(), InheritanceError> {
    for (name, dep) in deps {
        if dep.is_workspace_inherited() {
            *dep = resolve_dep_from_workspace(name, dep, ws_deps, section)?;
        }
    }
    Ok(())
}

/// Resolve a single dependency marked `workspace = true` from the workspace
/// dependency table.
///
/// # Intentionally ignored local fields
///
/// Cargo forbids a member from overriding the *source* of a workspace-inherited
/// dependency. When `workspace = true` is set on a detailed dep, only `features`,
/// `optional`, and `default-features` are meaningful local overrides (see
/// [`extract_local_overrides`]). Any local `version`, `path`, `git`, `branch`,
/// `tag`, `rev`, `target`, or `package` field is silently discarded — matching
/// `cargo`'s behaviour where specifying these alongside `workspace = true` is a
/// hard error. We drop them silently rather than erroring because ops processes
/// manifests for reporting, not for build-graph fidelity, and erroring here would
/// prevent introspection of technically-invalid-but-readable manifests.
fn resolve_dep_from_workspace(
    name: &str,
    local: &DepSpec,
    ws_deps: &BTreeMap<String, DepSpec>,
    section: &'static str,
) -> Result<DepSpec, InheritanceError> {
    let ws_dep = ws_deps
        .get(name)
        .ok_or_else(|| InheritanceError::MissingWorkspaceDependency {
            name: name.to_string(),
            section,
        })?;

    let resolved = match ws_dep {
        DepSpec::Simple(v) => resolve_from_simple_dep(v, local),
        DepSpec::Detailed(d) => resolve_from_detailed_dep(d, local),
    };

    Ok(DepSpec::Detailed(resolved))
}

/// Resolve a member dep against a workspace dep given as a bare version
/// string. Only the four fields this resolver actually decides are listed;
/// everything else — the source fields the doc comment on
/// [`resolve_dep_from_workspace`] says are deliberately discarded, plus
/// `workspace`, `package` and `target` — falls through to
/// [`DetailedDepSpec::default`].
///
/// DUP-7 / TASK-1804: restating the nine default fields here made
/// `DetailedDepSpec` exhaustively constructed in three places, so a new
/// cargo dependency key had to be given a value three times with no single
/// place stating the default.
fn resolve_from_simple_dep(version: &str, local: &DepSpec) -> DetailedDepSpec {
    let (local_features, local_optional, local_default_features) = extract_local_overrides(local);
    DetailedDepSpec {
        version: Some(version.to_string()),
        features: local_features,
        optional: local_optional,
        default_features: local_default_features,
        ..DetailedDepSpec::default()
    }
}

/// Merge a workspace `DetailedDepSpec` with a local override, mirroring
/// cargo's workspace-inheritance precedence:
///
/// - **features**: union of workspace + local (additive; cargo never lets a
///   member subtract features its workspace requested).
/// - **optional**: `ws.optional || local_optional`. Cargo treats `optional`
///   as "either side may turn this on, neither side may turn it off"; a
///   workspace dep marked `optional = true` stays optional even if the
///   member omits the flag, and a member can opt-in locally when the
///   workspace did not.
/// - **`default_features`**: `ws.default_features && local_default_features`.
///   Cargo's documented footgun: once the workspace sets
///   `default-features = false`, members **cannot** re-enable them with
///   `default-features = true` (cargo emits a warning and keeps defaults
///   off). The AND fold reproduces that behavior.
///
/// AC for TASK-0555: this is the rule the resolver implements; deviations
/// from cargo's actual precedence (e.g. cargo > 1.71's edge cases) are not
/// modeled because the resolver consumes manifests for reporting, not for
/// build-graph fidelity.
///
/// DUP-7 / TASK-1804: unlike [`resolve_from_simple_dep`], this constructor
/// stays exhaustive on purpose. Every field except `workspace` is *copied
/// from the workspace spec*, so the exhaustive literal is the compile-time
/// guard that a newly added cargo dependency key is consciously propagated
/// here rather than silently defaulted away by a `..Default::default()`.
fn resolve_from_detailed_dep(ws: &DetailedDepSpec, local: &DepSpec) -> DetailedDepSpec {
    let (local_features, local_optional, local_default_features) = extract_local_overrides(local);
    DetailedDepSpec {
        version: ws.version.clone(),
        path: ws.path.clone(),
        git: ws.git.clone(),
        branch: ws.branch.clone(),
        tag: ws.tag.clone(),
        rev: ws.rev.clone(),
        features: merge_features(&ws.features, &local_features),
        optional: ws.optional || local_optional,
        default_features: ws.default_features && local_default_features,
        workspace: None,
        package: ws.package.clone(),
        target: ws.target.clone(),
    }
}

fn extract_local_overrides(local: &DepSpec) -> (Vec<String>, bool, bool) {
    match local {
        DepSpec::Simple(_) => (vec![], false, true),
        DepSpec::Detailed(d) => {
            if d.version.is_some() || d.path.is_some() || d.git.is_some() {
                tracing::debug!(
                    version = ?d.version,
                    path = ?d.path,
                    git = ?d.git,
                    "workspace-inherited dep has local source overrides that will be discarded"
                );
            }
            (d.features.clone(), d.optional, d.default_features)
        }
    }
}

fn merge_features(base: &[String], additional: &[String]) -> Vec<String> {
    // PERF-2 (TASK-0807): feature lists are typically tiny (<10 entries), so a
    // linear scan beats allocating + hashing into a HashSet just to dedup. The
    // merge is order-preserving (base first, then new entries from
    // `additional`).
    let mut merged = base.to_vec();
    for f in additional {
        if !merged.iter().any(|m| m == f) {
            merged.push(f.clone());
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TASK-0385: `workspace = false` is parseable but cargo rejects it. Our
    /// resolver permissively ignores it: the field stays `Inherited { false }`
    /// and is treated as unresolved (no value substituted from the workspace).
    #[test]
    fn resolve_string_field_workspace_false_is_ignored() {
        let mut field: InheritableString = InheritableField::Inherited { workspace: false };
        resolve_string_field(&mut field, Some(&"1.0.0".to_string()));
        assert_eq!(
            field,
            InheritableField::Inherited { workspace: false },
            "workspace=false should not pull in a value"
        );
    }

    /// TASK-0961: when the workspace did not declare `keywords` (parsed as an
    /// empty Vec), an inheriting member must remain `Inherited`, not be
    /// overwritten with an empty `Value`.
    #[test]
    fn resolve_vec_field_empty_ws_leaves_inherited_unchanged() {
        let mut field: InheritableVec = InheritableField::Inherited { workspace: true };
        resolve_vec_field(&mut field, &[]);
        assert_eq!(
            field,
            InheritableField::Inherited { workspace: true },
            "an empty workspace vec should not substitute"
        );
    }

    #[test]
    fn resolve_vec_field_non_empty_ws_substitutes() {
        let mut field: InheritableVec = InheritableField::Inherited { workspace: true };
        resolve_vec_field(&mut field, &["cli".to_string(), "tool".to_string()]);
        assert_eq!(
            field,
            InheritableField::Value(vec!["cli".to_string(), "tool".to_string()]),
            "a non-empty workspace vec should substitute"
        );
    }

    #[test]
    fn resolve_string_field_workspace_true_substitutes() {
        let mut field: InheritableString = InheritableField::Inherited { workspace: true };
        resolve_string_field(&mut field, Some(&"1.0.0".to_string()));
        assert_eq!(
            field,
            InheritableField::Value("1.0.0".to_string()),
            "workspace=true should substitute"
        );
    }
}
