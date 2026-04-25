//! Java stack `project_identity` providers (Maven and Gradle).
//!
//! Provides two extensions:
//! - `AboutMavenExtension` (stack: JavaMaven) — parses `pom.xml`
//! - `AboutGradleExtension` (stack: JavaGradle) — parses `settings.gradle` + `gradle.properties`

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
        registry.register("project_identity", Box::new(MavenIdentityProvider));
    },
    factory: MAVEN_ABOUT_FACTORY = |_, _| {
        Some((MAVEN_NAME, Box::new(AboutMavenExtension)))
    },
}

// --- Gradle ---

const GRADLE_NAME: &str = "about-java-gradle";
const GRADLE_DESCRIPTION: &str = "Java Gradle project identity";
const GRADLE_SHORTNAME: &str = "about-gradle";

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
        registry.register("project_identity", Box::new(GradleIdentityProvider));
    },
    factory: GRADLE_ABOUT_FACTORY = |_, _| {
        Some((GRADLE_NAME, Box::new(AboutGradleExtension)))
    },
}

// --- Shared ---

fn java_about_fields() -> Vec<AboutFieldDef> {
    let mut fields = base_about_fields();
    insert_homepage_field(&mut fields);
    fields
}
