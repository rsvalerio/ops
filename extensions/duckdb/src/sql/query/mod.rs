//! Query functions for extracting project and crate-level metrics from DuckDB.
//!
//! Split by query family for cohesion:
//! - [`helpers`] — shared scaffolding (locking, `CrateCoverage`, per-crate builders)
//! - [`loc`]      — LOC, file count, per-language queries over `tokei_files`,
//!   plus the Rust region breakdown over `rust_loc_summary`
//! - [`coverage`] — project/per-crate coverage over `coverage_files`
//! - [`deps`]     — dependency count and per-crate deps over `crate_dependencies`

mod coverage;
mod deps;
mod helpers;
mod loc;
#[cfg(test)]
mod tests;

pub use coverage::{query_crate_coverage, query_project_coverage};
pub use deps::{query_crate_dep_counts, query_crate_deps, query_dependency_count};
pub use helpers::CrateCoverage;
pub use loc::{
    query_crate_file_count, query_crate_loc, query_project_file_count, query_project_languages,
    query_project_loc, query_rust_loc_file_count, query_rust_loc_summary, RustLocStat,
};
