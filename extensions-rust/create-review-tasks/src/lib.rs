//! Rust stack implementation for the create-review-tasks system.
//!
//! Registers the `review_targets` data provider consumed by the generic
//! `ops_create_review_tasks` engine: every Cargo workspace member becomes
//! one review target, identified by its package name, with the
//! `code-review-rust` skill named in the subtask titles.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

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
