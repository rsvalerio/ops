# Plan: a DuckDB CLI backend alongside the embedded one

## Why

`duckdb = { features = ["bundled"] }` compiles the DuckDB C++ amalgamation from
source on every cold build. Measured in this tree:

| | |
|---|---|
| `target/debug/.../libduckdb.a` | 2.03 GB |
| `target/release/.../libduckdb.a` | 118 MB |
| `target/release/ops` | 51 MB |
| `libduckdb-sys-*` build dirs in `target/debug` | 12, ~3.8 GB each (~21 GB of a 60 GB `target/`) |

`~/.cargo/config.toml` sets `rustc-wrapper = "sccache"`, which caches *rustc*
invocations only. The cc-rs C++ compilation is not cached, so sccache has never
helped here — locally or in CI. CI runs five jobs (`check`, `build`, `test`,
`msrv`, `deps`) on fresh runners with `--all-features`; each pays the full C++
build. Releases add four more (one per target).

Shelling out to the `duckdb` CLI removes that compile entirely while keeping the
real DuckDB engine.

## Why not SQLite, and why not Docker

**Not SQLite — but see the caveat.** The whole ingest path is:

```
tool emits JSON → sidecar file → CREATE OR REPLACE TABLE … AS SELECT * FROM read_json_auto(path) → CREATE OR REPLACE VIEW …
```

There is no Rust-side data model for any of it. The only `serde_json::from_*`
calls in the ingest path are `extensions-rust/metadata/src/lib.rs:304` and
`:327`, and both produce an untyped `serde_json::Value` — one for a byte-cap
guard, one as passthrough of `cargo metadata` stdout.

`crate_dependencies_view_sql` (`extensions-rust/metadata/src/views.rs:29`) does
`unnest(packages)`, `unnest(pkg.dependencies)`, then struct field access on
`pkg.name`, `pkg.manifest_path`, `dep.req`, `dep.kind`, `dep.optional`,
`dep.target` — nested LIST-of-STRUCT navigation over cargo's `metadata.json`
with the schema inferred at read time.

An earlier draft of this document claimed SQLite could not express that without
hand-written typed models. **That was wrong.** SQLite's JSON1 `json_each` +
`json_extract` reproduces the view faithfully — verified against this
workspace's real `cargo metadata` output, including the `cfg(unix)` target rows
that PATTERN-1/TASK-1056 exists to preserve. The trade is explicit
`json_extract` paths instead of inferred columns, which is if anything *more*
tolerant of upstream drift (a missing path yields NULL rather than a missing
column). See [`duckdb-alternatives.md`](duckdb-alternatives.md) — a SQLite
migration removes 140 of the workspace's 259 crates and the C++ build, and may
well be less work than this plan.

**Not Docker.** The CLI binary gives the same build win with the same engine,
minus a daemon dependency, bind mounts, host↔container path translation, and
per-invocation container start. It is also a single self-contained binary that
can be vendored into the cargo-dist archives next to `ops`, which a Docker image
cannot.

## The coupling is thinner than it looks

A raw grep for `duckdb::` returns 152 hits, but most are `ops_duckdb::` — our own
crate. Actual `duckdb` crate API surface:

| symbol | sites |
|---|---|
| `duckdb::Connection` | 16 |
| `duckdb::Row` | 11 |
| `duckdb::Error` | 11 |
| `duckdb::params` | 6 |
| `duckdb::params_from_iter` | 2 |
| `duckdb::Config` / `AccessMode` | 2 |

Connection methods actually called: `execute_batch` (35 sites, 9 outside tests),
`execute` (14), `query_row` (3), `prepare` + `query_map` (6 sites across 4 files).
That is the entire engine contract.

Only **three files outside `extensions/duckdb`** touch the `duckdb` crate:
`extensions-rust/metadata/src/lib.rs`, `extensions-rust/metadata/src/ingestor.rs`,
`extensions-rust/test-coverage/src/provider.rs`.

Three properties of the existing design make an out-of-process engine viable:

1. **SQL is already plain strings.** `create_table_from_json_sql`,
   `tokei_languages_view_sql`, `coverage_summary_view_sql` and friends never
   touch a `Connection`. They need no changes at all.
2. **Data already flows through files.** Sidecar JSON under a single
   `data_dir_for_db(db.path())` root; database at `target/ops/data.duckdb`.
   Nothing is passed in memory.
3. **Results are already materialised.** `query_rows_fold`,
   `collect_per_crate_map` and `query_rows_to_json` each fold the full result
   into a `HashMap`/`Vec`/`Value`. No consumer streams.

Point 3 matters: it means the backend trait can return an owned result set,
which a subprocess can produce and a `Connection` can too.

## Design

One trait, minimal, in `extensions/duckdb/src/backend.rs`:

```rust
pub trait Backend: Send + Sync {
    fn execute_batch(&self, sql: &str) -> DbResult<()>;
    fn execute(&self, sql: &str, params: &[SqlValue<'_>]) -> DbResult<usize>;
    fn query(&self, sql: &str, params: &[SqlValue<'_>]) -> DbResult<Rows>;
}
```

with owned `Rows` / `Row` types and `Row::get::<T>(idx)`, plus a `SqlValue`
enum (`Str` / `I64` / `F64` / `Null`).

`DuckDb` keeps its name and most of its public API but holds
`Box<dyn Backend>` instead of `Mutex<duckdb::Connection>`. Consumers outside the
crate barely move.

### The one thing that must change regardless of backend

`DuckDb::lock()` is public and returns `MutexGuard<'_, duckdb::Connection>`, and
`PerCrateSetup::Ready` carries that guard across a module boundary
(`sql/query/helpers.rs`). Under a CLI backend there is no `Connection` to hand
out, so `lock()` cannot be feature-gated — it has to go from the public API
outright, replaced by `execute`/`query` methods on `DuckDb`.

**This is worth doing on its own merits.** `lock()` is precisely what leaks
`duckdb::Connection`, `duckdb::Row` and `duckdb::Error` into 20-odd closure
signatures across the tree. Sealing it is a good change even if the CLI backend
is never shipped.

### Parameters

The DuckDB CLI has no bind-parameter mechanism over stdin. There are only six
non-test binding sites:

- `schema.rs:49` `params![source_name, workspace_root]`
- `schema.rs:170` (upsert)
- `sql/ingest/sql.rs:59` `params![table_name, table_name]` (validated identifier)
- `sql/query/coverage.rs:129` `params![filename, count, covered]`
- `extensions-rust/metadata/src/lib.rs:278` `params![cap as i64]`
- `sql/query/helpers.rs:297` `params_from_iter(member_paths)` (already
  `validate_path_chars`-checked)

Approach: the CLI backend renders `SqlValue` to a SQL literal through a single
escaping function, reusing the existing `escape_sql_string` in
`sql/validation.rs`. This is not a new risk class — the codebase already
interpolates escaped paths into `read_json_auto` and has 768 lines of validation
plus tests covering exactly that. Keep the escaper private to the backend layer
and property-test it.

Rejected alternative: writing params to a temp JSON file and reading them via
`read_json_auto`. It avoids interpolation entirely but forces the two backends
to run *different SQL* for parameterized queries, which doubles the surface the
tests have to pin. Not worth it for six sites.

### In-memory databases

This is the sharpest constraint. There are **103 `open_in_memory()` call sites
across 19 files**. Each CLI invocation is a fresh process, so `:memory:` cannot
carry state between statements.

Resolution: under the `cli` feature, `DuckDb::open_in_memory()` opens a
temp-file-backed database deleted on drop. Semantically equivalent for every
current test — none of them depend on the database being non-durable, only on it
being isolated and empty.

That keeps the model "one CLI invocation per operation, state lives in the
file", which needs no output framing protocol. A persistent child process with
stdin held open would amortise startup and give true session semantics, but it
needs a delimiter protocol to know where one statement's output ends. Defer it
to Phase 5, and only if measurement justifies it.

### Version and file format

The `.duckdb` storage format is versioned; a mismatched CLI cannot read a file
another version wrote. Two mitigations:

- Check `duckdb --version` at open, compare against a pinned expected version,
  fail with an actionable message.
- **The database is a cache, not a source of truth.** It lives under
  `target/ops/`, is gitignored, and every byte is re-derivable from
  `cargo metadata`, `tokei` and `llvm-cov`. On format mismatch, delete and
  re-ingest. That removes most of the risk.

### Binary discovery

Resolution order: `OPS_DUCKDB_BIN` env → a new `duckdb_bin` key on `DataConfig`
(`crates/core/src/config/sections.rs:44`, currently a single-field struct) →
`PATH` → a copy vendored next to the `ops` binary by cargo-dist.

## Feature flag: feasible

`embedded` (default) and `cli`, mutually exclusive, with a `compile_error!` when
both or neither are set. The `Backend` trait is what makes this work — once
Phase 1 seals the abstraction, the two impls are genuinely interchangeable.

Costs to accept:

- The CI matrix grows for the duckdb-touching crates. The `cli` job is fast; the
  `embedded` job is what you already pay today.
- Two backends means two things that can drift. Mitigated by running the same
  test suite against both.

## Phases

**Phase 0 — cache and measure (independent, do first).**
Wrap the C++ compiler for CI (`CC="sccache gcc" CXX="sccache g++"`) or
`actions/cache` on `target/*/build/libduckdb-sys-*/out` keyed on the duckdb
version. Config-only, no code churn, biggest immediate win. Record a baseline
cold-build time so later phases have something to compare against.

**Phase 1 — seal the abstraction (no behaviour change, still embedded).**
Remove `DuckDb::lock()` from the public API. Introduce `ops_duckdb::Row`,
`SqlValue` and backend-owned errors; convert the 11 `duckdb::Row` signatures and
the `duckdb::Error` matches (including `QueryReturnedNoRows` at `schema.rs:53`).
Rework `PerCrateSetup::Ready` so it no longer carries a `MutexGuard`. Fix the
three external files. **Ships on its own; valuable independent of the outcome.**

**Phase 2 — introduce `Backend`.**
Move the current `Mutex<Connection>` implementation behind
`EmbeddedBackend: Backend`. Still one backend, still default. Green suite here
means the boundary is right.

**Phase 3 — implement `CliBackend` behind the feature flag.**
Subprocess invocation with `.mode json`, `SqlValue` literal rendering, version
check, binary discovery, temp-file `open_in_memory()`. Run the full existing
suite under `--features cli`.

**Phase 4 — distribution.**
Vendor the `duckdb` binary into the cargo-dist archives; extend the Homebrew
formula with a `duckdb` dependency or the vendored copy. Add the CI job matrix
entry.

**Phase 5 — decide.**
Measure per-invocation overhead on a real `ops about` run. Then either flip the
default to `cli` and delete the embedded backend, or keep both deliberately. Do
not drift into carrying two backends by default — that is the outcome with all
of the cost and none of the benefit.

## Explicitly not doing

- **Docker.** Covered above: a daemon and path translation for no advantage over
  the bare binary.
- **Replacing DuckDB with SQLite or a hand-rolled store.** Schema inference over
  nested JSON is the reason DuckDB is here.
- **Linking a prebuilt `libduckdb`** — same build win, zero code change, keeps
  everything in-process. Written up separately in
  [`duckdb-prebuilt-lib.md`](duckdb-prebuilt-lib.md), and it should be tried
  **before** this plan: its Phase 0 is a few hours and answers the only hard
  question. Come back here only if the static link proves intractable.
