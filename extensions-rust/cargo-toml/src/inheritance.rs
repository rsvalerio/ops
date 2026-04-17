//! Workspace inheritance resolution for Cargo.toml manifests.
//!
//! Resolves `{ workspace = true }` fields and dependencies by merging from the
//! workspace section. Called by [`CargoToml::resolve_inheritance`] and
//! [`CargoToml::resolve_package_inheritance`].

use std::collections::BTreeMap;

use crate::types::{CargoToml, DepSpec, DetailedDepSpec, InheritableField, InheritableString};

/// Error during workspace inheritance resolution.
#[derive(Debug, Clone, thiserror::Error)]
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

        resolve_string_field(&mut pkg.version, &ws_pkg.version);
        resolve_string_field(&mut pkg.edition, &ws_pkg.edition);
        resolve_string_field(&mut pkg.rust_version, &ws_pkg.rust_version);
        resolve_string_field(&mut pkg.description, &ws_pkg.description);
        resolve_string_field(&mut pkg.documentation, &ws_pkg.documentation);
        resolve_string_field(&mut pkg.homepage, &ws_pkg.homepage);
        resolve_string_field(&mut pkg.repository, &ws_pkg.repository);
        resolve_string_field(&mut pkg.license, &ws_pkg.license);

        if let InheritableField::Inherited { workspace: true } = &pkg.authors {
            pkg.authors = InheritableField::Value(ws_pkg.authors.clone());
        }
    }
}

pub(crate) fn resolve_string_field(field: &mut InheritableString, ws_value: &Option<String>) {
    if let InheritableField::Inherited { workspace: true } = field {
        if let Some(v) = ws_value {
            *field = InheritableField::Value(v.clone());
        }
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
    let mut seen: std::collections::HashSet<&str> = base.iter().map(|s| s.as_str()).collect();
    let mut merged = base.to_vec();
    for f in additional {
        if seen.insert(f.as_str()) {
            merged.push(f.clone());
        }
    }
    merged
}
