//! Cargo.toml extension: parses manifest files and provides structured data to other extensions.
//!
//! This extension serves as the canonical example of a **data-source-only extension**:
//! it registers no commands, only a data provider that other extensions can consume.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────┐    register_data_providers()    ┌──────────────────┐
//! │ CargoTomlExt    │ ─────────────────────────────▶  │ DataRegistry     │
//! │ (Extension)     │                                 │ ("cargo_toml")   │
//! └─────────────────┘                                 └──────────────────┘
//!                                                              │
//!                                                              │ provide()
//!                                                              ▼
//! ┌─────────────────┐    Context::get_or_provide()    ┌──────────────────┐
//! │ Other Extension │ ◀─────────────────────────────  │ CargoTomlProvider│
//! │ (Consumer)      │                                 │ (DataProvider)   │
//! └─────────────────┘                                 └──────────────────┘
//! ```
//!
//! # Usage
//!
//! ## As a Data Provider (from other extensions)
//!
//! ```ignore
//! use ops_extension::{Context, DataRegistry};
//! use ops_cargo_toml::CargoToml;
//!
//! fn my_extension_logic(ctx: &mut Context, registry: &DataRegistry) -> Result<(), anyhow::Error> {
//!     // Get cached Cargo.toml data
//!     let value = ctx.get_or_provide("cargo_toml", registry)?;
//!     let manifest: CargoToml = serde_json::from_value((*value).clone())?;
//!
//!     if let Some(pkg) = &manifest.package {
//!         println!("Package: {} v{}", pkg.name, pkg.version.as_str().unwrap_or(""));
//!     }
//!     Ok(())
//! }
//! ```
//!
//! ## Workspace Inheritance
//!
//! Dependencies with `workspace = true` are automatically resolved from
//! `[workspace.dependencies]`. Enable this by calling [`CargoToml::resolve_inheritance`]
//! after parsing, or use the resolved data from the provider.
//!
//! # Implementation Notes for New Data Providers
//!
//! When creating a new data-source-only extension, follow this pattern:
//!
//! 1. **Create types** in `types.rs` with `#[derive(Deserialize)]`
//! 2. **Implement `DataProvider`** with a unique name (used as registry key)
//! 3. **Implement `Extension`** that registers only data providers (no commands)
//! 4. **Support configuration** via builder pattern if needed (e.g., custom paths)
//! 5. **Document the data contract** clearly so consumers know what to expect

mod inheritance;
#[cfg(test)]
mod tests;
mod types;

#[allow(unused_imports)]
pub use inheritance::InheritanceError;
#[allow(unused_imports)]
pub use types::{
    CargoToml, DepSpec, DetailedDepSpec, Package, PublishSpec, ReadmeSpec, Workspace,
    WorkspacePackage,
};

use anyhow::Context as _;
use ops_core::text::read_capped_to_string;
use ops_extension::{Context, DataProvider, DataProviderError, DataProviderSchema, ExtensionType};
use std::fs;
use std::path::{Path, PathBuf};

/// ARCH-2 / TASK-0871: typed errors for [`find_workspace_root`]. Replaces
/// the previously synthesised `io::Error::new(NotFound, …)`, so consumers
/// (notably `is_manifest_missing` in `extensions-rust/about`) can match a
/// typed variant instead of walking the source chain looking for an
/// `io::ErrorKind::NotFound` shape that another wrapping layer would mask.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FindWorkspaceRootError {
    #[error(
        "no Cargo.toml found in {start} or any parent directory (walked up to {depth} ancestors)"
    )]
    NotFound { start: PathBuf, depth: usize },
    #[error("failed to canonicalize {path}")]
    CanonicalizeFailed {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

impl FindWorkspaceRootError {
    /// True when this error indicates the search walked to its bound without
    /// finding any `Cargo.toml`. Mirrors the legacy `io::ErrorKind::NotFound`
    /// signal that `is_manifest_missing` consumed.
    #[must_use]
    pub fn is_not_found(&self) -> bool {
        matches!(self, Self::NotFound { .. })
    }
}

pub const NAME: &str = "cargo-toml";
#[allow(dead_code)]
pub const DESCRIPTION: &str = "Cargo.toml manifest parser and workspace data provider";
#[allow(dead_code)]
pub const SHORTNAME: &str = "cargo";
pub const DATA_PROVIDER_NAME: &str = "cargo_toml";

/// Extension that provides Cargo.toml parsing capabilities.
///
/// This extension registers no commands—it only provides data.
/// Use [`CargoTomlExtension::new`] for default behavior (auto-discover workspace root)
/// or [`CargoTomlExtension::with_root`] to specify a custom path.
///
/// # Example
///
/// ```
/// use ops_cargo_toml::CargoTomlExtension;
/// use std::path::PathBuf;
///
/// // Auto-discover from current directory
/// let ext = CargoTomlExtension::new();
///
/// // Or specify explicit root
/// let ext = CargoTomlExtension::with_root(PathBuf::from("/path/to/workspace"));
/// ```
pub struct CargoTomlExtension {
    root: Option<PathBuf>,
}

impl CargoTomlExtension {
    /// Create extension that auto-discovers workspace root from working directory.
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Create extension with an explicit workspace root path.
    #[allow(dead_code)]
    pub fn with_root(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }
}

impl Default for CargoTomlExtension {
    fn default() -> Self {
        Self::new()
    }
}

ops_extension::impl_extension! {
    CargoTomlExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Rust),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |this, registry| {
        let provider = match &this.root {
            Some(p) => CargoTomlProvider::with_root(p.clone()),
            None => CargoTomlProvider::new(),
        };
        registry.register(DATA_PROVIDER_NAME, Box::new(provider));
    },
    factory: CARGO_TOML_FACTORY = |_, _| {
        Some((NAME, Box::new(CargoTomlExtension::new())))
    },
}

/// Data provider that parses Cargo.toml and returns structured JSON.
///
/// This provider:
/// - Discovers workspace root by walking up from the working directory
/// - Parses Cargo.toml into [`CargoToml`] types
/// - Resolves workspace inheritance (`workspace = true`)
/// - Returns fresh data on each call (no internal caching)
///
/// Consumers should use `Context::get_or_provide()` for caching.
pub struct CargoTomlProvider {
    root: Option<PathBuf>,
}

impl CargoTomlProvider {
    /// Create provider that auto-discovers workspace root.
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Create provider with an explicit workspace root path.
    pub fn with_root(root: PathBuf) -> Self {
        Self { root: Some(root) }
    }

    fn resolve_root(&self, working_dir: &Path) -> Result<PathBuf, anyhow::Error> {
        if let Some(root) = &self.root {
            return Ok(root.clone());
        }
        Ok(find_workspace_root(working_dir)?)
    }
}

impl Default for CargoTomlProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DataProvider for CargoTomlProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        let root = self.resolve_root(&ctx.working_directory)?;
        let cargo_toml = root.join("Cargo.toml");

        // SEC-33 (TASK-0926): byte-cap the manifest read so an adversarial
        // workspace cannot OOM `ops` via an oversized or `/dev/zero` Cargo.toml.
        let content = read_capped_to_string(&cargo_toml)
            .with_context(|| format!("reading {}", cargo_toml.display()))?;

        let mut manifest: CargoToml = toml::from_str(&content)
            .with_context(|| format!("parsing {}", cargo_toml.display()))?;

        manifest
            .resolve_inheritance()
            .context("resolving workspace inheritance")?;

        manifest.resolve_package_inheritance();

        Ok(serde_json::to_value(&manifest)?)
    }

    fn schema(&self) -> DataProviderSchema {
        use ops_extension::data_field;
        DataProviderSchema::new(
            "Cargo.toml manifest data (parsed from workspace root)",
            vec![
                data_field!(
                    "package",
                    "Option<Package>",
                    "Root package definition (None for virtual workspaces)"
                ),
                data_field!("workspace", "Option<Workspace>", "Workspace configuration"),
                data_field!(
                    "dependencies",
                    "Map<String, DepSpec>",
                    "Package dependencies"
                ),
                data_field!(
                    "dev-dependencies",
                    "Map<String, DepSpec>",
                    "Development dependencies"
                ),
                data_field!(
                    "build-dependencies",
                    "Map<String, DepSpec>",
                    "Build dependencies"
                ),
                data_field!("Package.name", "String", "Package name"),
                data_field!("Package.version", "String", "Package version"),
                data_field!("Package.edition", "String", "Rust edition (e.g., 2021)"),
                data_field!("Package.license", "Option<String>", "License identifier"),
                data_field!(
                    "Package.description",
                    "Option<String>",
                    "Package description"
                ),
                data_field!("Package.repository", "Option<String>", "Repository URL"),
                data_field!("Package.authors", "Vec<String>", "Package authors"),
                data_field!("Workspace.members", "Vec<String>", "Workspace member paths"),
                data_field!(
                    "Workspace.dependencies",
                    "Map<String, DepSpec>",
                    "Shared workspace dependencies"
                ),
                data_field!(
                    "DepSpec",
                    "String | DetailedDepSpec",
                    "Simple version string or detailed spec with features"
                ),
            ],
        )
    }
}

/// Maximum ancestor depth walked when searching for `Cargo.toml`. Defensive
/// bound that prevents a symlink loop (or pathologically deep mount layout)
/// from spinning the discovery loop forever.
const MAX_ANCESTOR_DEPTH: usize = 64;

/// Find the workspace root by walking up from `start` looking for Cargo.toml.
///
/// TASK-0501: prefers the *outermost* `Cargo.toml` containing `[workspace]`
/// over the first `Cargo.toml` encountered. Running from inside a member
/// crate (e.g. `cd crates/foo`) used to return the member manifest; the new
/// walk continues past member manifests until it finds the workspace root.
/// If no manifest in the chain declares `[workspace]`, the first encountered
/// `Cargo.toml` is returned — preserving the single-crate / non-workspace
/// project behaviour.
///
/// The caller's `start` is canonicalized first so symlinks under `start` are
/// resolved once up front, and the walk is capped at [`MAX_ANCESTOR_DEPTH`]
/// so a symlink-induced loop cannot hang the process.
///
/// SEC-25 (TASK-0604): canonicalisation is *not* re-applied on each ancestor.
/// Because `start_canonical` is already a resolved path, walking it with
/// `.parent()` returns a prefix that is itself canonical — for the typical
/// case this is sufficient. The remaining gap is a TOCTOU window: if a
/// parent directory is replaced with a symlink between canonicalisation and
/// `manifest_declares_workspace`, the walk reads through the new symlink.
/// The depth cap bounds the damage; treat the result as "best-effort
/// symlink-safe" rather than absolute.
pub fn find_workspace_root(start: &Path) -> Result<PathBuf, FindWorkspaceRootError> {
    // ARCH-2 / TASK-0918: a missing or dangling-symlink `start`
    // (transient cwd unlink — CI volume eviction, watcher rename, etc.)
    // used to surface as a confusing "failed to canonicalize" error.
    // Treat NotFound on the canonicalize as "no manifest reachable from
    // here" so downstream consumers route through the same NotFound
    // branch as a regular missing-Cargo.toml. Other IO errors
    // (PermissionDenied, IsADirectory, …) keep their typed CanonicalizeFailed
    // variant so they remain investigable.
    let start_canonical = match fs::canonicalize(start) {
        Ok(p) => p,
        Err(source) if source.kind() == std::io::ErrorKind::NotFound => {
            tracing::debug!(
                start = ?start.display(),
                "find_workspace_root: start path is unreachable (canonicalize NotFound); reporting NotFound"
            );
            return Err(FindWorkspaceRootError::NotFound {
                start: start.to_path_buf(),
                depth: MAX_ANCESTOR_DEPTH,
            });
        }
        Err(source) => {
            return Err(FindWorkspaceRootError::CanonicalizeFailed {
                path: start.to_path_buf(),
                source,
            });
        }
    };
    let mut current = start_canonical.as_path();
    let mut first_cargo_toml: Option<PathBuf> = None;
    for _ in 0..MAX_ANCESTOR_DEPTH {
        let cargo_toml = current.join("Cargo.toml");
        if cargo_toml.exists() {
            if manifest_declares_workspace(&cargo_toml) {
                return Ok(current.to_path_buf());
            }
            if first_cargo_toml.is_none() {
                first_cargo_toml = Some(current.to_path_buf());
            }
        }
        match current.parent() {
            Some(parent) => current = parent,
            None => break,
        }
    }
    if let Some(root) = first_cargo_toml {
        return Ok(root);
    }
    Err(FindWorkspaceRootError::NotFound {
        start: start.to_path_buf(),
        depth: MAX_ANCESTOR_DEPTH,
    })
}

/// True iff the manifest at `path` parses to TOML and contains a top-level
/// `[workspace]` table. Read errors and parse errors return false — the walk
/// will keep looking and ultimately fall back to the first Cargo.toml seen.
fn manifest_declares_workspace(path: &Path) -> bool {
    // ERR-1 / TASK-0605: distinguish NotFound (legitimately absent during the
    // ancestor walk) from other IO errors (permission denied, EIO, partial
    // write) so a flaky disk does not silently mis-root the entire
    // about/units/coverage stack.
    // SEC-33 (TASK-0926): the ancestor walk visits up to MAX_ANCESTOR_DEPTH
    // candidate manifests; an oversized one anywhere on the chain would
    // otherwise stall the walk with an unbounded read.
    let content = match read_capped_to_string(path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return false,
        Err(e) => {
            // ERR-7 (TASK-0947): Debug-format path/error so an attacker-
            // controlled CWD-derived ancestor path cannot inject newlines
            // or ANSI escapes into operator-facing logs.
            tracing::debug!(
                path = ?path.display(),
                error = ?e,
                "Cargo.toml unreadable during workspace walk; skipping candidate"
            );
            return false;
        }
    };
    match toml::from_str::<toml::Value>(&content) {
        Ok(value) => value
            .as_table()
            .is_some_and(|t| t.contains_key("workspace")),
        Err(e) => {
            // ERR-7 (TASK-0947): Debug-format path/error so a candidate
            // Cargo.toml path with embedded control characters cannot
            // forge log records.
            tracing::warn!(
                path = ?path.display(),
                error = ?e,
                "Cargo.toml parse failed during workspace walk; skipping candidate"
            );
            false
        }
    }
}
