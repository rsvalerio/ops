//! Maven `project_identity` provider — wires the [`pom`] parser to a
//! [`DataProvider`] that emits a [`ProjectIdentity`] for the current
//! workspace.

mod pom;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::AboutFieldDef;
use ops_extension::{Context, DataProvider, DataProviderError};

use super::java_about_fields;
use pom::parse_pom_xml;

pub(crate) struct MavenIdentityProvider;

impl DataProvider for MavenIdentityProvider {
    fn name(&self) -> &'static str {
        "project_identity"
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        java_about_fields()
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        provide_identity_from_manifest(ctx.working_directory.as_path(), |root| {
            let pom = parse_pom_xml(root).unwrap_or_default();
            let module_count = (!pom.modules.is_empty()).then_some(pom.modules.len());

            ParsedManifest::build(|m| {
                m.name = pom.name.or(pom.artifact_id);
                m.version = pom.version;
                m.description = pom.description;
                m.license = pom.license;
                m.authors = pom.developers;
                m.repository = pom.scm_url;
                m.stack_label = "Java";
                m.stack_detail = Some("Maven".to_string());
                m.module_label = "modules";
                m.module_count = module_count;
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_provider_name() {
        assert_eq!(MavenIdentityProvider.name(), "project_identity");
    }

    #[test]
    fn maven_provider_about_fields() {
        let fields = MavenIdentityProvider.about_fields();
        assert!(!fields.is_empty());
    }

    #[test]
    fn maven_provider_provide_no_pom() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let result = MavenIdentityProvider.provide(&mut ctx).unwrap();

        let name = result["name"].as_str().unwrap();
        assert!(!name.is_empty());
        assert_eq!(result["stack_detail"], "Maven");
        assert!(result["version"].is_null());
    }

    #[test]
    fn maven_provider_provide_uses_dir_name_fallback() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <version>1.0</version>\n</project>",
        )
        .unwrap();

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let result = MavenIdentityProvider.provide(&mut ctx).unwrap();

        let name = result["name"].as_str().unwrap();
        assert!(!name.is_empty());
    }

    /// Provider-specific shape: empty modules yields null module_count, no
    /// homepage is set, and stack_detail is always "Maven". Parser coverage
    /// lives in `pom::tests::parse_pom_basic`.
    #[test]
    fn maven_provider_provide_shape() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <artifactId>testapp</artifactId>\n</project>",
        )
        .unwrap();

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let result = MavenIdentityProvider.provide(&mut ctx).unwrap();

        assert_eq!(result["stack_detail"], "Maven");
        assert!(result["module_count"].is_null());
        assert!(result["homepage"].is_null());
    }
}
