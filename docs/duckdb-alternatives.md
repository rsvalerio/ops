# DuckDB contenders: what could replace it, and at what cost

Companion to [`duckdb-prebuilt-lib.md`](duckdb-prebuilt-lib.md) and
[`duckdb-cli-backend.md`](duckdb-cli-backend.md). Those two keep DuckDB and
attack the build cost. This one asks whether something else should hold the job.

## What the job actually is

Verified from the ingest and view layer:

- Ingest arbitrary JSON emitted by external tools (`cargo metadata`, `tokei`,
  `llvm-cov`, rust-loc) with **no Rust-side data model** — the only
  `serde_json::from_*` calls in the path produce an untyped `Value`.
- Navigate nested arrays/objects: `crate_dependencies` unnests `packages`, then
  `pkg.dependencies`, filtered against `workspace_members`.
- Aggregate: `GROUP BY` + `SUM` + `CASE` (tokei by language, rust-loc by region,
  coverage summary, per-crate counts).
- Persist to a file under `target/ops/` that is a **disposable cache** — every
  byte is re-derivable.

Scale: this workspace's `crate_dependencies` is **354 rows** from a 100 KB
`metadata.json`. Nothing here is a performance problem.

## What DuckDB costs

| | |
|---|---|
| `target/release/.../libduckdb.a` | 118 MB (2.03 GB in debug) |
| C++ amalgamation | recompiled on every cold build; **sccache does not cache it** |
| Rust crates pulled in | **140 of the workspace's 259** (54%), including 11 arrow crates |

The dependency-graph half is easy to miss. Dropping DuckDB removes arrow too,
and *those* crates do cache under sccache — so the win is smaller than the C++
win but real.

## Contenders

### SQLite (rusqlite, bundled) — the strongest

**Verified, not assumed.** A direct port of `crate_dependencies_view_sql` to
JSON1 reproduces the view against this workspace's real `cargo metadata`:

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

354 rows, matching per-crate counts, and the `cfg(unix)` target rows that
PATTERN-1/TASK-1056 exists to preserve come through intact.

**Build cost**: a cold `cargo build --release` of a crate depending on
`rusqlite = { features = ["bundled", "serde_json"] }` — the whole thing, deps
included — took **1m 12s**.

**Migration is mechanical.** `duckdb-rs` is a fork of `rusqlite`; every API the
tree uses exists under the same name: `params!`, `params_from_iter`,
`execute_batch`, `prepare`, `query_map`, `query_row`, `Row::get`,
`Error::QueryReturnedNoRows`. The ~50 engine call sites are near drop-in. The
work is in the SQL: 4 view definitions, the ingest SQL
(`read_json_auto(path)` → insert the blob + `json_each` views), and the query
modules.

**What you give up**: inferred column types (write `CAST(json_extract(...) AS
INTEGER)` where you aggregate), columnar execution (irrelevant at 354 rows), and
DuckDB SQL sugar the tree does not use. Explicit `json_extract` paths are
arguably *more* drift-tolerant than inference — a missing path yields NULL
rather than a missing column.

### DataFusion — pure Rust, wrong JSON reader

Attractive on paper: no C++, so it caches under the existing sccache setup, and
it has `unnest` plus struct field access. But its JSON reader is
**line-delimited only**, and `cargo metadata` emits a single JSON object — you
would need a conversion step in front of every ingest. It also brings its own
large arrow-based dependency tree and has no persistence layer. Not less pain.

### Polars — no

Heavy compile, partial SQL surface, no views, no persisted database. Solves a
problem this project does not have.

### chDB / clickhouse-local — no

A larger C++ build than DuckDB. Strictly worse on the axis that matters.

### Plain Rust over `serde_json` — the maximum-simplification endpoint

Worth naming because it is the only option that deletes code rather than
swapping engines. The four views are `GROUP BY`/`SUM` folds; `crate_dependencies`
is a nested iteration. At this data volume that is maybe 150 lines of iterator
code, and `serde_json` is already a dependency.

It would delete the entire SQL-string apparatus: `sql/validation.rs` (768 lines),
the `TableName`/`ColumnAlias`/`ColumnName` newtypes, `escape_sql_string`,
`prepare_path_for_sql`, `ExtraOpts` — all of which exist *only* because SQL is
built by interpolation. Every SEC-12-class task in the backlog is a tax on that
choice.

The open question is whether ad-hoc queryability of `target/ops/data.duckdb` is
a feature anyone actually uses. If yes, this is off the table. If it has never
been opened by hand, it is the biggest simplification available.

## Ranking

| option | build win | code churn | risk |
|---|---|---|---|
| Prebuilt libduckdb | C++ only | none | static link (23 archives) |
| **SQLite / rusqlite** | **C++ + 140 crates** | **SQL rewrite, engine calls near drop-in** | **low — verified** |
| Plain Rust | C++ + 140 crates | query layer rewrite, big deletion | loses ad-hoc SQL |
| DuckDB CLI backend | C++ + 140 crates | Phase 1 refactor | subprocess, version skew |
| DataFusion / Polars / chDB | mixed | large | poor fit |

## Recommendation

Two live options, and the choice is about appetite:

- **Least risk, least reward**: [prebuilt libduckdb](duckdb-prebuilt-lib.md).
  Zero code change, kills the C++ compile, keeps all 140 crates. Its Phase 0
  answers the only hard question in a few hours.
- **Best end state**: SQLite. Kills the C++ compile *and* 54% of the dependency
  graph, and the verification above shows the hard part already works. Bigger
  diff, but a mechanical one.

The DuckDB CLI backend is now the weakest of the three — it carries the most
risk (subprocess overhead, storage-format skew, 103 in-memory test sites) for a
win SQLite delivers more cleanly. Treat it as the fallback if both of the above
fail.

Suggested order: run the prebuilt-lib Phase 0 first since it is cheap and
unblocks immediate relief either way. Then decide whether the SQLite migration
is worth the diff — the answer likely turns on how much the DuckDB-specific SQL
is expected to grow.
