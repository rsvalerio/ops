//! Registry keys of data providers that stack-agnostic consumers name.
//!
//! A generic crate (the about subpages, for example) warms providers that a
//! stack crate produces but cannot depend on that stack crate. Both sides read
//! the key from here instead, so renaming a provider is one edit that every
//! warm call follows, not a literal that compiles and silently warms nothing.

/// Registry and context-cache key of the `cargo metadata` data provider.
pub const CARGO_METADATA: &str = "metadata";

/// Registry key of the per-project `SQLite` database data provider.
pub const SQLITE: &str = "sqlite";

/// Registry key of the `tokei` code-statistics data provider.
pub const TOKEI: &str = "tokei";

/// Registry key of the Rust production/test/example line-count data provider.
pub const RUST_LOC: &str = "rust-loc";

/// Registry key of the per-unit test-coverage data provider.
pub const COVERAGE: &str = "coverage";

/// Registry key of the parsed `Cargo.toml` data provider.
///
/// Underscore spelling, unlike the producing extension's hyphenated name.
pub const CARGO_TOML: &str = "cargo_toml";
