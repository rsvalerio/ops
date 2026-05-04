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
    #[error("dependency '{name}' not found in workspace.dependencies")]
    MissingWorkspaceDependency { name: String },
}

impl CargoToml {
    /// Merges inherited dependencies (workspace = true) with workspace definitions.
    ///
    /// After calling this, all dependencies with `workspace = true` will have
    /// their values filled from `workspace.dependencies`.
    pub fn resolve_inheritance(&mut self) -> Result<(), InheritanceError> {
        let Some(ws) = &self.workspace else {
            return Ok(());
        };

        let ws_deps = &ws.dependencies;

        resolve_deps_inheritance(&mut self.dependencies, ws_deps)?;
        resolve_deps_inheritance(&mut self.dev_dependencies, ws_deps)?;
        resolve_deps_inheritance(&mut self.build_dependencies, ws_deps)?;

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
        resolve_string_field(&mut pkg.version, &ws_pkg.version);
        resolve_string_field(&mut pkg.edition, &ws_pkg.edition);
        resolve_string_field(&mut pkg.rust_version, &ws_pkg.rust_version);
        resolve_string_field(&mut pkg.description, &ws_pkg.description);
        resolve_string_field(&mut pkg.documentation, &ws_pkg.documentation);
        resolve_string_field(&mut pkg.homepage, &ws_pkg.homepage);
        resolve_string_field(&mut pkg.repository, &ws_pkg.repository);
        resolve_string_field(&mut pkg.license, &ws_pkg.license);

        resolve_vec_field(&mut pkg.keywords, &ws_pkg.keywords);
        resolve_vec_field(&mut pkg.categories, &ws_pkg.categories);

        if let InheritableField::Inherited { workspace: true } = &pkg.authors {
            pkg.authors = InheritableField::Value(ws_pkg.authors.clone());
        }

        resolve_optional_string(&mut pkg.license_file, &ws_pkg.license_file);
        resolve_readme(&mut pkg.readme, &ws_pkg.readme);
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
pub(crate) fn resolve_string_field(field: &mut InheritableString, ws_value: &Option<String>) {
    if let InheritableField::Inherited { workspace: true } = field {
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
pub(crate) fn resolve_vec_field(field: &mut InheritableVec, ws_value: &[String]) {
    if let InheritableField::Inherited { workspace: true } = field {
        if !ws_value.is_empty() {
            *field = InheritableField::Value(ws_value.to_vec());
        }
    }
}

/// Resolve `license-file = { workspace = true }` against the workspace's
/// `license-file`. Mirrors [`resolve_string_field`] but for `Option<InheritableString>`.
pub(crate) fn resolve_optional_string(
    field: &mut Option<InheritableString>,
    ws_value: &Option<String>,
) {
    if let Some(inner) = field {
        resolve_string_field(inner, ws_value);
    }
}

/// Resolve `readme = { workspace = true }` against the workspace's `readme`.
pub(crate) fn resolve_readme(field: &mut Option<ReadmeSpec>, ws_value: &Option<ReadmeSpec>) {
    if let Some(ReadmeSpec::Inherited { workspace: true }) = field {
        if let Some(v) = ws_value {
            *field = Some(v.clone());
        }
    }
}

/// Resolve `publish = { workspace = true }` against the workspace's `publish`.
pub(crate) fn resolve_publish(field: &mut PublishSpec, ws_value: &PublishSpec) {
    if let PublishSpec::Inherited { workspace: true } = field {
        *field = ws_value.clone();
    }
}

fn resolve_deps_inheritance(
    deps: &mut BTreeMap<String, DepSpec>,
    ws_deps: &BTreeMap<String, DepSpec>,
) -> Result<(), InheritanceError> {
    for (name, dep) in deps {
        if dep.is_workspace_inherited() {
            *dep = resolve_dep_from_workspace(name, dep, ws_deps)?;
        }
    }
    Ok(())
}

fn resolve_dep_from_workspace(
    name: &str,
    local: &DepSpec,
    ws_deps: &BTreeMap<String, DepSpec>,
) -> Result<DepSpec, InheritanceError> {
    let ws_dep = ws_deps
        .get(name)
        .ok_or_else(|| InheritanceError::MissingWorkspaceDependency {
            name: name.to_string(),
        })?;

    let resolved = match ws_dep {
        DepSpec::Simple(v) => resolve_from_simple_dep(v, local),
        DepSpec::Detailed(d) => resolve_from_detailed_dep(d, local),
    };

    Ok(DepSpec::Detailed(resolved))
}

fn resolve_from_simple_dep(version: &str, local: &DepSpec) -> DetailedDepSpec {
    let (local_features, local_optional, local_default_features) = extract_local_overrides(local);
    DetailedDepSpec {
        version: Some(version.to_string()),
        path: None,
        git: None,
        branch: None,
        tag: None,
        rev: None,
        features: local_features,
        optional: local_optional,
        default_features: local_default_features,
        workspace: None,
        package: None,
        target: None,
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
/// - **default_features**: `ws.default_features && local_default_features`.
///   Cargo's documented footgun: once the workspace sets
///   `default-features = false`, members **cannot** re-enable them with
///   `default-features = true` (cargo emits a warning and keeps defaults
///   off). The AND fold reproduces that behavior.
///
/// AC for TASK-0555: this is the rule the resolver implements; deviations
/// from cargo's actual precedence (e.g. cargo > 1.71's edge cases) are not
/// modeled because the resolver consumes manifests for reporting, not for
/// build-graph fidelity.
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
        DepSpec::Detailed(d) => (d.features.clone(), d.optional, d.default_features),
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
        resolve_string_field(&mut field, &Some("1.0.0".to_string()));
        match field {
            InheritableField::Inherited { workspace } => assert!(!workspace),
            InheritableField::Value(_) => panic!("workspace=false should not pull in a value"),
        }
    }

    /// TASK-0961: when the workspace did not declare `keywords` (parsed as an
    /// empty Vec), an inheriting member must remain `Inherited`, not be
    /// overwritten with an empty `Value`.
    #[test]
    fn resolve_vec_field_empty_ws_leaves_inherited_unchanged() {
        let mut field: InheritableVec = InheritableField::Inherited { workspace: true };
        resolve_vec_field(&mut field, &[]);
        match field {
            InheritableField::Inherited { workspace } => assert!(workspace),
            InheritableField::Value(v) => panic!("empty ws should not substitute, got {v:?}"),
        }
    }

    #[test]
    fn resolve_vec_field_non_empty_ws_substitutes() {
        let mut field: InheritableVec = InheritableField::Inherited { workspace: true };
        resolve_vec_field(&mut field, &["cli".to_string(), "tool".to_string()]);
        match field {
            InheritableField::Value(v) => assert_eq!(v, vec!["cli", "tool"]),
            InheritableField::Inherited { .. } => panic!("non-empty ws should substitute"),
        }
    }

    #[test]
    fn resolve_string_field_workspace_true_substitutes() {
        let mut field: InheritableString = InheritableField::Inherited { workspace: true };
        resolve_string_field(&mut field, &Some("1.0.0".to_string()));
        match field {
            InheritableField::Value(v) => assert_eq!(v, "1.0.0"),
            InheritableField::Inherited { .. } => panic!("workspace=true should substitute"),
        }
    }
}
