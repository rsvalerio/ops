//! Java stack `project_identity` providers (Maven and Gradle).
//!
//! Provides two extensions:
//! - `AboutMavenExtension` (stack: `JavaMaven`) — parses `pom.xml`
//! - `AboutGradleExtension` (stack: `JavaGradle`) — parses `settings.gradle` + `gradle.properties`
//!
//! Parse and read errors fall back to defaults; non-NotFound read errors and
//! parse errors are reported via `tracing` (`debug!` / `warn!`) so a malformed
//! manifest does not silently look like a missing one. The Maven provider folds
//! the parser's `Option<PomData>` through `unwrap_or_default`, and Gradle
//! line-based scans use `for_each_trimmed_line`, which treats unreadable files
//! as absent.

// `allow`, not `expect`: the suppression is cfg-scoped to `test`, so an
// `#[expect]` would fire `unfulfilled_lint_expectations` in every non-test
// build.
#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        reason = "fixture setup and assertions in this crate's tests unwrap freely; a failed unwrap is a failed test"
    )
)]

mod gradle;
mod maven;

use ops_core::project_identity::{base_about_fields, insert_homepage_field, AboutFieldDef};
use ops_extension::ExtensionType;

use gradle::GradleIdentityProvider;
use maven::MavenIdentityProvider;

// --- Maven ---

const MAVEN_NAME: &str = "about-java-maven";
const MAVEN_DESCRIPTION: &str = "Java Maven project identity";
const MAVEN_SHORTNAME: &str = "about-mvn";

/// Datasource extension supplying the Maven stack's about providers
/// (identity and units, read from `pom.xml`) to the generic `ops_about`
/// renderers.
///
/// `#[non_exhaustive]` because construction goes through the
/// `impl_extension!` factory, not a struct literal.
#[non_exhaustive]
pub struct AboutMavenExtension;

ops_extension::impl_extension! {
    AboutMavenExtension,
    name: MAVEN_NAME,
    description: MAVEN_DESCRIPTION,
    shortname: MAVEN_SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::JavaMaven),
    data_provider_name: Some("project_identity"),
    register_data_providers: |_self, registry| {
        let _ = registry.register("project_identity", Box::new(MavenIdentityProvider));
        // The card counts `<module>` entries, so the units page lists them.
        let _ = registry.register("project_units", Box::new(maven::MavenUnitsProvider));
    },
    factory: MAVEN_ABOUT_FACTORY = |_, _| {
        Some((MAVEN_NAME, Box::new(AboutMavenExtension)))
    },
}

// --- Gradle ---

const GRADLE_NAME: &str = "about-java-gradle";
const GRADLE_DESCRIPTION: &str = "Java Gradle project identity";
const GRADLE_SHORTNAME: &str = "about-gradle";

/// Datasource extension supplying the Gradle stack's about providers
/// (identity and units, read from `build.gradle`/`build.gradle.kts` and
/// `settings.gradle`) to the generic `ops_about` renderers.
///
/// `#[non_exhaustive]` because construction goes through the
/// `impl_extension!` factory, not a struct literal.
#[non_exhaustive]
pub struct AboutGradleExtension;

ops_extension::impl_extension! {
    AboutGradleExtension,
    name: GRADLE_NAME,
    description: GRADLE_DESCRIPTION,
    shortname: GRADLE_SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::JavaGradle),
    data_provider_name: Some("project_identity"),
    register_data_providers: |_self, registry| {
        let _ = registry.register("project_identity", Box::new(GradleIdentityProvider));
        // The card counts `include` entries, so the units page lists them.
        let _ = registry.register("project_units", Box::new(gradle::GradleUnitsProvider));
    },
    factory: GRADLE_ABOUT_FACTORY = |_, _| {
        Some((GRADLE_NAME, Box::new(AboutGradleExtension)))
    },
}

// --- Shared ---

// The two Java stacks declare different field sets. The Maven card declares
// `homepage` because the POM's top-level `<url>` fills it; the Gradle provider
// parses no homepage source, so its card omits a row that would always be
// empty.

fn maven_about_fields() -> Vec<AboutFieldDef> {
    use std::sync::OnceLock;
    static FIELDS: OnceLock<Vec<AboutFieldDef>> = OnceLock::new();
    FIELDS
        .get_or_init(|| {
            let mut fields = base_about_fields();
            insert_homepage_field(&mut fields);
            fields
        })
        .clone()
}

fn gradle_about_fields() -> Vec<AboutFieldDef> {
    base_about_fields()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_extension::{Context, DataRegistry, Extension};

    /// Builds each extension through its linkme factory and asserts the
    /// declared metadata — name, shortname, description, type, stack. A wrong
    /// stack tag registers the extension for the wrong Java build tool while
    /// every provider-level test stays green, so the tags are pinned here.
    #[test]
    fn factories_build_extensions_with_declared_metadata() {
        let cfg = ops_core::config::Config::empty();
        let cwd = std::path::Path::new(".");

        let (maven_name, maven) =
            (super::MAVEN_ABOUT_FACTORY)(&cfg, cwd).expect("Maven factory must yield an extension");
        assert_eq!(maven_name, MAVEN_NAME);
        assert_eq!(Extension::name(maven.as_ref()), MAVEN_NAME);
        assert_eq!(maven.shortname(), MAVEN_SHORTNAME);
        assert_eq!(maven.description(), MAVEN_DESCRIPTION);
        assert!(maven.types().is_datasource());
        assert_eq!(maven.stack(), Some(ops_extension::Stack::JavaMaven));
        assert_eq!(maven.data_provider_name(), Some("project_identity"));

        let (gradle_name, gradle) = (super::GRADLE_ABOUT_FACTORY)(&cfg, cwd)
            .expect("Gradle factory must yield an extension");
        assert_eq!(gradle_name, GRADLE_NAME);
        assert_eq!(Extension::name(gradle.as_ref()), GRADLE_NAME);
        assert_eq!(gradle.shortname(), GRADLE_SHORTNAME);
        assert_eq!(gradle.description(), GRADLE_DESCRIPTION);
        assert!(gradle.types().is_datasource());
        assert_eq!(gradle.stack(), Some(ops_extension::Stack::JavaGradle));
        assert_eq!(gradle.data_provider_name(), Some("project_identity"));
    }

    /// Asserts that each `register_data_providers` closure installs *its own
    /// stack's* identity provider under `project_identity`. The two closures
    /// are near-identical and a swap would pass a mere presence check, so the
    /// registered provider is identified by what it answers over a real
    /// fixture: `stack_detail` "Maven" for the Maven extension, "Gradle" for
    /// the Gradle one.
    #[test]
    fn each_extension_registers_its_own_stack_identity_provider() {
        let maven_dir = tempfile::tempdir().unwrap();
        std::fs::write(
            maven_dir.path().join("pom.xml"),
            "<project><artifactId>demo-maven</artifactId><version>1.0</version></project>",
        )
        .unwrap();
        let mut registry = DataRegistry::new();
        AboutMavenExtension.register_data_providers(&mut registry);
        let mut ctx = Context::test_context(maven_dir.path().to_path_buf());
        let identity = registry
            .provide("project_identity", &mut ctx)
            .expect("Maven extension must register project_identity");
        assert_eq!(
            identity["stack_detail"],
            serde_json::json!("Maven"),
            "the Maven extension must register the Maven provider: {identity}"
        );

        let gradle_dir = tempfile::tempdir().unwrap();
        std::fs::write(
            gradle_dir.path().join("settings.gradle"),
            "rootProject.name = \"demo-gradle\"\n",
        )
        .unwrap();
        let mut registry = DataRegistry::new();
        AboutGradleExtension.register_data_providers(&mut registry);
        let mut ctx = Context::test_context(gradle_dir.path().to_path_buf());
        let identity = registry
            .provide("project_identity", &mut ctx)
            .expect("Gradle extension must register project_identity");
        assert_eq!(
            identity["stack_detail"],
            serde_json::json!("Gradle"),
            "the Gradle extension must register the Gradle provider: {identity}"
        );
    }

    /// Pins both halves of the per-stack field split: `homepage` sits
    /// immediately before `coverage` in `maven_about_fields` (the POM's
    /// top-level `<url>` fills it), and `gradle_about_fields` carries no
    /// `homepage` row at all because the Gradle provider parses no homepage
    /// source.
    #[test]
    fn about_fields_place_homepage_before_coverage_only_where_fillable() {
        let fields = maven_about_fields();
        let homepage = fields
            .iter()
            .position(|f| f.id == "homepage")
            .expect("homepage field present on the Maven card");
        let coverage = fields
            .iter()
            .position(|f| f.id == "coverage")
            .expect("coverage field present");
        assert_eq!(
            homepage + 1,
            coverage,
            "homepage must sit immediately before coverage: {:?}",
            fields.iter().map(|f| f.id).collect::<Vec<_>>()
        );

        let gradle_fields = gradle_about_fields();
        assert!(
            gradle_fields.iter().all(|f| f.id != "homepage"),
            "the Gradle card must not advertise a structurally empty homepage row: {:?}",
            gradle_fields.iter().map(|f| f.id).collect::<Vec<_>>()
        );
    }
}
