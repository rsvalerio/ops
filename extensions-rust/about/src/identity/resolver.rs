//! Resolve inheritable identity fields from `[package]` / `[workspace.package]`.

use std::path::Path;

/// Resolved inheritable fields from `[package]` / `[workspace.package]`.
pub(super) struct ResolvedFields {
    pub version: Option<String>,
    pub description: Option<String>,
    pub edition: Option<String>,
    pub license: Option<String>,
    pub repository: Option<String>,
    pub homepage: Option<String>,
    pub msrv: Option<String>,
    pub authors: Vec<String>,
}

/// Resolves a string field by trying `[package]` first, then falling back to `[workspace.package]`.
fn resolve_field(
    pkg: Option<&ops_cargo_toml::Package>,
    ws_pkg: Option<&ops_cargo_toml::WorkspacePackage>,
    pkg_getter: impl Fn(&ops_cargo_toml::Package) -> Option<&str>,
    ws_getter: impl Fn(&ops_cargo_toml::WorkspacePackage) -> Option<&str>,
) -> Option<String> {
    pkg.and_then(&pkg_getter)
        .or_else(|| ws_pkg.and_then(&ws_getter))
        .map(std::string::ToString::to_string)
}

pub(super) fn resolve_identity_fields(
    pkg: Option<&ops_cargo_toml::Package>,
    ws_pkg: Option<&ops_cargo_toml::WorkspacePackage>,
    cwd: &Path,
) -> ResolvedFields {
    macro_rules! r {
        ($pg:ident, $wg:ident) => {
            resolve_field(pkg, ws_pkg, |p| p.$pg.as_str(), |wp| wp.$wg.as_deref())
        };
    }

    let repository = ops_git::resolve_repository_with_git_fallback(cwd, r!(repository, repository));

    let authors = pkg
        .and_then(|p| p.authors.value())
        .cloned()
        .or_else(|| {
            ws_pkg
                .filter(|wp| !wp.authors.is_empty())
                .map(|wp| wp.authors.clone())
        })
        .unwrap_or_default();

    ResolvedFields {
        version: r!(version, version),
        description: r!(description, description),
        edition: r!(edition, edition),
        license: r!(license, license),
        repository,
        homepage: r!(homepage, homepage),
        msrv: r!(rust_version, rust_version),
        authors,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_cargo_toml::CargoToml;

    fn parse_pkg(toml_str: &str) -> CargoToml {
        toml::from_str(toml_str).expect("test toml should parse")
    }

    #[test]
    fn resolve_field_prefers_package_over_workspace() {
        let manifest = parse_pkg(
            r#"
[package]
name = "test"
version = "1.0.0"
description = "pkg desc"

[workspace.package]
description = "ws desc"
"#,
        );
        let pkg = manifest.package.as_ref();
        let ws_pkg = manifest.workspace.as_ref().and_then(|w| w.package.as_ref());
        let result = resolve_field(
            pkg,
            ws_pkg,
            |p| p.description.as_str(),
            |wp| wp.description.as_deref(),
        );
        assert_eq!(result.as_deref(), Some("pkg desc"));
    }

    #[test]
    fn resolve_field_falls_back_to_workspace_when_no_package() {
        let manifest = parse_pkg(
            r#"
[workspace.package]
description = "ws desc"
"#,
        );
        let ws_pkg = manifest.workspace.as_ref().and_then(|w| w.package.as_ref());
        let result = resolve_field(
            None,
            ws_pkg,
            |p: &ops_cargo_toml::Package| p.description.as_str(),
            |wp| wp.description.as_deref(),
        );
        assert_eq!(result.as_deref(), Some("ws desc"));
    }

    #[test]
    fn resolve_field_returns_none_when_both_none() {
        let result: Option<String> = resolve_field(
            None,
            None,
            |p: &ops_cargo_toml::Package| p.description.as_str(),
            |wp: &ops_cargo_toml::WorkspacePackage| wp.description.as_deref(),
        );
        assert!(result.is_none());
    }

    #[test]
    fn resolve_field_no_package_uses_workspace() {
        let manifest = parse_pkg(
            r#"
[workspace.package]
version = "2.0.0"
"#,
        );
        let ws_pkg = manifest.workspace.as_ref().and_then(|w| w.package.as_ref());
        let result = resolve_field(
            None,
            ws_pkg,
            |p: &ops_cargo_toml::Package| p.version.as_str(),
            |wp| wp.version.as_deref(),
        );
        assert_eq!(result.as_deref(), Some("2.0.0"));
    }

    #[test]
    fn resolve_field_no_package_no_workspace() {
        let result: Option<String> = resolve_field(
            None,
            None,
            |p: &ops_cargo_toml::Package| p.version.as_str(),
            |wp: &ops_cargo_toml::WorkspacePackage| wp.version.as_deref(),
        );
        assert!(result.is_none());
    }

    /// ERR-6 / TASK-1793: a `[package]` that *exists* but omits the field
    /// must still fall through to `[workspace.package]`.
    ///
    /// This is the cross-crate consequence of the empty-string sentinel:
    /// `InheritableField::default()` used to be `Value("")`, so
    /// `p.version.as_str()` returned `Some("")` for an omitted key and the
    /// `.or_else(...)` fallback never fired — `about` reported an empty
    /// version and description instead of the workspace's values. Note the
    /// `authors` arm in `resolve_identity_fields` had to hand-roll a
    /// `!wp.authors.is_empty()` guard for exactly this reason.
    #[test]
    fn resolve_field_falls_back_to_workspace_when_package_omits_the_key() {
        let manifest = parse_pkg(
            r#"
[package]
name = "member"

[workspace.package]
version = "2.0.0"
description = "ws desc"
"#,
        );
        let pkg = manifest.package.as_ref();
        let ws_pkg = manifest.workspace.as_ref().and_then(|w| w.package.as_ref());

        assert_eq!(
            resolve_field(
                pkg,
                ws_pkg,
                |p: &ops_cargo_toml::Package| p.version.as_str(),
                |wp| wp.version.as_deref(),
            )
            .as_deref(),
            Some("2.0.0")
        );
        assert_eq!(
            resolve_field(
                pkg,
                ws_pkg,
                |p: &ops_cargo_toml::Package| p.description.as_str(),
                |wp| wp.description.as_deref(),
            )
            .as_deref(),
            Some("ws desc")
        );
    }

    /// ERR-6 / TASK-1793: the guard rail for the test above — a member that
    /// *declares* an empty string keeps it, rather than silently inheriting.
    #[test]
    fn resolve_field_declared_empty_string_beats_workspace() {
        let manifest = parse_pkg(
            r#"
[package]
name = "member"
description = ""

[workspace.package]
description = "ws desc"
"#,
        );
        let result = resolve_field(
            manifest.package.as_ref(),
            manifest.workspace.as_ref().and_then(|w| w.package.as_ref()),
            |p: &ops_cargo_toml::Package| p.description.as_str(),
            |wp| wp.description.as_deref(),
        );
        assert_eq!(result.as_deref(), Some(""));
    }
}
