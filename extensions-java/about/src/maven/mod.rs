//! Maven `project_identity` provider — wires the [`pom`] parser to a
//! [`DataProvider`] that emits a [`ops_core::project_identity::ProjectIdentity`] for the current
//! workspace.

mod pom;

use std::path::{Component, Path};

use ops_about::cards::format_unit_name;
use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{AboutFieldDef, ProjectUnit};
use ops_extension::{Context, DataProvider, DataProviderError};

use super::maven_about_fields;
use pom::parse_pom_xml;

pub struct MavenIdentityProvider;

impl DataProvider for MavenIdentityProvider {
    fn name(&self) -> &'static str {
        "project_identity"
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        maven_about_fields()
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        provide_identity_from_manifest(ctx.working_directory(), |root| {
            let pom = parse_pom_xml(root).unwrap_or_default();
            let module_count = (!pom.modules.is_empty()).then_some(pom.modules.len());

            ParsedManifest::build(|m| {
                m.name = pom.name.or(pom.artifact_id);
                m.version = pom.version;
                m.description = pom.description;
                m.license = pom.license;
                m.authors = pom.developers;
                // TASK-2204: the POM's top-level `<url>` is the project
                // homepage; `<scm><url>` is the repository. Mapping the
                // former to `homepage` (not `repository`) lets a POM that
                // declares only `<url>` keep the git-remote fallback for
                // the repository instead of mislabelling its homepage.
                m.homepage = pom.project_url;
                m.repository = pom.scm_url;
                m.stack_label = "Java";
                m.stack_detail = Some("Maven".to_string());
                m.module_label = "modules";
                m.module_count = module_count;
            })
        })
    }
}

/// TASK-2207: the Maven `project_units` provider. The identity card counts
/// `<modules><module>` entries, so the units page must list exactly those
/// modules — one `ProjectUnit` per `<module>`, in declaration order — or the
/// card's "N modules" and the units table disagree by construction.
pub struct MavenUnitsProvider;

impl DataProvider for MavenUnitsProvider {
    fn name(&self) -> &'static str {
        "project_units"
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let units = collect_units(ctx.working_directory());
        serde_json::to_value(&units).map_err(DataProviderError::from)
    }
}

/// Build one [`ProjectUnit`] per `<modules><module>` entry. Every entry
/// becomes a unit — including one whose child `pom.xml` is missing or whose
/// path is hostile — so `units.len()` always equals the identity provider's
/// `module_count` (which counts the same list).
fn collect_units(cwd: &Path) -> Vec<ProjectUnit> {
    let Some(pom) = parse_pom_xml(cwd) else {
        return Vec::new();
    };
    pom.modules
        .into_iter()
        .map(|module| {
            // SEC-14 (sibling policy: `go_work`/`use` directives,
            // `resolved_workspace_members`): a `<module>` entry is untrusted
            // manifest text, so an absolute or `..`-carrying path never
            // reaches `join` + read. The unit is still emitted (count
            // parity); only the child-pom enrichment is skipped.
            let in_tree = !Path::new(&module).is_absolute()
                && !Path::new(&module)
                    .components()
                    .any(|c| matches!(c, Component::ParentDir));
            let child = in_tree.then(|| parse_pom_xml(&cwd.join(&module))).flatten();
            let name = child
                .as_ref()
                .and_then(|c| c.name.clone().or_else(|| c.artifact_id.clone()))
                .unwrap_or_else(|| format_unit_name(&module));
            let version = child.and_then(|c| c.version);
            let mut unit = ProjectUnit::new(name, module);
            unit.version = version;
            unit
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::ProjectUnit;

    #[test]
    fn maven_provider_name() {
        assert_eq!(MavenIdentityProvider.name(), "project_identity");
    }

    #[test]
    fn maven_provider_about_fields() {
        let fields = MavenIdentityProvider.about_fields();
        assert!(!fields.is_empty());
    }

    /// TEST-11 / TASK-1751: with **no `pom.xml` at all** the provider still
    /// yields an identity whose `name` is the working-directory name — not
    /// merely "some non-empty string", which every realistic breakage of the
    /// fallback (wrong ancestor, hardcoded placeholder, whitespace) would
    /// also satisfy.
    #[test]
    fn maven_provider_provide_no_pom_falls_back_to_dir_name() {
        let dir = tempfile::tempdir().unwrap();
        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let result = MavenIdentityProvider.provide(&mut ctx).unwrap();

        let expected = dir.path().file_name().unwrap().to_str().unwrap();
        assert_eq!(result["name"], expected);
        assert_eq!(result["stack_detail"], "Maven");
        assert!(result["version"].is_null());
    }

    /// TEST-11 / TASK-1751: a `pom.xml` that exists but carries **no name and
    /// no artifactId** takes the same fallback — a distinct scenario from the
    /// missing-manifest case above, since here the parser did run (its
    /// `<version>` comes through).
    #[test]
    fn maven_provider_provide_pom_without_name_falls_back_to_dir_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <version>1.0</version>\n</project>",
        )
        .unwrap();

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let result = MavenIdentityProvider.provide(&mut ctx).unwrap();

        let expected = dir.path().file_name().unwrap().to_str().unwrap();
        assert_eq!(result["name"], expected);
        assert_eq!(result["version"], "1.0");
    }

    /// Provider-specific shape: empty modules yields null `module_count`, a
    /// POM without `<url>` yields null `homepage`, and `stack_detail` is
    /// always "Maven". Parser coverage lives in `pom::tests::parse_pom_basic`.
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

    /// TASK-2204 AC #2: a POM declaring only a top-level `<url>` yields that
    /// URL as the homepage and leaves `repository` to the git-remote fallback
    /// (null here — the fixture has no `.git`), instead of mislabelling the
    /// homepage as the repository.
    #[test]
    fn maven_provider_top_level_url_is_homepage_not_repository() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            "<project>\n    <artifactId>mylib</artifactId>\n    <url>https://example.com</url>\n</project>",
        )
        .unwrap();

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let result = MavenIdentityProvider.provide(&mut ctx).unwrap();

        assert_eq!(result["homepage"], "https://example.com");
        assert!(
            result["repository"].is_null(),
            "a POM without <scm> must leave repository to the git fallback, got: {}",
            result["repository"]
        );
    }

    /// TASK-2204 AC #3: a POM declaring both a top-level `<url>` and an
    /// `<scm><url>` maps the former to homepage and the latter to
    /// repository, in either source order.
    #[test]
    fn maven_provider_homepage_and_repository_come_from_distinct_elements() {
        for (label, pom_xml) in [
            (
                "scm first",
                "<project>\n    <artifactId>mylib</artifactId>\n    <scm><url>https://github.com/user/mylib</url></scm>\n    <url>https://example.com</url>\n</project>",
            ),
            (
                "url first",
                "<project>\n    <artifactId>mylib</artifactId>\n    <url>https://example.com</url>\n    <scm><url>https://github.com/user/mylib</url></scm>\n</project>",
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            std::fs::write(dir.path().join("pom.xml"), pom_xml).unwrap();

            let mut ctx = Context::test_context(dir.path().to_path_buf());
            let result = MavenIdentityProvider.provide(&mut ctx).unwrap();

            assert_eq!(result["homepage"], "https://example.com", "{label}");
            assert_eq!(
                result["repository"], "https://github.com/user/mylib",
                "{label}"
            );
        }
    }

    /// TASK-2207 AC #2: the units provider's list length must equal the
    /// identity card's `module_count` on the same fixture — one unit per
    /// `<module>` entry, whether or not the child `pom.xml` exists.
    #[test]
    fn maven_units_length_equals_identity_module_count() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r"<project>
    <artifactId>root</artifactId>
    <modules>
        <module>core</module>
        <module>web</module>
    </modules>
</project>",
        )
        .unwrap();
        // A child pom for one module only: enrichment must not decide
        // whether a module is a unit.
        std::fs::create_dir_all(dir.path().join("core")).unwrap();
        std::fs::write(
            dir.path().join("core/pom.xml"),
            "<project>\n    <artifactId>core-lib</artifactId>\n    <version>1.0.0</version>\n</project>",
        )
        .unwrap();

        let mut ctx = Context::test_context(dir.path().to_path_buf());
        let identity = MavenIdentityProvider.provide(&mut ctx).unwrap();
        let units: Vec<ProjectUnit> =
            serde_json::from_value(MavenUnitsProvider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(identity["module_count"].as_u64(), Some(2));
        assert_eq!(units.len(), 2, "one unit per <module> entry");
        // The module with a child pom takes its name/version from there…
        assert_eq!(units[0].name, "core-lib");
        assert_eq!(units[0].version.as_deref(), Some("1.0.0"));
        assert_eq!(units[0].path, "core");
        // …and the module without one falls back to the directory name.
        assert_eq!(units[1].name, "Web");
        assert_eq!(units[1].path, "web");
        assert_eq!(units[1].version, None);
    }

    /// TASK-2207: an out-of-tree `<module>` entry (absolute or `..`-carrying)
    /// still becomes a unit — count parity — but its child `pom.xml` is
    /// never read, mirroring the SEC-14 policy the Go stack applies to `use`
    /// directives.
    #[test]
    fn maven_units_out_of_tree_module_is_listed_but_not_read() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("pom.xml"),
            r"<project>
    <artifactId>root</artifactId>
    <modules>
        <module>../sibling</module>
    </modules>
</project>",
        )
        .unwrap();
        // A pom at the traversal target — it must not be read into the unit.
        let sibling = dir
            .path()
            .parent()
            .unwrap()
            .join("sibling-should-not-be-read");
        std::fs::create_dir_all(&sibling).unwrap();
        std::fs::write(
            sibling.join("pom.xml"),
            "<project>\n    <artifactId>should-not-be-read</artifactId>\n</project>",
        )
        .unwrap();

        let units = collect_units(dir.path());

        assert_eq!(units.len(), 1);
        assert_eq!(units[0].path, "../sibling");
        assert_ne!(units[0].name, "should-not-be-read");
        assert_eq!(units[0].version, None);

        std::fs::remove_dir_all(&sibling).ok();
    }
}
