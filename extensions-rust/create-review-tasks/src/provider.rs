//! Rust-specific `review_targets` data provider.
//!
//! One target per resolved `[workspace].members` entry (the same member view
//! the about providers use), named by cargo package name with a display-name
//! fallback for members whose manifest cannot be read or parsed.

use ops_about::cards::format_unit_name;
use ops_cargo_toml::{find_workspace_root_strict, CargoToml, CargoTomlProvider};
use ops_extension::{Context, DataProvider, DataProviderError};

/// Review skill the subtask titles reference.
pub const SKILL_NAME: &str = "code-review-rust";

pub struct RustReviewTargetsProvider;

impl DataProvider for RustReviewTargetsProvider {
    fn name(&self) -> &'static str {
        ops_create_review_tasks::DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let root = find_workspace_root_strict(ctx.working_directory.as_path())
            .map_err(DataProviderError::computation_error)?;
        let manifest = CargoTomlProvider::with_root(root.clone()).provide_typed(ctx)?;
        let members = ops_about_rust::resolved_workspace_members(&manifest, &root);

        let targets: Vec<serde_json::Value> = members
            .iter()
            .map(|member| {
                let member_toml = root.join(member).join("Cargo.toml");
                let name = member_package_name(&member_toml).unwrap_or_else(|| {
                    tracing::warn!(
                        member = %member,
                        "no package name for workspace member; falling back to display name"
                    );
                    format_unit_name(member)
                });
                serde_json::json!({ "name": name, "path": member })
            })
            .collect();

        serde_json::to_value(serde_json::json!({
            "skill": SKILL_NAME,
            "targets": targets,
        }))
        .map_err(DataProviderError::from)
    }
}

/// Package name of one workspace member's manifest, or `None` when the
/// manifest is absent, unreadable, unparseable, or has no `[package].name`.
///
/// Follows the read-log policy of the about units provider: `NotFound` reads
/// are silent, other read errors log at `debug`, parse errors at `warn`, and
/// tracing path fields use the `?` formatter so attacker-controlled member
/// names cannot forge log records.
fn member_package_name(member_toml: &std::path::Path) -> Option<String> {
    let content = match ops_core::text::read_capped_to_string(member_toml) {
        Ok(content) => content,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            tracing::debug!(
                path = ?member_toml.display(),
                error = ?e,
                "failed to read crate manifest"
            );
            return None;
        }
    };
    match CargoToml::parse(&content) {
        Ok(parsed) => parsed.package_name().map(str::to_string),
        Err(e) => {
            tracing::warn!(
                path = ?member_toml.display(),
                error = ?e,
                "failed to parse crate manifest as TOML"
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a scratch Cargo workspace: root manifest with `members` plus
    /// one member crate per (dir, package-name) pair. Members whose name is
    /// `None` get no manifest at all.
    fn scratch_workspace(
        members: &str,
        crates: &[(&str, Option<&str>)],
    ) -> (tempfile::TempDir, std::path::PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path().to_path_buf();
        std::fs::write(
            root.join("Cargo.toml"),
            format!("[workspace]\nmembers = [{members}]\n"),
        )
        .expect("root manifest");
        for (dir_name, pkg_name) in crates {
            let crate_dir = root.join(dir_name);
            std::fs::create_dir_all(&crate_dir).expect("member dir");
            if let Some(pkg) = pkg_name {
                std::fs::write(
                    crate_dir.join("Cargo.toml"),
                    format!("[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\n"),
                )
                .expect("member manifest");
            }
        }
        (dir, root)
    }

    fn provide(root: &std::path::Path) -> Result<serde_json::Value, DataProviderError> {
        let mut ctx = Context::test_context(root.to_path_buf());
        RustReviewTargetsProvider.provide(&mut ctx)
    }

    #[test]
    fn payload_lists_every_member_with_package_names() {
        let (_dir, root) = scratch_workspace(
            "\"crates/*\"",
            &[
                ("crates/core", Some("ops-core")),
                ("crates/cli", Some("ops-cli")),
                ("crates/not-a-crate", None),
            ],
        );
        let value = provide(&root).expect("provide");
        assert_eq!(
            value,
            serde_json::json!({
                "skill": "code-review-rust",
                "targets": [
                    { "name": "ops-cli", "path": "crates/cli" },
                    { "name": "ops-core", "path": "crates/core" }
                ]
            })
        );
    }

    /// A member without a usable manifest must not disappear — it falls back
    /// to the capitalized display name so it still gets a review subtask.
    #[test]
    fn member_without_manifest_falls_back_to_display_name() {
        let (_dir, root) = scratch_workspace("\"crates/orphan\"", &[("crates/orphan", None)]);
        let value = provide(&root).expect("provide");
        assert_eq!(
            value["targets"][0],
            serde_json::json!({ "name": "Orphan", "path": "crates/orphan" })
        );
    }

    #[test]
    fn member_with_malformed_manifest_falls_back_to_display_name() {
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/broken\"]\n",
        )
        .expect("root manifest");
        std::fs::create_dir_all(root.join("crates/broken")).expect("member dir");
        std::fs::write(
            root.join("crates/broken/Cargo.toml"),
            "[package\nname = \"unterminated\n",
        )
        .expect("malformed manifest");
        let value = provide(root).expect("provide");
        assert_eq!(
            value["targets"][0],
            serde_json::json!({ "name": "Broken", "path": "crates/broken" })
        );
    }

    /// No Cargo.toml anywhere at or above the start: the provider must
    /// surface the typed workspace-root error, not an empty target list
    /// (which the engine would misread as "nothing to review").
    #[test]
    fn missing_workspace_manifest_is_an_error() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = provide(dir.path());
        let err = result.expect_err("must fail");
        assert!(
            err.to_string().contains("Cargo.toml"),
            "error must mention the missing manifest, got: {err:#}"
        );
    }
}
