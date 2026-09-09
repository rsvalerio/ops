//! `git` extension: exposes local git repository metadata as a data provider.
//!
//! Datasource-only. Registers a single [`GitInfoProvider`] under the name
//! `git_info` returning `{ host, owner, repo, remote_url, branch }`.
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

pub use provider::{
    resolve_repository_with_git_fallback, GitInfo, GitInfoProvider, DATA_PROVIDER_NAME,
};
pub use remote::{parse_remote_url, RemoteInfo};

use ops_extension::ExtensionType;

pub const NAME: &str = "git";
pub const DESCRIPTION: &str = "Git repository metadata (remote, branch, etc.)";
pub const SHORTNAME: &str = "git";

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
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(GitInfoProvider));
    },
    factory: GIT_FACTORY = |_, _| {
        Some((NAME, Box::new(GitExtension)))
    },
}
