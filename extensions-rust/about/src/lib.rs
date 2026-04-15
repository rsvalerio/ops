//! Rust stack implementation for the about system.
//!
//! Provides:
//! - `project_identity` data provider (Rust-specific project identity)
//! - About subpages (coverage, code, dependencies, crates)
//!
//! Split into submodules by responsibility (CQ-001):
//! - `identity`: Rust project_identity data provider
//! - `text_util`: formatting, padding, truncation, wrapping
//! - `cards`: crate card rendering and grid layout
//! - `query`: data fetching from DuckDB/providers
//! - `format`: section formatters for about pages
//! - `pages`: about subpage orchestration and section formatters

pub(crate) mod cards;
pub(crate) mod format;
pub(crate) mod identity;
pub mod pages;
pub(crate) mod query;
pub(crate) mod text_util;

pub use pages::{run_about_page, AboutPage};

pub const NAME: &str = "about-rust";
pub const DESCRIPTION: &str = "Rust project identity and about pages";
pub const SHORTNAME: &str = "about-rs";
pub const DATA_PROVIDER_NAME: &str = "project_identity";

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
    },
    factory: ABOUT_RUST_FACTORY = |_, _| {
        Some((NAME, Box::new(AboutRustExtension)))
    },
}
