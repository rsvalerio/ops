# DuckDB → SQLite migration notes

The workspace's embedded analytics engine moved from the prebuilt
libduckdb link (`scripts/fetch-duckdb.sh`, `ops-duckdb`) to
**rusqlite with bundled SQLite** (`extensions/sqlite`, crate `ops-sqlite`).
`docs/duckdb-alternatives.md` made the case; this doc records what changed
for day-to-day work.

## What changed

- **No link env.** Every linking build (`cargo build`, `cargo test`,
  `cargo nextest run`, `ops verify qa install`) now works in a plain shell —
  the `eval "$(scripts/fetch-duckdb.sh)"` step is gone, along with the
  fetch script, the pins file, the CI/release fetch steps, and ~140
  dependency crates (the arrow tree among them).
- **The database file is `.ops/data.db`** (was `.ops/data.duckdb`). It is a
  disposable cache, same as before: delete it and the next `ops about`
  re-ingests from the staged sidecars. Old `data.duckdb` files and the
  `target/duckdb-prebuilt/` cache can be deleted.
- **Ad-hoc queries**: `sqlite3 target/ops/data.db` (or the `.ops/data.db`
  next to the project being inspected) — `.tables`, plain SELECTs, JSON1
  functions all work with the stock `sqlite3` CLI. This is an improvement:
  the DuckDB file needed a matching `duckdb` CLI.
- **Ingest is parameter-bound.** Staged JSON reaches the engine as a bound
  `?1` parameter (`json_each(?1)` / `json(?1)`), never as an interpolated
  `read_json_auto('<path>')` — the former SEC-25/TASK-2067 residual is
  closed. Table shapes are declared as `const` `JsonTableLoad` specs
  (`flat_array` for the pre-flattened collectors, `single_object_blob` for
  `metadata_raw`).
- **Booleans are INTEGER 0/1** (`is_optional` in `crate_dependencies`).
  SQLite has no boolean type; no consumer reads the column typed.
- **No `CREATE OR REPLACE`** — builders emit `DROP … IF EXISTS; CREATE …`
  batches, executed via `execute_batch`.

## Verified JSON1 port of `crate_dependencies` (appendix)

From `docs/duckdb-alternatives.md` (deleted; this is the verified artifact
it contributed): a direct port of the `unnest` body to JSON1, checked
against this workspace's real `cargo metadata` — 354 rows, matching
per-crate counts, and the `cfg(unix)` target rows that PATTERN-1/TASK-1056
exists to preserve came through intact.

```sql
WITH pkgs AS (SELECT p.value AS pkg FROM metadata_raw m, json_each(m.json,'$.packages') p),
ws AS (SELECT w.value AS member_id FROM metadata_raw m, json_each(m.json,'$.workspace_members') w),
member_deps AS (
  SELECT json_extract(pkg,'$.name') AS crate_name,
         json_extract(pkg,'$.manifest_path') AS crate_manifest_path,
         d.value AS dep
  FROM pkgs, json_each(pkgs.pkg,'$.dependencies') d
  WHERE json_extract(pkg,'$.id') IN (SELECT member_id FROM ws)
)
SELECT crate_name,
       json_extract(dep,'$.name') AS dependency_name,
       json_extract(dep,'$.req')  AS version_req,
       COALESCE(json_extract(dep,'$.kind'),'normal') AS dependency_kind,
       COALESCE(json_extract(dep,'$.optional'),0)    AS is_optional,
       NULLIF(json_extract(dep,'$.target'),'')       AS target,
       crate_manifest_path
FROM member_deps
ORDER BY crate_name, dependency_kind, dependency_name, target
```
