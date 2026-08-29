//! Metadata extension: runs `cargo metadata` and provides workspace info as JSON.
//! `DuckDB` is the single source of truth - metadata is loaded into `metadata_raw` table.
//!
//! # No typed accessor layer (ARCH-9 / TASK-1898)
//!
//! This crate used to also export a `Metadata` / `Package` / `Dependency` /
//! `Target` wrapper family (`types.rs`, ~660 lines plus ~750 lines of tests)
//! offering typed accessors over the same JSON. It had **no consumers**:
//! nothing in the workspace named any of those types, `Metadata::from_context`
//! — the documented production entry point — was never called, and nine
//! blanket `#[allow(dead_code)]` attributes were what kept the question open.
//! Everything that actually ships goes through
//! `MetadataProvider::provide` → `provide_from_db` → `query_metadata_raw`,
//! which returns a raw `serde_json::Value`.
//!
//! The decision recorded here is **removed, not deferred**: `ops-metadata` is
//! `publish = false`, its only cross-crate reference is the
//! `extern crate ops_metadata;` in `crates/cli/src/main.rs` that exists so the
//! `linkme` factory registers, and no backlog item plans a consumer. If a
//! consumer ever appears, the wrappers are cheap to reintroduce against a real
//! call site — which is also the only way their contract can be tested for
//! something other than `serde_json::Value::get`. Until then the provider's
//! JSON shape is documented by [`MetadataProvider::schema`], and consumers
//! read it with `serde_json` directly.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

mod ingestor;
#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
mod views;

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
pub(crate) const CARGO_METADATA_TIMEOUT: Duration = Duration::from_mins(2);

/// ERR-1 / TASK-1034: byte cap on the JSON payload read from
/// `metadata_raw`.
///
/// `query_metadata_raw` materialises the row as a `String` (via
/// `to_json(m)::VARCHAR`) and then parses it into a `serde_json::Value`,
/// which keeps two full copies live during the round-trip in addition to
/// the `DuckDB` columnar buffer. A pathologically large workspace (10+ MiB
/// cargo-metadata output is possible) could OOM the `ops about` process at
/// this step. Cap the payload at 64 MiB by default — well above realistic
/// workspace sizes — and fail with a clear error when exceeded so operators
/// learn before the OS kills the process. Override via
/// `OPS_METADATA_MAX_BYTES`.
pub const METADATA_MAX_BYTES_DEFAULT: u64 = 64 * 1024 * 1024;

/// Environment variable that overrides [`METADATA_MAX_BYTES_DEFAULT`].
pub const METADATA_MAX_BYTES_ENV: &str = "OPS_METADATA_MAX_BYTES";

/// SEC-11 / TASK-1897: hard ceiling on the resolved cap.
///
/// ARCH-9 / TASK-1247 unified the post-ingest reader cap and the
/// ingest-time `maximum_object_size` ceiling on this one env knob, and that
/// unification is only sound over the range **both** consumers accept.
/// `DuckDB` types `read_json`'s `maximum_object_size` as `UINTEGER`
/// (32-bit), so anything above `u32::MAX` does not raise the ingest
/// ceiling — it makes the `CREATE TABLE … read_json_auto(…)` statement fail
/// with an option-conversion error attributed to `"metadata_raw create"`,
/// naming nothing the operator set.
///
/// Verified against the pinned `DuckDB` v1.5.5 (`scripts/duckdb-pins.txt`):
/// `maximum_object_size=4294967295` is accepted, while `=4294967296` fails
/// with *"Type INT64 with value 4294967296 can't be cast because the value
/// is out of range for the destination type UINT32"*.
///
/// Spelled as a literal because `u64::from` is not callable in a `const`
/// initialiser and `u32::MAX as u64` would need an `as_conversions`
/// exception (`docs/clippy.md`); the equality with `u32::MAX` is pinned by
/// `ceiling_is_exactly_duckdb_uinteger_max` in `tests/payload_cap.rs`.
pub const METADATA_MAX_BYTES_CEILING: u64 = 4_294_967_295;

/// SEC-11 / TASK-1897: validate and bound the raw `OPS_METADATA_MAX_BYTES`
/// value at the boundary, warning on every value that is not honoured
/// verbatim.
///
/// Previously any unparseable, zero, or oversized value resolved silently to
/// [`METADATA_MAX_BYTES_DEFAULT`]: an operator who raised the cap to work
/// around an over-cap failure saw the identical failure with no signal that
/// the knob had been ignored, and a value above [`METADATA_MAX_BYTES_CEILING`]
/// broke ingest instead of raising it.
///
/// Split out from [`metadata_max_bytes`] so tests can drive every branch
/// without mutating process-global env (the `OnceLock` snapshot can be
/// initialised exactly once per process).
pub(crate) fn resolve_metadata_max_bytes(raw: Option<&str>) -> u64 {
    let Some(raw) = raw else {
        return METADATA_MAX_BYTES_DEFAULT;
    };
    let Ok(parsed) = raw.trim().parse::<u64>() else {
        tracing::warn!(
            env = METADATA_MAX_BYTES_ENV,
            value = raw,
            default = METADATA_MAX_BYTES_DEFAULT,
            "value is not a non-negative integer byte count; using the default cap"
        );
        return METADATA_MAX_BYTES_DEFAULT;
    };
    if parsed == 0 {
        tracing::warn!(
            env = METADATA_MAX_BYTES_ENV,
            value = raw,
            default = METADATA_MAX_BYTES_DEFAULT,
            "a zero byte cap would reject every payload; using the default cap"
        );
        return METADATA_MAX_BYTES_DEFAULT;
    }
    if parsed > METADATA_MAX_BYTES_CEILING {
        tracing::warn!(
            env = METADATA_MAX_BYTES_ENV,
            value = raw,
            ceiling = METADATA_MAX_BYTES_CEILING,
            "value exceeds DuckDB's UINTEGER maximum_object_size domain; clamping to the ceiling"
        );
        return METADATA_MAX_BYTES_CEILING;
    }
    parsed
}

/// Resolved metadata payload byte cap. See [`resolve_metadata_max_bytes`]
/// for the validation and clamping policy.
///
/// PERF-3 / TASK-1248: cached behind `OnceLock<u64>` to mirror the
/// `manifest_max_bytes` / `output_byte_cap` discipline; the env knob is
/// process-global so re-reading on every `provide_from_db` was wasted
/// work. The snapshot is also the single moment a diagnostic can be
/// emitted, which is why the warnings live in the resolver.
pub(crate) fn metadata_max_bytes() -> u64 {
    use std::sync::OnceLock;
    static CACHED: OnceLock<u64> = OnceLock::new();
    *CACHED.get_or_init(|| {
        resolve_metadata_max_bytes(std::env::var(METADATA_MAX_BYTES_ENV).ok().as_deref())
    })
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
        &CARGO_METADATA_ARGS,
        working_dir,
        CARGO_METADATA_TIMEOUT,
        "cargo metadata",
    )
}

/// TEST-25 / TASK-1899: the argument list [`run_cargo_metadata`] passes,
/// named so a test can assert on the *value* the production call site uses.
///
/// The previous pin read `include_str!("../lib.rs")` and searched it for the
/// literal `["metadata", "--format-version", "1", "--locked"]`. That tested
/// the formatter: `cargo fmt` wrapping the list would fail it while
/// `--locked` was still passed, and deleting `run_cargo_metadata` outright
/// would keep it green so long as the literal survived anywhere in the file
/// — a doc comment included.
pub(crate) const CARGO_METADATA_ARGS: [&str; 4] = ["metadata", "--format-version", "1", "--locked"];

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
///
/// API-1 / TASK-1549: derives `Debug` so the unit struct can be included in
/// `tracing::debug!(?ext)` and assertion failure output.
#[derive(Debug)]
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
        let _ = registry.register(DATA_PROVIDER_NAME, Box::new(MetadataProvider));
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

/// SEC-33 / TASK-1194 + PERF-3 / TASK-1551: bound the JSON payload size
/// **before** materialising the full row into a Rust `String`, in a single
/// SQL round trip. The payload is replaced with `NULL` when over cap so it
/// never crosses the FFI boundary into a Rust `String`; SEC-33's intent (no
/// oversized Rust allocation) and the bail-with-byte-count behaviour are
/// both preserved.
///
/// READ-1 / TASK-1896 — the serialisation cost, stated precisely. This SQL
/// spells `to_json(m)::VARCHAR` three times (twice inside `octet_length`,
/// once in the CASE's ELSE branch), but the expression is evaluated **once
/// per row**. That is a property of `DuckDB`'s common-subexpression
/// elimination, not of the SQL text, so it is pinned by a test rather than
/// assumed: `cap_guard_sql_serialises_to_json_once` (`tests/payload_cap.rs`)
/// reads the `EXPLAIN` physical plan and asserts a single `to_json`
/// projection node. Measured on the pinned `DuckDB` v1.5.5
/// (`scripts/duckdb-pins.txt`), the plan collapses to one
/// `CAST(to_json(struct_pack(...)) AS VARCHAR)` PROJECTION whose output the
/// node above references as `#0`. A hand-written
/// `WITH j AS (SELECT to_json(m)::VARCHAR AS txt …)` CTE was measured
/// against this shape on a 32 MiB row and was not faster — `DuckDB` inlines
/// the CTE and the extra projection layer costs more than it saves — so the
/// simpler form stays.
const CAP_GUARD_SQL: &str = "SELECT octet_length(CAST(to_json(m)::VARCHAR AS BLOB)) AS bytes, \
            CASE WHEN octet_length(CAST(to_json(m)::VARCHAR AS BLOB)) > ? \
                 THEN NULL ELSE to_json(m)::VARCHAR END AS payload \
     FROM metadata_raw m";

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
            |row: &duckdb::Row<'_>| row.get(0),
        )
        .context("counting metadata_raw rows")?;
    anyhow::ensure!(
        count == 1,
        "metadata_raw must contain exactly one row, found {count}"
    );
    let (len, json_text): (i64, Option<String>) = conn
        .query_row(
            CAP_GUARD_SQL,
            duckdb::params![i64::try_from(cap).unwrap_or(i64::MAX)],
            |row: &duckdb::Row<'_>| Ok((row.get(0)?, row.get(1)?)),
        )
        .context("reading metadata_raw payload with cap guard")?;
    drop(conn);
    // READ-5 / TASK-1550: a negative `octet_length` is not a real DuckDB
    // shape — treat any negative i64 as zero-length so the over-cap branch
    // cannot fire on a sentinel. Overflow on i64 → u64 is impossible after
    // the `.try_from(len)` succeeds, so we no longer carry a `u64::MAX`
    // arm whose policy would have been ambiguous.
    let len = u64::try_from(len).unwrap_or(0);
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
    let json_text =
        json_text.ok_or_else(|| anyhow::anyhow!("metadata_raw payload missing under cap"))?;
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
    let output = run_cargo_metadata(ctx.working_directory())?;
    check_metadata_output(&output)?;
    // ERR-4 (TASK-0938): attribute parse failures to the cargo-metadata
    // pipeline so operators see "parsing cargo metadata stdout" in the
    // chain, not a bare serde_json::Error. Sister pattern to
    // `test-coverage::collect_coverage` (parsing llvm-cov JSON output).
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parsing cargo metadata stdout")?;
    Ok(json)
}
