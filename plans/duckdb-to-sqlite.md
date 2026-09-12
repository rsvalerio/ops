# Plan: Migrate ops from DuckDB to rusqlite (SQLite)

> **Status: executed 2026-09-12** (PR #54). This is the historical plan, kept
> verbatim as approved. The 354-row count below was measured when the plan was
> written; the merged migration re-verified the same view at 377 rows — the
> workspace had grown in between. The "first implementation action" note no
> longer applies: this file already is the artifact it describes.

## Context

The workspace links a prebuilt libduckdb (`scripts/fetch-duckdb.sh`, SHA-pinned) instead of compiling the C++ amalgamation. That solved the compile cost but left real friction: every linking build needs `eval "$(scripts/fetch-duckdb.sh)"` in the invoking shell (the pre-commit hook fails otherwise), releases need a 23-archive static link dance, and DuckDB still drags in **140 of 259 workspace crates (54%, incl. 11 arrow crates)**.

`docs/duckdb-alternatives.md` already verified the migration's hardest piece: the `crate_dependencies` view ported to SQLite JSON1 reproduces all 354 rows from this workspace's real `cargo metadata`, including the `cfg(unix)` target rows. The workload is tiny (354 rows from 100 KB JSON; the DB under `target/ops/` is a disposable cache) — nothing needs a columnar engine.

**Decisions made with the user:**
- Crate renamed: `extensions/duckdb` (`ops-duckdb`) → `extensions/sqlite` (`ops-sqlite`).
- The prebuilt-DuckDB machinery (fetch script, pins, CI/release wiring, AGENTS.md eval instruction) is **deleted in this same migration**, not deferred.

`rusqlite` (bundled) is a plain-C build (~1m12s cold release, verified in the doc), caches under sccache, needs no link env, and `duckdb-rs` is a fork of `rusqlite` — the engine call sites are near drop-in.

## What actually uses DuckDB (explored)

| Surface | Location | Port cost |
|---|---|---|
| Handle wrapper `DuckDb` (open/readonly/in_memory/`lock()` → `Mutex<Connection>`) | `extensions/duckdb/src/connection.rs` | Mechanical (rusqlite same API) |
| `data_sources` tracking table DDL | `extensions/duckdb/src/schema.rs` | Types: `VARCHAR/TIMESTAMP/BIGINT/JSON` → `TEXT` |
| Ingest: `CREATE TABLE … AS SELECT * FROM read_json_auto('<path>')` — **one builder**, `sql/ingest/sql.rs:140-155` | `sql/ingest/sql.rs` | **Redesign** → parameter-bound inserts (below) |
| Staging dir (`IngestDir`: atomic writes, checksums, inode verification) | `sql/ingest/dir.rs` (~1400 lines) | **None** — engine-agnostic, survives verbatim |
| SQL identifier/path validation (`TableName`, `quoted_ident`, …) | `sql/validation.rs` (791 lines) | Mostly survives; `ExtraOpts`/`validate_extra_opts` die |
| 4 views | `extensions/tokei/src/views.rs`, `extensions-rust/{metadata,loc,test-coverage}/src/views.rs` | 3 port verbatim; `crate_dependencies` → verified JSON1 SQL |
| Queries (deps/loc/coverage) | `extensions/duckdb/src/sql/query/` | SQL unchanged (tables keep typed columns); `Row::get` types same |
| Erasure trait + `duckdb` feature flag | `crates/extension/src/db_handle.rs`, Cargo.tomls | Rename trait/feature |
| Consumers: `get_db`/`try_provide_from_db` | `extensions/about/src/{code,loc,units,identity}.rs`, tokei, extensions-rust/* | Import rename |
| ~52 `DuckDb::open*` test sites (mostly in-crate tests) | workspace-wide | Mechanical |
| Build wiring to delete | `scripts/{fetch-duckdb.sh,duckdb-pins.txt}`, `.github/workflows/{ci,release}.yml`, `AGENTS.md`, root `Cargo.toml`, `crates/core/src/.default.ops.toml:129` comment, `deny.toml:8` comment, `docs/duckdb-*.md` | Deletion |

Only `metadata_raw` holds a nested object; `tokei_files`, `rust_loc_files`, `coverage_files` are **pre-flattened to one-record-per-row arrays** by their collectors before staging (e.g. `flatten_tokei_records`, `extensions/tokei/src/lib.rs`) — so those three tables load with a trivial `json_each` over a bound parameter.

## Phases

Each phase ends with the workspace compiling: `cargo fmt && cargo clippy --workspace --all-targets --all-features -- -D warnings && cargo nextest run --workspace --all-features` (+ `ops test-doc`). Per AGENTS.md: read the `code-review-rust` skill before editing; run `ops verify` and `ops qa` after each phase (note: `ops verify` still needs the duckdb eval until Phase 3 lands — after Phase 3 it must work *without* it; `qa` needs Trivy on PATH).

### Phase 1 — Engine core swap (`extensions/duckdb` → `extensions/sqlite`)

`git mv extensions/duckdb extensions/sqlite` (preserves blame), rename package `ops-duckdb` → `ops-sqlite`, then port internals. This phase must land as one unit — `read_json_auto` has no SQLite equivalent, so the ingest builder, the four consumer view definitions, and the queries change together (consumers in Phase 2 compile against the new crate only after this).

1. **Workspace deps** (`Cargo.toml`): add `rusqlite = { version = "0.37", features = ["bundled"] }` (JSON1 is built into bundled SQLite ≥ 3.38; no extra feature). Leave the `duckdb` workspace dep in place until Phase 3 (the old crate is gone after the `git mv`, so remove it from `[workspace.dependencies]` now and fix the two inline references: root `Cargo.toml:59` chrono/arrow comment — chrono stays, rewrite the comment since arrow leaves the graph).
2. **`connection.rs`**: `DuckDb` → `Sqlite` wrapping `Mutex<rusqlite::Connection>`. `open`/`open_readonly`/`open_in_memory`/`lock`/`path`/`resolve_path` keep signatures. `resolve_path` default filename `data.duckdb` → `data.db` (`connection.rs:224`). Set `busy_timeout` on open (cheap insurance; single-process today).
3. **`error.rs`**: `DbError::query_failed(source: duckdb::Error)` → `rusqlite::Error`. Variant name `DuckDb` → `Sqlite`.
4. **`schema.rs`**: `data_sources` DDL → `TEXT`/`INTEGER` (`CURRENT_TIMESTAMP` exists in SQLite). `upsert_data_source` / `get_source_checksum`: `duckdb::params!` → `rusqlite::params!`.
5. **Ingest builder redesign** (`sql/ingest/sql.rs`) — the heart of the migration:
   - **Flat-array tables** (tokei_files, rust_loc_files, coverage_files): new builder `create_table_from_json_array_sql(table: TableName, columns: &[(TableName-col, &'static str json_path, SqlType)])` emitting `DROP TABLE IF EXISTS "t"; CREATE TABLE "t" (col INTEGER NOT NULL, …);` — then the ingestor executes `INSERT INTO "t" SELECT CAST(json_extract(value, '$.x') AS INTEGER), … FROM json_each(?1)` with the **staged file's bytes bound as a parameter** (read via the existing anchored `IngestDir::open_read`).
   - **Single-blob table** (metadata_raw): `metadata_raw` becomes one row (`json TEXT NOT NULL`); the whole `cargo metadata` payload is bound as a parameter.
   - Keep the SEC-12 gated-newtype discipline (`CreateTableSql`/`CreateViewSql` stay the only currency `load_with_sidecar` accepts).
   - **Security win to record in code comments**: binding bytes as a parameter deletes the SEC-25/TASK-2067 residual — the one un-anchored `read_json_auto('<path>')` staged read. `prepare_path_for_sql`, `escape_sql_string`, `ExtraOpts`, `validate_extra_opts` die with it.
   - `OPS_METADATA_MAX_BYTES` knob survives, applied Rust-side: check byte length of the payload before binding (replaces `maximum_object_size=` threading; `metadata_raw_create_sql_with_cap` and its test are reworked to pin the Rust-side cap).
   - `record_count` for `LoadResult`: `INSERT … SELECT` then `conn.changes()`.
6. **Views**:
   - `tokei_languages`, `rust_loc_summary`, `coverage_summary`: bodies port **verbatim** — the base tables now have real typed columns, so `GROUP BY`/`SUM`/`CASE` SQL is plain and identical. `CREATE OR REPLACE VIEW` → SQLite has no OR REPLACE: `CreateViewSql::create_or_replace` emits `DROP VIEW IF EXISTS …; CREATE VIEW …`.
   - `crate_dependencies` (`extensions-rust/metadata/src/views.rs`): replace `unnest`/struct-access body with the **verified JSON1 port from `docs/duckdb-alternatives.md`** (§ "SQLite (rusqlite, bundled)"), adapted: source is the single-row `metadata_raw`, so `FROM metadata_raw m, json_each(m.json, '$.packages')`. Booleans: `COALESCE(json_extract(dep,'$.optional'),0)` yields INTEGER 0/1 — no consumer reads `is_optional` as SQL bool (checked: `query_crate_deps` selects name/req only).
7. **Queries** (`sql/query/{deps,loc,coverage,helpers}.rs`): SQL text unchanged; `duckdb::` paths → `rusqlite::`. `query_rows_to_json` helper: rusqlite has no `stmt.query_map`-to-JSON sugar identical to duckdb's `Statement::query` + `Row::get`? It does have the same `query_map`/`query_row` API (fork parent) — keep the helper's shape.
8. **`dir.rs`, `sidecar.rs`, `orchestrator.rs`**: unchanged except `ops_duckdb`→`ops_sqlite` in doc comments and the `data.duckdb.ingest` test-dir names.
9. **`validation.rs`**: delete `ExtraOpts` + `validate_extra_opts` + path-for-SQL escaping; keep identifier/path/traversal validation (still used for table names and staging entries).
10. **`test_create_sql_validation!` macro + `test-helpers` feature**: rework assertions from `read_json_auto` string-shape checks to builder validation behavior (bad identifier rejected, columns validated).
11. **Constants**: `NAME = "sqlite"`, `DATA_PROVIDER_NAME = "sqlite"`, `DESCRIPTION` updated; `SHORTNAME = "db"` unchanged.

### Phase 2 — Flip consumers

1. `crates/extension`: feature `duckdb = []` → `sqlite = []` (`Cargo.toml:9`); trait `DuckDbHandle` → `SqliteHandle` (`src/db_handle.rs`) — keep the blanket-impl + reborrow contract docs (SEC-38) intact, renamed. Update `src/error.rs` and `tests/public_api.rs`.
2. `extensions/tokei`, `extensions/about`: `ops-duckdb` → `ops-sqlite` deps, `duckdb` feature → `sqlite`, `get_db`/`try_provide_from_db` call sites import-rename (`about/src/{code,loc,units,identity,lib}.rs`).
3. `extensions-rust/{metadata,loc,test-coverage}`: views.rs port (Phase 1 item 6 covers the SQL; here switch crates + adapt `metadata_raw_create_sql` to the new builder signature). `extensions-rust/about` and `extensions-rust/deps`: import renames only.
4. `crates/cli/Cargo.toml:14-15,24-25`: feature `duckdb` → `sqlite`; `stack-rust`, `tokei`, `coverage` features repointed. Grep `ops_duckdb` across the workspace to catch stragglers (expect the ~52 test `open*` sites — mechanical renames; in-memory test pattern `Sqlite::open_in_memory` is identical under rusqlite).

### Phase 3 — Delete the DuckDB machinery

- `git rm scripts/fetch-duckdb.sh scripts/duckdb-pins.txt`.
- `.github/workflows/ci.yml`: remove both "Fetch prebuilt libduckdb" steps + `DUCKDB_LIB_DIR` env (lines ~132-163); release.yml: remove the static fetch step + `DUCKDB_LIB_DIR`/`DUCKDB_STATIC` exports (lines ~183-188). Release binaries no longer need the static-link notes.
- `AGENTS.md`: delete the "DuckDB prebuilt library" section (the eval instruction is obsolete — linking builds now just work) and the two duckdb doc bullets; add one line pointing at `docs/duckdb-to-sqlite.md` (new, short: what changed, why the DB file is `data.db`, that `sqlite3 target/ops/data.db` works for ad-hoc queries).
- `docs/`: mark `duckdb-prebuilt-lib.md`, `duckdb-cli-backend.md`, `duckdb-alternatives.md` as superseded (short header note linking the new doc) or delete — recommend delete; git history keeps them, and `duckdb-alternatives.md`'s verified JSON1 SQL must be preserved by copying it into the new doc's appendix.
- `crates/core/src/.default.ops.toml:129`: comment example `.ops/data.duckdb` → `.ops/data.db`.
- `deny.toml:8`: fix the stale "via tera/duckdb/rust_decimal" comment; re-run `cargo deny check` to confirm no new advisories and that the removed arrow tree took its advisories with it.
- Root `Cargo.toml`: remove `duckdb = { version = "1.10502" }` if any reference survived; `Cargo.lock`: regenerate (expect ~140 crates gone).
- Old cache files (`target/ops/data.duckdb`, `target/duckdb-prebuilt/`) are disposable — note in the doc that they can be deleted; no code migration.

### Phase 4 — Verification

1. **Gates**: `ops verify` (now *without* any duckdb eval — this is the acceptance proof for the original pain), `ops qa`, `ops clippy-default`, `ops install`.
2. **End-to-end ingest + query**: `cargo run -- about` on a clean `target/ops/` (first run exercises fresh ingest of tokei/rust-loc/metadata/coverage sidecars; second run exercises the checksum-skip path). Compare `ops about` output before/after the migration on the same commit — numbers must match.
3. **Row-level equivalence**: the ported `crate_dependencies` view must reproduce the same row count/per-crate counts as before (the alternatives doc's benchmark: 354 rows). A one-off check: `sqlite3 target/ops/data.db 'SELECT COUNT(*) FROM crate_dependencies'` and per-crate counts vs. the pre-migration `data.duckdb` (query it before Phase 1, or from a checkout of the pre-migration commit).
4. **Ad-hoc queryability preserved**: `sqlite3 target/ops/data.db '.tables'` works (the doc's open question — SQLite keeps it, better than before).
5. **CI**: push and confirm the workflow passes without the fetch steps; release `dry-run` (per docs/releasing.md) confirms dist builds with no libduckdb and smaller binaries.

## Key risks

- **Big-bang core**: Phase 1 is indivisible (ingest builder + views + queries change together). Mitigated by: hardest SQL pre-verified in the doc; 3 of 4 views port verbatim; engine API is a fork of rusqlite's.
- **Type inference loss**: DuckDB inferred column types; SQLite needs explicit `CAST` in the array-table builders. Missing JSON paths now yield NULL columns instead of errors — the doc argues this is more drift-tolerant; the builders' column specs make the contract explicit.
- **Boolean shape**: `is_optional` becomes INTEGER 0/1 — no current consumer reads it typed; the view column comment should note it.
- **`CREATE OR REPLACE`**: doesn't exist in SQLite for tables/views — builders emit DROP+CREATE; fine because `load_with_sidecar` already owns the whole load transactionally and the DB is a disposable cache.

## Estimate

Phases 1+2 are the work (~3.2k-line crate port, heavily mechanical, plus ~10 consumer files and ~52 test sites). Phase 3 is deletion. Days, not weeks — consistent with the alternatives doc's "mechanical" assessment.
