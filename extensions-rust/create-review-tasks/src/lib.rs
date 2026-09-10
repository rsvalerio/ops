//! Rust stack implementation for the create-review-tasks system.
//!
//! Registers the `review_targets` data provider consumed by the generic
//! `ops_create_review_tasks` engine: every Cargo workspace member becomes
//! one review target, identified by its package name, with the
//! `code-review-rust` skill named in the subtask titles. A single-package
//! project (no `[workspace]` table) yields its root package as the one
//! review target — see [`provider`].

pub(crate) mod provider;

/// Extension identifier used to register this crate in the engine's
/// extension registry.
const NAME: &str = "create-review-tasks-rust";
/// One-line description shown by `ops about` for this extension.
const DESCRIPTION: &str = "Rust review targets for create-review-tasks";
/// CLI-facing short name (`create-review-tasks-rs`) used in commands and
/// user-facing output.
const SHORTNAME: &str = "create-review-tasks-rs";

/// Extension type wiring the Rust `review_targets` provider into the generic
/// create-review-tasks engine.
pub struct CreateReviewTasksRustExtension;

ops_extension::impl_extension! {
    CreateReviewTasksRustExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ops_extension::ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Rust),
    command_names: &[],
    data_provider_name: Some(ops_create_review_tasks::DATA_PROVIDER_NAME),
    register_commands: |_self, _registry| {},
    register_data_providers: |_self, registry| {
        let _ = registry.register(
            ops_create_review_tasks::DATA_PROVIDER_NAME,
            Box::new(provider::RustReviewTargetsProvider),
        );
    },
    factory: CREATE_REVIEW_TASKS_RUST_FACTORY = |_, _| {
        Some((NAME, Box::new(CreateReviewTasksRustExtension)))
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_extension::{Context, DataRegistry, Extension};

    /// The registration closure in `impl_extension!` is verified through
    /// the engine's own lookup path — `registry.provide` under the *engine
    /// crate's* key constant — so the assertions that carry the weight are
    /// behavioural: the provider that answers is this crate's Rust provider
    /// (it emits `code-review-rust` and lists the solo package as a target).
    /// Fails if this crate's registration key ever drifts from the engine's
    /// constant (e.g. replaced by a decoupled literal): the lookup then
    /// lands on `NotFound`.
    #[test]
    fn extension_registers_the_review_targets_provider_under_the_engine_key() {
        let mut registry = DataRegistry::new();
        CreateReviewTasksRustExtension.register_data_providers(&mut registry);

        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"solo\"\nversion = \"0.1.0\"\n",
        )
        .expect("root manifest");
        let mut ctx = Context::test_context(root.to_path_buf());
        let payload = registry
            .provide(ops_create_review_tasks::DATA_PROVIDER_NAME, &mut ctx)
            .expect("the engine's lookup key must resolve this provider");
        assert_eq!(payload["skill"], provider::SKILL_NAME);
        // Identity, not just presence: the solo package appears as the one
        // review target, proving the answering provider is this crate's.
        assert_eq!(
            payload["targets"][0]["name"],
            serde_json::json!("solo"),
            "got: {payload}"
        );
        assert_eq!(payload["targets"][0]["path"], serde_json::json!("."));
    }
}
