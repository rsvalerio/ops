//! Rust stack implementation for the create-review-tasks system.
//!
//! Registers the `review_targets` data provider consumed by the generic
//! `ops_create_review_tasks` engine: every Cargo workspace member becomes
//! one review target, identified by its package name, with the
//! `code-review-rust` skill named in the subtask titles. A single-package
//! project (no `[workspace]` table) yields its root package as the one
//! review target — see [`provider`].

// TEST-5 / TASK-1816: the crate-root `#![cfg_attr(test, allow(..))]` block
// that used to sit here is gone. The tests in this crate use `expect` and
// indexing, both already permitted in test code by the `allow-*-in-tests`
// keys in `clippy.toml`, and none of them casts — the block excused nothing.

pub(crate) mod provider;

pub const NAME: &str = "create-review-tasks-rust";
pub const DESCRIPTION: &str = "Rust review targets for create-review-tasks";
pub const SHORTNAME: &str = "create-review-tasks-rs";
pub const DATA_PROVIDER_NAME: &str = ops_create_review_tasks::DATA_PROVIDER_NAME;

pub struct CreateReviewTasksRustExtension;

ops_extension::impl_extension! {
    CreateReviewTasksRustExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ops_extension::ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Rust),
    command_names: &[],
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_commands: |_self, _registry| {},
    register_data_providers: |_self, registry| {
        registry.register(
            DATA_PROVIDER_NAME,
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

    /// TEST-5 / TASK-1816: the registration closure in `impl_extension!` had
    /// no test at all. The key it registers under is what the engine looks
    /// up; a mismatch surfaces at runtime as "the detected stack has no
    /// create-review-tasks extension compiled in".
    #[test]
    fn extension_registers_the_review_targets_provider_under_the_engine_key() {
        let mut registry = DataRegistry::new();
        CreateReviewTasksRustExtension.register_data_providers(&mut registry);

        assert_eq!(
            DATA_PROVIDER_NAME,
            ops_create_review_tasks::DATA_PROVIDER_NAME
        );
        assert!(registry.provider_names().contains(&DATA_PROVIDER_NAME));

        // Identity, not just presence: the registered provider must be this
        // crate's Rust provider, which is what emitting `code-review-rust`
        // for a real workspace proves.
        let dir = tempfile::tempdir().expect("tempdir");
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[package]\nname = \"solo\"\nversion = \"0.1.0\"\n",
        )
        .expect("root manifest");
        let mut ctx = Context::test_context(root.to_path_buf());
        let payload = registry
            .provide(DATA_PROVIDER_NAME, &mut ctx)
            .expect("registered provider must answer");
        assert_eq!(payload["skill"], provider::SKILL_NAME);
    }
}
