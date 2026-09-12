//! Metadata extension: runs `cargo metadata` and provides workspace info as JSON.
//! `SQLite` is the single source of truth - metadata is loaded into `metadata_raw` table.
//!
//! # Consuming the metadata
//!
//! The crate exposes the workspace as raw JSON, not as a typed wrapper family.
//! Everything ships through `MetadataProvider::provide` → `provide_from_db` →
//! `query_metadata_raw`, which returns a `serde_json::Value`. The shape of that
//! value is documented by `MetadataProvider::schema`; consumers read it with
//! `serde_json` directly.

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
use ops_extension::{Context, DataProvider, DataProviderError, DataProviderSchema, ExtensionType};
use ops_sqlite::Sqlite;
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

/// Default byte cap on the JSON payload read back from `metadata_raw`.
///
/// `query_metadata_raw` materialises the row as a `String` (via
/// `to_json(m)::VARCHAR`) and then parses it into a `serde_json::Value`,
/// which keeps two full copies live during the round-trip in addition to
/// the `SQLite` columnar buffer. A pathologically large workspace (10+ MiB
/// cargo-metadata output is possible) could OOM the `ops about` process at
/// this step. Cap the payload at 64 MiB by default — well above realistic
/// workspace sizes — and fail with a clear error when exceeded so operators
/// learn before the OS kills the process. Override via
/// `OPS_METADATA_MAX_BYTES`.
///
/// This cap governs the **post-ingest read only**. On the
/// collect side the subprocess capture cap (4 MiB default,
/// `OPS_OUTPUT_BYTE_CAP`) binds first — see [`metadata_output_cap`] — so a
/// workspace above that cap never reaches this one until `OPS_OUTPUT_BYTE_CAP`
/// is raised; the 64 MiB budget here is not a claim about what `cargo
/// metadata` output can reach the reader.
pub(crate) const METADATA_MAX_BYTES_DEFAULT: u64 = 64 * 1024 * 1024;

/// Environment variable that overrides [`METADATA_MAX_BYTES_DEFAULT`].
pub(crate) const METADATA_MAX_BYTES_ENV: &str = "OPS_METADATA_MAX_BYTES";

/// The per-stream capture cap that bounds [`run_cargo_metadata`]'s stdout.
///
/// Resolved exactly the way `ops_core::subprocess` resolves it (same env var,
/// same default, same clamping via [`ops_core::text::cached_byte_cap_env`]), so
/// the value the guard compares against is the value the drain threads
/// enforced.
///
/// The subprocess cap (4 MiB default,
/// `OPS_OUTPUT_BYTE_CAP`) binds **before** [`METADATA_MAX_BYTES_DEFAULT`]
/// ever can — the drain discards bytes past its cap and returns a truncated
/// buffer that never exceeds 4 MiB, so the 64 MiB reader cap governs only
/// the post-ingest read. A workspace whose `cargo metadata` output exceeds
/// the subprocess cap must raise `OPS_OUTPUT_BYTE_CAP`; the guard below
/// refuses the truncated buffer instead of parsing it.
fn metadata_output_cap() -> u64 {
    static CAP: std::sync::OnceLock<u64> = std::sync::OnceLock::new();
    ops_core::text::cached_byte_cap_env(
        &CAP,
        ops_core::subprocess::OUTPUT_CAP_ENV,
        u64::try_from(ops_core::subprocess::DEFAULT_OUTPUT_BYTE_CAP).unwrap_or(u64::MAX),
    )
}

/// Refuses a `cargo metadata` stdout that hit the subprocess capture cap,
/// rather than parsing (or staging) a silently truncated document.
///
/// `run_with_timeout`'s drain threads bound each stream at the cap and
/// treat truncation as a `warn`-level breadcrumb, not an error — so a
/// caller that goes on to `serde_json::from_slice` the buffer fails with a
/// misattributed parse error, or worse stages the truncated bytes as
/// ground truth. A captured length at the cap is the truncation signal
/// available on `std::process::Output`: the drain never returns more than
/// the cap, so a document that *genuinely* ends exactly at the cap is
/// refused too — the conservative direction for a gate that certifies
/// workspace data.
pub(crate) fn check_metadata_not_capped(output: &Output) -> Result<(), anyhow::Error> {
    let cap = metadata_output_cap();
    let kept = u64::try_from(output.stdout.len()).unwrap_or(u64::MAX);
    if kept >= cap {
        tracing::warn!(
            kept_bytes = kept,
            cap,
            env = ops_core::subprocess::OUTPUT_CAP_ENV,
            "cargo metadata stdout hit the subprocess capture cap; refusing to parse it"
        );
        anyhow::bail!(
            "cargo metadata stdout reached the {cap}-byte subprocess capture cap \
             ({kept} bytes kept, anything beyond was discarded); refusing to parse a \
             possibly-truncated document — raise the cap via \
             {} if the workspace genuinely produces metadata this large",
            ops_core::subprocess::OUTPUT_CAP_ENV
        );
    }
    Ok(())
}

/// Hard ceiling on the resolved cap.
///
/// One env knob drives both the ingest-side capped read and the post-ingest
/// reader guard, and neither consumer has an engine-imposed domain anymore:
/// the ingest side compares `str::len()` against a `usize`, and the read
/// side binds the cap as an i64 SQL parameter (`i64::try_from(cap)` with a
/// saturating fallback), so both accept any `u64` the resolver can produce.
///
/// The ceiling that remains is **policy**, not an engine limit: a knob
/// value above 4 GiB is almost certainly a typo or an attempt to disable
/// the payload guard rather than a real metadata document size, and an
/// unbounded knob would silently disable the SEC-33 cap (see
/// `above_ceiling_warns_and_clamps` in `tests/payload_cap.rs`). The
/// historical value (`u32::MAX`) is kept from the `DuckDB` era so existing
/// deployments that reasoned about the old limit see no behavior change.
///
/// Spelled as a literal because `u64::from` is not callable in a `const`
/// initialiser and `u32::MAX as u64` would need an `as_conversions`
/// exception (`docs/clippy.md`); the equality with `u32::MAX` is pinned by
/// `ceiling_is_exactly_u32_max` in `tests/payload_cap.rs`.
pub(crate) const METADATA_MAX_BYTES_CEILING: u64 = 4_294_967_295;

/// Validates and bounds the raw `OPS_METADATA_MAX_BYTES` value at the
/// boundary, warning on every value that is not honoured verbatim.
///
/// An unparseable or zero value falls back to [`METADATA_MAX_BYTES_DEFAULT`]
/// and a value above [`METADATA_MAX_BYTES_CEILING`] is clamped to it — in both
/// cases with a warning, so an operator who raised the cap to work around an
/// over-cap failure learns that the knob was not honoured instead of seeing the
/// identical failure with no signal.
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
            "value exceeds the sanity ceiling; clamping to the ceiling"
        );
        return METADATA_MAX_BYTES_CEILING;
    }
    parsed
}

/// Resolved metadata payload byte cap. See [`resolve_metadata_max_bytes`]
/// for the validation and clamping policy.
///
/// Cached behind a `OnceLock<u64>`, mirroring the `manifest_max_bytes` /
/// `output_byte_cap` discipline: the env knob is process-global, so it is read
/// once rather than on every `provide_from_db`. That snapshot is also the
/// single moment a diagnostic can be emitted, which is why the warnings live in
/// the resolver.
pub(crate) fn metadata_max_bytes() -> u64 {
    use std::sync::OnceLock;
    static CACHED: OnceLock<u64> = OnceLock::new();
    *CACHED.get_or_init(|| {
        resolve_metadata_max_bytes(std::env::var(METADATA_MAX_BYTES_ENV).ok().as_deref())
    })
}

/// Run `cargo metadata --format-version 1 --locked`.
///
/// `--locked` is passed so the read-only ingestor cannot
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

/// The argument list [`run_cargo_metadata`] passes to `cargo`.
///
/// It is a named constant so a test can assert on the *value* the production
/// call site uses. Asserting on the source text instead would test the
/// formatter — `cargo fmt` rewrapping the list would fail while `--locked` was
/// still passed, and deleting the call site would stay green so long as the
/// literal survived anywhere in the file, a doc comment included.
pub(crate) const CARGO_METADATA_ARGS: [&str; 4] = ["metadata", "--format-version", "1", "--locked"];

/// Turns a non-zero `cargo metadata` exit into an error naming the failure.
///
/// The error string carries the numeric exit code (or `signal` when there is
/// none) so a SIGKILL/OOM kill is distinguishable from a real cargo failure.
/// Mirrors `interpret_deny_result` / `interpret_upgrade_output` in the deps
/// crate.
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

/// Extension entry point for `ops metadata`.
///
/// Construct it through the registered extension factory, not directly. It
/// derives `Debug` so the unit struct can be included in `tracing::debug!(?ext)`
/// and in assertion failure output.
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
        ops_sqlite::try_provide_from_db(ctx, provide_from_db, |ctx| provide_via_cargo_metadata(ctx))
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

/// Bounds the JSON payload size **before** materialising the full blob into a
/// Rust `String`, in a single SQL round trip.
///
/// The payload is replaced with `NULL` when over cap, so an oversized document
/// never crosses the FFI boundary into a Rust allocation, and the caller still
/// bails with the observed byte count.
///
/// SQLite port note: the blob is already JSON text in a `json TEXT NOT NULL`
/// column, so no serialisation happens at all — the guard is a pure byte
/// count. `length()` on TEXT counts *characters*, hence the
/// `CAST(m.json AS BLOB)`: on a BLOB, `length()` counts bytes, which is the
/// unit `OPS_METADATA_MAX_BYTES` promises (and matches the Rust-side
/// `str::len()` check the ingestor applies to the same payload before it is
/// staged into the table).
const CAP_GUARD_SQL: &str = "SELECT length(CAST(m.json AS BLOB)) AS bytes, \
            CASE WHEN length(CAST(m.json AS BLOB)) > ? \
                 THEN NULL ELSE m.json END AS payload \
     FROM metadata_raw m";

fn query_metadata_raw(db: &Sqlite) -> Result<serde_json::Value, anyhow::Error> {
    query_metadata_raw_with_cap(db, metadata_max_bytes())
}

fn query_metadata_raw_with_cap(db: &Sqlite, cap: u64) -> Result<serde_json::Value, anyhow::Error> {
    use anyhow::Context as AnyhowContext;
    let conn = db.lock().context("acquiring db lock for metadata query")?;
    // ERR-1: `metadata_raw` is a singleton table. Counting every row and
    // asserting exactly one surfaces a clear error if a future ingest path
    // inserts more (re-collect without truncate, a schema version row); a
    // `LIMIT 1` read would silently pick an arbitrary one instead.
    let count: i64 = conn
        .query_row(
            "SELECT count(*) FROM metadata_raw",
            [],
            |row: &rusqlite::Row<'_>| row.get(0),
        )
        .context("counting metadata_raw rows")?;
    anyhow::ensure!(
        count == 1,
        "metadata_raw must contain exactly one row, found {count}"
    );
    let (len, json_text): (i64, Option<String>) = conn
        .query_row(
            CAP_GUARD_SQL,
            rusqlite::params![i64::try_from(cap).unwrap_or(i64::MAX)],
            |row: &rusqlite::Row<'_>| Ok((row.get(0)?, row.get(1)?)),
        )
        .context("reading metadata_raw payload with cap guard")?;
    drop(conn);
    // READ-5 / TASK-1550: a negative byte length from the `length(CAST(…
    // AS BLOB))` guard is not a real SQLite shape — treat any negative i64
    // as zero-length so the over-cap branch cannot fire on a sentinel.
    // Overflow on i64 → u64 is impossible after the `.try_from(len)`
    // succeeds, so we no longer carry a `u64::MAX` arm whose policy would
    // have been ambiguous.
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

fn provide_from_db(db: &Sqlite, ctx: &Context) -> Result<serde_json::Value, anyhow::Error> {
    ops_sqlite::sql::provide_via_ingestor(
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
    // ERR-1 / TASK-2188: a capped stdout is a truncated document — refuse it
    // here rather than letting serde fail below with a misattributed parse
    // error naming neither the cap nor its env var.
    check_metadata_not_capped(&output)?;
    // ERR-4 (TASK-0938): attribute parse failures to the cargo-metadata
    // pipeline so operators see "parsing cargo metadata stdout" in the
    // chain, not a bare serde_json::Error. Sister pattern to
    // `test-coverage::collect_coverage` (parsing llvm-cov JSON output).
    let json: serde_json::Value =
        serde_json::from_slice(&output.stdout).context("parsing cargo metadata stdout")?;
    Ok(json)
}
