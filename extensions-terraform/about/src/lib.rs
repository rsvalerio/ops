//! Terraform stack `project_identity` provider.
//!
//! Parses `.tf` files for `required_version` constraints and counts local
//! modules under `modules/*/`. No terraform subprocess — purely filesystem.

use std::path::Path;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{base_about_fields, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};

const NAME: &str = "about-terraform";
const DESCRIPTION: &str = "Terraform project identity";
const SHORTNAME: &str = "about-terraform";
const DATA_PROVIDER_NAME: &str = "project_identity";

#[non_exhaustive]
pub struct AboutTerraformExtension;

ops_extension::impl_extension! {
    AboutTerraformExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Terraform),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(TerraformIdentityProvider));
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
        provide_identity_from_manifest(ctx.working_directory.as_path(), |root| {
            let required_version = find_required_version(root);
            let module_count = count_local_modules(root);

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

/// Scan `.tf` files for `required_version` in a `terraform` block.
///
/// Looks for patterns like `required_version = ">= 1.5"` or
/// `required_version = "~> 1.0"`. Only the first match is used.
fn find_required_version(root: &Path) -> Option<String> {
    let candidates = ["versions.tf", "main.tf", "terraform.tf", "version.tf"];
    for candidate in candidates {
        let path = root.join(candidate);
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Some(v) = extract_required_version(&content) {
                return Some(v);
            }
        }
    }
    // Scan all .tf files as fallback
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "tf") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Some(v) = extract_required_version(&content) {
                        return Some(v);
                    }
                }
            }
        }
    }
    None
}

/// Extract `required_version` value from a single `.tf` file's content.
fn extract_required_version(content: &str) -> Option<String> {
    // Look for: required_version = "..."
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("required_version") {
            let rest = rest.trim();
            if let Some(rest) = rest.strip_prefix('=') {
                let rest = rest.trim();
                // Strip quotes
                let v = rest.trim_matches('"').trim();
                if !v.is_empty() {
                    return Some(v.to_string());
                }
            }
        }
    }
    None
}

/// Count local modules under `modules/*/main.tf`.
fn count_local_modules(root: &Path) -> Option<usize> {
    let modules_dir = root.join("modules");
    let Ok(entries) = std::fs::read_dir(&modules_dir) else {
        return None;
    };
    let count = entries
        .flatten()
        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|e| e.path().join("main.tf").exists())
        .count();
    if count > 0 {
        Some(count)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::ProjectIdentity;

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn provider_name() {
        let provider = TerraformIdentityProvider;
        assert_eq!(provider.name(), "project_identity");
    }

    #[test]
    fn about_fields_match_base() {
        let provider = TerraformIdentityProvider;
        let fields = provider.about_fields();
        let base = base_about_fields();
        assert_eq!(fields.len(), base.len());
        for (a, b) in fields.iter().zip(base.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn provide_simple_terraform_project() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("main.tf"),
            "resource \"null_resource\" \"test\" {}\n",
        );

        let provider = TerraformIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        let expected = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
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
        write(
            &dir.path().join("modules").join("network").join("main.tf"),
            "",
        );
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

        let expected = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
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

    #[test]
    fn extract_required_version_from_content() {
        let content = r#"terraform {
  required_version = "~> 1.0"
}
"#;
        assert_eq!(
            extract_required_version(content),
            Some("~> 1.0".to_string())
        );
    }

    #[test]
    fn extract_required_version_skips_comments() {
        let content = r#"# required_version = "skip"
// required_version = "also skip"
required_version = ">= 1.5"
"#;
        assert_eq!(
            extract_required_version(content),
            Some(">= 1.5".to_string())
        );
    }

    #[test]
    fn extract_required_version_none_when_absent() {
        assert_eq!(
            extract_required_version("resource \"test\" \"x\" {}\n"),
            None
        );
    }
}
