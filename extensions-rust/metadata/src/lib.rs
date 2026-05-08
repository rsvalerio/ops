//! Metadata extension: runs `cargo metadata` and provides workspace info as JSON.
//! DuckDB is the single source of truth - metadata is loaded into metadata_raw table.

mod ingestor;
#[cfg(test)]
mod tests;
mod types;
mod views;

pub use types::{Dependency, DependencyKind, Metadata, Package, Target};

use ingestor::MetadataIngestor;
use ops_core::output::format_error_tail;
use ops_core::subprocess::{run_cargo, RunError};
use ops_duckdb::DuckDb;
use ops_extension::{Context, DataProvider, DataProviderError, DataProviderSchema, ExtensionType};
use std::path::Path;
use std::process::Output;
use std::time::Duration;

const NAME: &str = "metadata";
const DESCRIPTION: &str = "Cargo metadata provider (workspace info, dependencies)";
const SHORTNAME: &str = "meta";
const DATA_PROVIDER_NAME: &str = "metadata";

/// Default timeout for `cargo metadata`; overridable via
/// `OPS_SUBPROCESS_TIMEOUT_SECS`.
pub(crate) const CARGO_METADATA_TIMEOUT: Duration = Duration::from_secs(120);

/// ERR-1 / TASK-1034: byte cap on the JSON payload read from
/// `metadata_raw`. `query_metadata_raw` materialises the row as a
/// `String` (via `to_json(m)::VARCHAR`) and then parses it into a
/// `serde_json::Value`, which keeps two full copies live during the
/// round-trip in addition to the DuckDB columnar buffer. A pathologically
/// large workspace (10+ MiB cargo-metadata output is possible) could OOM
/// the `ops about` process at this step. Cap the payload at 64 MiB by
/// default — well above realistic workspace sizes — and fail with a
/// clear error when exceeded so operators learn before the OS kills
/// the process. Override via `OPS_METADATA_MAX_BYTES`.
pub const METADATA_MAX_BYTES_DEFAULT: u64 = 64 * 1024 * 1024;

/// Environment variable that overrides [`METADATA_MAX_BYTES_DEFAULT`].
pub const METADATA_MAX_BYTES_ENV: &str = "OPS_METADATA_MAX_BYTES";

/// Resolved metadata payload byte cap. Non-numeric or zero values fall
/// back to [`METADATA_MAX_BYTES_DEFAULT`].
fn metadata_max_bytes() -> u64 {
    std::env::var(METADATA_MAX_BYTES_ENV)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(METADATA_MAX_BYTES_DEFAULT)
}

/// Run `cargo metadata --format-version 1 --locked`.
///
/// PATTERN-1 / TASK-1059: pass `--locked` so the read-only ingestor cannot
/// silently mutate `Cargo.lock` (resolver refresh, yanked-version refresh,
/// transitive-dep additions). Without it, two concurrent invocations
/// (`ops about` + `cargo build`) can race on lockfile rewrites and
/// reproducibility breaks (`data_sources.checksum` drifts between runs of
/// the same workspace). `--locked` fails fast if cargo would need to
/// update the lockfile, surfacing the drift rather than rewriting on the
/// operator's behalf. We prefer `--locked` over `--frozen` because the
/// latter additionally forbids network access, which can break first-run
/// metadata for fresh checkouts where the registry index has not yet been
/// downloaded — the operator-visible failure mode of `--frozen` is worse
/// than the lockfile-mutation issue we're guarding against.
pub(crate) fn run_cargo_metadata(working_dir: &Path) -> Result<Output, RunError> {
    run_cargo(
        &["metadata", "--format-version", "1", "--locked"],
        working_dir,
        CARGO_METADATA_TIMEOUT,
        "cargo metadata",
    )
}

/// PATTERN-1 / TASK-1099: include the numeric exit code (or `signal` for
/// `None`) in the error string so a SIGKILL/OOM kill is distinguishable
/// from a real cargo failure. Mirrors `interpret_deny_result` /
/// `interpret_upgrade_output` in the deps crate.
pub(crate) fn check_metadata_output(output: &Output) -> Result<(), anyhow::Error> {
    if !output.status.success() {
        let tail = format_error_tail(&output.stderr, 5);
        match output.status.code() {
            Some(code) => anyhow::bail!("cargo metadata exited with status {code}: {tail}"),
            None => anyhow::bail!("cargo metadata terminated by signal (exit_code = None): {tail}"),
        }
    }
    Ok(())
}

/// API-9 / TASK-0922: construct via the registered extension factory only.
#[non_exhaustive]
pub struct MetadataExtension;

ops_extension::impl_extension! {
    MetadataExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    stack: Some(ops_extension::Stack::Rust),
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |_self, registry| {
        registry.register(DATA_PROVIDER_NAME, Box::new(MetadataProvider));
    },
    factory: METADATA_FACTORY = |_, _| {
        Some((NAME, Box::new(MetadataExtension)))
    },
}

struct MetadataProvider;

impl DataProvider for MetadataProvider {
    fn name(&self) -> &'static str {
        DATA_PROVIDER_NAME
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        ops_duckdb::try_provide_from_db(ctx, provide_from_db, |ctx| provide_via_cargo_metadata(ctx))
    }

    fn schema(&self) -> DataProviderSchema {
        use ops_extension::data_field;
        DataProviderSchema::new(
            "Cargo workspace metadata from `cargo metadata`",
            vec![
                data_field!(
                    "workspace_root",
                    "str",
                    "Absolute path to the workspace root directory"
                ),
                data_field!(
                    "target_directory",
                    "str",
                    "Absolute path to the build artifacts directory"
                ),
                data_field!(
                    "build_directory",
                    "Option<str>",
                    "Build directory if specified via config"
                ),
                data_field!(
                    "packages",
                    "Iterator<Package>",
                    "All packages in the dependency graph"
                ),
                data_field!(
                    "members",
                    "Iterator<Package>",
                    "Workspace member packages only"
                ),
                data_field!(
                    "default_members",
                    "Iterator<Package>",
                    "Default workspace member packages"
                ),
                data_field!(
                    "root_package",
                    "Option<Package>",
                    "Root package (None for virtual workspaces)"
                ),
                data_field!(
                    "package_by_name",
                    "fn(&str) -> Option<Package>",
                    "Find a package by name"
                ),
                data_field!("Package.name", "str", "Package name"),
                data_field!("Package.version", "str", "Package version string"),
                data_field!("Package.edition", "str", "Rust edition (e.g., 2021)"),
                data_field!("Package.license", "Option<str>", "License identifier"),
                data_field!(
                    "Package.dependencies",
                    "Iterator<Dependency>",
                    "Normal dependencies"
                ),
                data_field!(
                    "Package.dev_dependencies",
                    "Iterator<Dependency>",
                    "Dev dependencies"
                ),
                data_field!(
                    "Package.build_dependencies",
                    "Iterator<Dependency>",
                    "Build dependencies"
                ),
                data_field!(
                    "Package.targets",
                    "Iterator<Target>",
                    "All build targets (lib, bins, tests, examples, benches)"
                ),
                data_field!("Dependency.name", "str", "Dependency name"),
                data_field!(
                    "Dependency.version_req",
                    "str",
                    "Version requirement (e.g., ^1.0)"
                ),
                data_field!("Dependency.kind", "enum", "Normal, Dev, or Build"),
                data_field!("Dependency.features", "Iterator<str>", "Enabled features"),
                data_field!("Target.name", "str", "Target name"),
                data_field!(
                    "Target.kind",
                    "Iterator<str>",
                    "Target kinds (lib, bin, test, example, bench)"
                ),
                data_field!("Target.src_path", "str", "Source file path"),
            ],
        )
    }
}

fn query_metadata_raw(db: &DuckDb) -> Result<serde_json::Value, anyhow::Error> {
    query_metadata_raw_with_cap(db, metadata_max_bytes())
}

fn query_metadata_raw_with_cap(db: &DuckDb, cap: u64) -> Result<serde_json::Value, anyhow::Error> {
    use anyhow::Context as AnyhowContext;
    let conn = db.lock().context("acquiring db lock for metadata query")?;
    // ERR-1 / TASK-0599: `metadata_raw` is a singleton table — the prior
    // `LIMIT 1` form silently picked an arbitrary row if a future ingest
    // path inserted more than one (re-collect without truncate, schema
    // version row). Read every row, assert one, and surface a clear error
    // if the invariant breaks.
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM metadata_raw",
            [],
            |row: &duckdb::Row| row.get(0),
        )
        .context("counting metadata_raw rows")?;
    anyhow::ensure!(
        count == 1,
        "metadata_raw must contain exactly one row, found {count}"
    );
    let json_text: String = conn
        .query_row(
            "SELECT to_json(m)::VARCHAR FROM metadata_raw m",
            [],
            |row: &duckdb::Row| row.get(0),
        )
        .context("reading from metadata_raw table")?;
    drop(conn);
    // ERR-1 / TASK-1034: bound the size of the JSON payload before the
    // `serde_json::from_str` call doubles allocation by materialising a
    // `serde_json::Value` tree. Above the cap, fail loudly with the
    // override hint rather than risk an OOM kill in `ops about`.
    let len = json_text.len() as u64;
    if len > cap {
        tracing::warn!(
            bytes = len,
            cap,
            env = METADATA_MAX_BYTES_ENV,
            "metadata_raw payload exceeds byte cap; aborting parse"
        );
        anyhow::bail!(
            "metadata_raw payload is {len} bytes, exceeds {cap}-byte cap \
             (override via {METADATA_MAX_BYTES_ENV})"
        );
    }
    let json: serde_json::Value =
        serde_json::from_str(&json_text).context("parsing metadata JSON")?;
    Ok(json)
}

fn provide_from_db(db: &DuckDb, ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    ops_duckdb::sql::provide_via_ingestor(
        db,
        ctx,
        "metadata_raw",
        &MetadataIngestor,
        query_metadata_raw,
    )
}

fn provide_via_cargo_metadata(ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    use anyhow::Context as _;
    let output = run_cargo_metadata(&ctx.working_directory)?;
    check_metadata_output(&output)?;
    // ERR-4 (TASK-0938): attribute parse failures to the cargo-metadata
    // pipeline so operators see "parsing cargo metadata stdout" in the
    // chain, not a bare serde_json::Error. Sister pattern to
    // `test-coverage::collect_coverage` (parsing llvm-cov JSON output).
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parsing cargo metadata stdout")?;
    Ok(json)
}
