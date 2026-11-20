//! Registry keys of data providers that stack-agnostic consumers name.
//!
//! A generic crate (the about subpages, for example) warms providers that a
//! stack crate produces but cannot depend on that stack crate. Both sides read
//! the key from here instead, so renaming a provider is one edit that every
//! warm call follows, not a literal that compiles and silently warms nothing.

/// Registry and context-cache key of the `cargo metadata` data provider.
pub const CARGO_METADATA: &str = "metadata";
