//! `git` extension: exposes local git repository metadata as a data provider.
//!
//! Datasource-only. Registers a single [`provider::GitInfoProvider`] under
//! the name `git_info` returning `{ host, owner, repo, remote_url, branch }`.
//!
//! This extension is stack-agnostic — it's useful in any project with a `.git`
//! directory, regardless of language.

// ARCH-11 / TASK-2107: the three cast allows this root used to carry are
// gone -- the crate contains no `as` cast, and the workspace denies
// `clippy::as_conversions` anyway -- so they suppressed nothing while
// pre-authorizing future lossy casts with no reviewer signal. The test
// unwrap allow stays: fixture assertions read better as `.unwrap()`.
#![cfg_attr(test, allow(clippy::unwrap_used))]

pub mod config;
pub mod provider;
pub mod remote;

use ops_extension::ExtensionType;

/// Extension identifier used to register this crate in the engine's
/// extension registry.
pub const NAME: &str = "git";
/// One-line description shown by `ops about` for this extension.
pub const DESCRIPTION: &str = "Git repository metadata (remote, branch, etc.)";
/// CLI-facing short name (`git`) used in commands and user-facing output.
pub const SHORTNAME: &str = "git";

/// Datasource extension exposing the `git_info` provider to the engine.
pub struct GitExtension;

impl Default for GitExtension {
    fn default() -> Self {
        Self
    }
}

ops_extension::impl_extension! {
    GitExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some(provider::DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        let _ = registry.register(
            provider::DATA_PROVIDER_NAME,
            Box::new(provider::GitInfoProvider),
        );
    },
    factory: GIT_FACTORY = |_, _| {
        Some((NAME, Box::new(GitExtension)))
    },
}
