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
/// shared by the about providers. See [`resolved_workspace_members`].
pub use query::resolved_workspace_members;

/// Re-exported for sibling Rust-stack extension crates (DUP-3 / TASK-1814):
/// the shared crate-manifest reader and the SEC-14 / TASK-1246 member-path
/// guard that protects it, so no sibling maintains a second copy of the
/// read/parse/log policy or of the absolute-and-`..` rejection. See
/// [`read_crate_metadata`] and [`member_path_is_workspace_safe`].
pub use query::member_path_is_workspace_safe;
pub use units::{read_crate_metadata, CrateMetadata};

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
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(identity::RustIdentityProvider));
        let _ = registry.register(units::PROVIDER_NAME, Box::new(units::RustUnitsProvider));
        let _ = registry.register(
            coverage_provider::PROVIDER_NAME,
            Box::new(coverage_provider::RustCoverageProvider),
        );
        let _ = registry.register(
            deps_provider::PROVIDER_NAME,
            Box::new(deps_provider::RustDepsProvider),
        );
    },
    factory: ABOUT_RUST_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutRustExtension)))
    },
}
