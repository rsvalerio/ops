//! Terraform stack `project_identity` provider.
//!
//! Parses `.tf` files for `required_version` constraints and counts local
//! modules under `modules/*/`. No terraform subprocess — purely filesystem.
//!
//! The crate root holds only the extension and provider wiring. Filesystem
//! discovery (which `.tf` files to read, which `modules/` children count)
//! lives in `discovery`, and the hand-written HCL lexer that locates
//! `required_version` in one file's text lives in `hcl_scan`.
//!
//! # Manifest IO policy
//!
//! Every `.tf` read goes through
//! [`ops_about::manifest_io::read_optional_text`], so the project-wide rule
//! "missing manifest is silent, real IO error is `tracing::warn!`-and-fall-
//! back" applies here the same way it does in the Python / Go siblings.
//! A permission-denied / EIO / "is a directory" failure on `versions.tf`
//! is therefore distinguishable from "no version declared" in the logs.
//! Directory enumeration follows the same policy through
//! `discovery::read_dir_logged`: a missing directory is silent, while any other
//! `read_dir` failure, and a failure reading a single entry, is logged at
//! `warn`.
//!
//! # Rendered values are untrusted
//!
//! `ops about` runs inside repositories the operator cloned but did not
//! audit, and `stack_detail` reaches the terminal with no escaping layer in
//! between. `hcl_scan::sanitize_required_version` is the single producing-side gate:
//! a value carrying a control or Unicode formatting codepoint is dropped, not
//! stripped, matching `ops_about::text_util`'s policy for manifest URL and
//! repository fields.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

mod discovery;
mod hcl_scan;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{base_about_fields, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};

const NAME: &str = "about-terraform";
const DESCRIPTION: &str = "Terraform project identity";
const SHORTNAME: &str = "about-terraform";
const DATA_PROVIDER_NAME: &str = "project_identity";

/// Datasource extension supplying the Terraform stack's about providers
/// (identity, read from `.tf` files) to the generic `ops_about` renderers.
#[non_exhaustive]
pub struct AboutTerraformExtension;

ops_extension::impl_extension! {
    AboutTerraformExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_core::stack::Stack::Terraform),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(TerraformIdentityProvider));
    },
    factory: TERRAFORM_ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutTerraformExtension)))
    },
}

struct TerraformIdentityProvider;

impl DataProvider for TerraformIdentityProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        base_about_fields()
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        provide_identity_from_manifest(ctx.working_directory(), |root| {
            let required_version = discovery::find_required_version(root);
            let module_count = discovery::count_local_modules(root);

            let stack_detail = required_version.map(|v| format!("Terraform {v}"));

            ParsedManifest::build(|m| {
                m.stack_label = "Terraform";
                m.stack_detail = stack_detail;
                m.module_label = "modules";
                m.module_count = module_count;
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // The `write` fixture helper is shared with the sibling about crates, so
    // a future tightening (error propagation, a setup message) lands once
    // rather than four times.
    use ops_about::test_support::write_file as write;
    use ops_core::project_identity::ProjectIdentity;
    use ops_core::stack::Stack;
    use ops_extension::{DataRegistry, Extension};

    // The crate's public surface is the `Extension` impl, so it is exercised
    // directly here: a drift between `DATA_PROVIDER_NAME` and the
    // registration key produces an extension that compiles, links into the
    // `linkme` registry and silently provides nothing.
    ops_extension::test_datasource_extension!(
        AboutTerraformExtension,
        name: "about-terraform",
        data_provider: "project_identity"
    );

    /// The other half of the host contract: the metadata the extension host
    /// dispatches on.
    #[test]
    fn extension_metadata_matches_contract() {
        let ext = AboutTerraformExtension;
        assert_eq!(ext.description(), "Terraform project identity");
        assert_eq!(ext.shortname(), "about-terraform");
        assert_eq!(ext.types(), ExtensionType::DATASOURCE);
        assert_eq!(ext.stack(), Some(Stack::Terraform));
        assert_eq!(ext.data_provider_name(), Some("project_identity"));
    }

    /// The registered provider must be the terraform one,
    /// not merely *a* provider under the right key.
    #[test]
    fn registered_provider_is_the_terraform_identity_provider() {
        let mut registry = DataRegistry::new();
        AboutTerraformExtension.register_data_providers(&mut registry);
        let provider = registry.get("project_identity").expect("provider");

        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("versions.tf"), "terraform {\n  required_version = \">= 1.5\"\n}\n");
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).expect("provide");
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();
        assert_eq!(id.stack_label, "Terraform");
        assert_eq!(id.stack_detail.as_deref(), Some("Terraform >= 1.5"));
    }

    #[test]
    fn provider_name() {
        let provider = TerraformIdentityProvider;
        assert_eq!(provider.name(), "project_identity");
    }

    /// The terraform provider exposes exactly the base about fields, in the
    /// base order — no stack-specific extras and none missing. The expected
    /// ids are pinned here rather than read back from `base_about_fields()`
    /// (which the implementation delegates to), so a change on either side
    /// of that delegation fails this test.
    #[test]
    fn about_fields_match_base() {
        let provider = TerraformIdentityProvider;
        let fields = provider.about_fields();
        let ids: Vec<&str> = fields.iter().map(|f| f.id).collect();
        assert_eq!(
            ids,
            vec![
                "stack",
                "license",
                "project",
                "modules",
                "codebase",
                "repository",
                "authors",
                "coverage",
            ]
        );
    }

    #[test]
    fn provide_simple_terraform_project() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("main.tf"), "resource \"null_resource\" \"test\" {}\n");

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        let expected = dir.path().file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(id.name, expected);
        assert_eq!(id.stack_label, "Terraform");
        assert_eq!(id.module_label, "modules");
        assert!(id.stack_detail.is_none());
    }

    #[test]
    fn provide_with_required_version() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("versions.tf"),
            r#"terraform {
  required_version = ">= 1.5"
}
"#,
        );
        write(&dir.path().join("main.tf"), "");

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.stack_detail.as_deref(), Some("Terraform >= 1.5"));
    }

    #[test]
    fn provide_with_modules() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("main.tf"), "");
        write(&dir.path().join("modules").join("api").join("main.tf"), "");
        write(&dir.path().join("modules").join("network").join("main.tf"), "");
        // Not a module (no main.tf)
        std::fs::create_dir_all(dir.path().join("modules").join("empty")).unwrap();

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.module_count, Some(2));
    }

    #[test]
    fn provide_no_manifest_falls_back_to_dir_name() {
        let dir = tempfile::tempdir().unwrap();

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        let expected = dir.path().file_name().unwrap().to_string_lossy().to_string();
        assert_eq!(id.name, expected);
        assert_eq!(id.stack_label, "Terraform");
        assert!(id.module_count.is_none());
        assert!(id.stack_detail.is_none());
    }

    #[test]
    fn provide_populates_repository_from_git_remote() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("main.tf"), "");
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/o/r.git\n",
        )
        .unwrap();

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.repository.as_deref(), Some("https://github.com/o/r"));
    }
}
