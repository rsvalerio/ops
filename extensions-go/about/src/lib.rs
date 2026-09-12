//! Go stack `project_identity` + `project_units` providers.
//!
//! Parses `go.mod` for module name, Go version, and local `replace`
//! directives, and `go.work` for workspace modules. `go.work` takes
//! precedence: a project that has one is a workspace whose modules are its
//! `use` directives, and the root `go.mod` then contributes only identity.
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
// `clippy::redundant_pub_crate` enforces workspace-wide. The crate's
// exported surface is `AboutGoExtension` alone.
mod go_mod;
mod go_syntax;
mod go_work;
mod modules;

use ops_about::identity::{provide_identity_from_manifest, ParsedManifest};
use ops_core::project_identity::{base_about_fields, AboutFieldDef};
use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};

const NAME: &str = "about-go";
const DESCRIPTION: &str = "Go project identity";
const SHORTNAME: &str = "about-go";
const DATA_PROVIDER_NAME: &str = "project_identity";

/// Datasource extension supplying the Go stack's about providers
/// (identity via `go.mod`, units via `go work`) to the generic `ops_about`
/// renderers.
#[non_exhaustive]
pub struct AboutGoExtension;

ops_extension::impl_extension! {
    AboutGoExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Go),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(GoIdentityProvider));
        let _ = registry.register(modules::PROVIDER_NAME, Box::new(modules::GoUnitsProvider));
    },
    factory: GO_ABOUT_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutGoExtension)))
    },
}

struct GoIdentityProvider;

impl DataProvider for GoIdentityProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn about_fields(&self) -> Vec<AboutFieldDef> {
        base_about_fields()
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        provide_identity_from_manifest(ctx.working_directory(), |root| {
            let go_mod = go_mod::parse(root);
            let go_work_use_dirs = go_work::parse_use_dirs(root);

            // The name is the last meaningful segment of the module path:
            // `last_segment` drops a trailing `/vN` major-version suffix, so
            // `github.com/openbao/openbao/api/v2` renders as `api`, not `v2`.
            let name = go_mod
                .as_ref()
                .and_then(|m| m.module.as_deref())
                .and_then(|m| modules::last_segment(Some(m)));

            let stack_detail = go_mod
                .as_ref()
                .and_then(|m| m.go_version.clone())
                .map(|v| format!("Go {v}"));

            let module_count = compute_module_count(go_work_use_dirs.as_deref());

            ParsedManifest::build(|m| {
                m.name = name;
                m.stack_label = "Go";
                m.stack_detail = stack_detail;
                m.module_label = "modules";
                m.module_count = module_count;
            })
        })
    }
}

/// Compute the module count surfaced in the About card.
///
/// A "module" here is exactly what the `project_units` provider lists as a
/// unit. Go's workspace-member concept is the `go.work` `use` directive, so
/// the count is the number of use dirs, which `collect_units` (`modules.rs`)
/// lists one-for-one. A `replace` directive is a dependency substitution, not
/// a workspace member, so a `go.mod`-only project is a *single*-module
/// project however many local replaces it carries and reports `None` — the
/// card omits a meaningless `1`, as it does for single-package Node and
/// Python projects.
///
/// The invariant: wherever `module_count` is `Some`, it equals
/// `collect_units(...).len()`. The Rust stack holds the same one by taking
/// `module_count` from `manifest.resolved_members().len()`
/// (`extensions-rust/about/src/identity/mod.rs`), the exact set its units
/// provider lists.
fn compute_module_count(go_work_use_dirs: Option<&[String]>) -> Option<usize> {
    go_work_use_dirs.map(<[String]>::len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_core::project_identity::ProjectIdentity;

    /// Only `go.work` use dirs are counted — one per workspace member, the
    /// same set `collect_units` lists.
    #[test]
    fn compute_module_count_counts_go_work_use_dirs() {
        let work = vec!["./a".to_string(), "./b".to_string()];
        assert_eq!(compute_module_count(Some(&work)), Some(2));
        assert_eq!(compute_module_count(Some(&[])), Some(0));
    }

    /// A `go.mod`-only project is a single-module project and reports `None`
    /// — the card omits a meaningless `1`, as it does for single-package
    /// Node and Python projects.
    #[test]
    fn compute_module_count_single_mod_project_is_none() {
        assert_eq!(compute_module_count(None), None);
    }

    // --- provider tests ---

    #[test]
    fn provider_name() {
        let provider = GoIdentityProvider;
        assert_eq!(provider.name(), "project_identity");
    }

    #[test]
    fn provider_about_fields_match_base() {
        let provider = GoIdentityProvider;
        let fields = provider.about_fields();
        let base = base_about_fields();
        assert_eq!(fields.len(), base.len());
        for (a, b) in fields.iter().zip(base.iter()) {
            assert_eq!(a.id, b.id);
        }
    }

    #[test]
    fn provide_simple_go_project() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/myapp\n\ngo 1.22\n",
        )
        .unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.name, "myapp");
        assert_eq!(id.stack_label, "Go");
        assert_eq!(id.stack_detail.as_deref(), Some("Go 1.22"));
        assert_eq!(id.module_label, "modules");
        assert!(id.module_count.is_none()); // single module: no count on the card
    }

    #[test]
    fn provide_go_project_with_local_replaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/org/mono\n\ngo 1.21\n\nreplace github.com/org/mono/api => ./api\nreplace github.com/org/mono/sdk => ./sdk\n",
        )
        .unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(id.name, "mono");
        // A `replace` directive is a dependency substitution, not a
        // workspace member — the single-module project reports no count.
        assert_eq!(id.module_count, None);
    }

    /// The identity card's `module_count` and the units provider's list come
    /// from one definition of "module". On a `go.mod`-only fixture with local
    /// `replace` directives the two agree: the units provider lists exactly
    /// the root module, and the count is `None` — never a count larger than
    /// the list.
    #[test]
    fn identity_module_count_agrees_with_units_on_local_replaces() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/org/mono\n\ngo 1.21\n\nreplace github.com/org/mono/api => ./api\nreplace github.com/org/mono/sdk => ./sdk\n",
        )
        .unwrap();
        // Real replace targets, so the fixture is representative.
        for target in ["api", "sdk"] {
            std::fs::create_dir(dir.path().join(target)).unwrap();
            std::fs::write(
                dir.path().join(target).join("go.mod"),
                format!("module github.com/org/mono/{target}\n\ngo 1.21\n"),
            )
            .unwrap();
        }

        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let identity: ProjectIdentity =
            serde_json::from_value(GoIdentityProvider.provide(&mut ctx).unwrap()).unwrap();
        let units: Vec<ops_core::project_identity::ProjectUnit> =
            serde_json::from_value(modules::GoUnitsProvider.provide(&mut ctx).unwrap()).unwrap();

        assert_eq!(
            units.len(),
            1,
            "a go.mod-only project lists exactly the root module"
        );
        assert!(
            identity
                .module_count
                .is_none_or(|count| count == units.len()),
            "module_count {:?} must be None or equal the units length {}; \
             the card and the units table must count the same things",
            identity.module_count,
            units.len()
        );
        assert_eq!(identity.module_count, None);
    }

    #[test]
    fn provide_go_workspace_module_count_from_go_work() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/user/ws\n\ngo 1.21\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("go.work"),
            "go 1.21\n\nuse (\n\t./svc-a\n\t./svc-b\n\t./lib\n)\n",
        )
        .unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        // go.work takes precedence over the root go.mod: 3 use dirs.
        assert_eq!(id.module_count, Some(3));
    }

    #[test]
    fn provide_no_go_mod_falls_back_to_dir_name() {
        let dir = tempfile::tempdir().unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        // Falls back to directory name
        let expected = dir
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        assert_eq!(id.name, expected);
        assert_eq!(id.stack_label, "Go");
        assert!(id.stack_detail.is_none());
        assert!(id.module_count.is_none());
    }

    #[test]
    fn provide_populates_repository_from_git_remote() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module github.com/openbao/openbao\n\ngo 1.21\n",
        )
        .unwrap();
        let git_dir = dir.path().join(".git");
        std::fs::create_dir(&git_dir).unwrap();
        std::fs::write(
            git_dir.join("config"),
            "[remote \"origin\"]\n\turl = https://github.com/openbao/openbao.git\n",
        )
        .unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert_eq!(
            id.repository.as_deref(),
            Some("https://github.com/openbao/openbao")
        );
    }

    #[test]
    fn provide_no_git_leaves_repository_empty() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/foo\n\ngo 1.21\n",
        )
        .unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        assert!(id.repository.is_none());
    }

    /// A `module    ` line (whitespace-only path) drops to `None`, so the
    /// directory-name fallback fires — the `trim_nonempty` policy the Node
    /// and Python identity providers also apply.
    #[test]
    fn provide_whitespace_only_module_falls_back_to_dir_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module    \n\ngo 1.22\n").unwrap();

        let provider = GoIdentityProvider;
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
        assert_eq!(id.stack_detail.as_deref(), Some("Go 1.22"));
    }

    #[test]
    fn provide_simple_module_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("go.mod"), "module myutil\n\ngo 1.20\n").unwrap();

        let provider = GoIdentityProvider;
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let value = provider.provide(&mut ctx).unwrap();
        let id: ProjectIdentity = serde_json::from_value(value).unwrap();

        // No slashes, so name is the whole module path
        assert_eq!(id.name, "myutil");
    }

    /// The real `register_data_providers` closure lands both providers under
    /// their own registry keys, and each answers with its own payload shape.
    /// Both halves matter: each `registry.register` result is discarded with
    /// `let _ =`, so a dropped line silently unregisters a provider, and a
    /// key collision between the two would be resolved by the registry's
    /// first-write-wins rejection rather than reported.
    #[test]
    fn extension_registers_identity_and_units_providers() {
        use ops_extension::{DataRegistry, Extension};

        let mut registry = DataRegistry::new();
        AboutGoExtension.register_data_providers(&mut registry);
        assert_eq!(
            registry.provider_names(),
            vec!["project_identity", "project_units"],
            "both providers must land under distinct keys"
        );

        // Distinct payloads prove distinct providers, not one key answering
        // twice: identity is a ProjectIdentity object, units an array.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("go.mod"),
            "module example.com/app\n\ngo 1.22\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("go.work"), "go 1.22\n\nuse (\n\t.\n)\n").unwrap();
        let mut ctx = ops_extension::Context::test_context(dir.path().to_path_buf());
        let identity = registry
            .provide("project_identity", &mut ctx)
            .expect("identity provider must answer");
        assert_eq!(identity["stack_label"], serde_json::json!("Go"));
        let units = registry
            .provide("project_units", &mut ctx)
            .expect("units provider must answer");
        assert!(
            units.as_array().is_some_and(|a| !a.is_empty()),
            "units payload must be a non-empty array over a go.work fixture: {units}"
        );
    }
}
