//! Rust stack implementation for the about system.
//!
//! Provides Rust-specific data providers (identity, units, coverage,
//! dependencies). All about subpages (units, coverage, dependencies, code)
//! are rendered by the generic `ops_about` crate — this crate only supplies
//! data.
//!
//! Split into submodules by responsibility:
//! - `identity`: `project_identity` data provider
//! - `units`: `project_units` data provider (workspace members)
//! - `coverage_provider`: `project_coverage` data provider
//! - `deps_provider`: `project_dependencies` data provider
//!
//! Shared rendering for about subpages (units, coverage, dependencies, code)
//! lives in the generic `ops_about` crate.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

pub(crate) mod coverage_provider;
pub(crate) mod deps_provider;
pub(crate) mod identity;
pub(crate) mod query;
pub(crate) mod units;

pub const NAME: &str = "about-rust";
pub const DESCRIPTION: &str = "Rust project identity and about pages";
pub const SHORTNAME: &str = "about-rs";
pub const DATA_PROVIDER_NAME: &str = "project_identity";

/// Re-exported for sibling Rust-stack extension crates: the resolved
/// `[workspace].members` view (glob-expanded, excluded, sorted, deduped)
/// shared by the about providers. See [`query::resolved_workspace_members`].
pub use query::resolved_workspace_members;

pub struct AboutRustExtension;

ops_extension::impl_extension! {
    AboutRustExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ops_extension::ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Rust),
    command_names: &[],
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_commands: |_self, _registry| {},
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(identity::RustIdentityProvider));
        registry.register(units::PROVIDER_NAME, Box::new(units::RustUnitsProvider));
        registry.register(
            coverage_provider::PROVIDER_NAME,
            Box::new(coverage_provider::RustCoverageProvider),
        );
        registry.register(
            deps_provider::PROVIDER_NAME,
            Box::new(deps_provider::RustDepsProvider),
        );
    },
    factory: ABOUT_RUST_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutRustExtension)))
    },
}
