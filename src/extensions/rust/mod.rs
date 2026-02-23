//! Rust stack extensions.
//!
//! These extensions are compiled in when the `stack-rust` feature is enabled.

pub mod about;
pub mod cargo_toml;
pub mod metadata;
pub mod ops_db;

pub use about::AboutExtension;
pub use cargo_toml::{
    CargoToml, CargoTomlExtension, DepSpec, DetailedDepSpec, InheritanceError,
    Package as CargoTomlPackage, PublishSpec, ReadmeSpec, Workspace,
};
pub use metadata::{Dependency, DependencyKind, Metadata, MetadataExtension, Package, Target};
pub use ops_db::OpsDbExtension;
