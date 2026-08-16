---
id: TASK-1540
title: >-
  DUP-4: cargo-metadata JSON fixture re-inlined 12+ times across
  metadata/src/tests.rs and ingestor.rs tests
status: Done
assignee:
  - TASK-1576
created_date: '2026-05-19 15:24'
updated_date: '2026-08-16 19:20'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests.rs:10-78` (plus ~12 re-inlined copies), `extensions-rust/metadata/src/ingestor.rs:208-256, 294-353, 394-430, 478-537`

**What**: The cargo-metadata JSON skeleton (`packages`, `workspace_members`, `workspace_default_members`, `resolve`, `target_directory`, `version`, `workspace_root`, `metadata`) is open-coded as a `serde_json::json!` literal in many test bodies:
- `tests.rs:10-78` is `sample_metadata()` — the proper shared fixture.
- `tests.rs:186-201, 213-228, 236-251, 266-271, 490-496, 501-505, 513-528, 537-552, 562-577, 590-609, 636-653, 669-686, 702-719, 727-744, 752-790, 805-823` re-inline near-identical packaging JSON instead of building on the shared fixture.
- `ingestor.rs:208-256, 294-353, 394-430 (via sample_obj), 478-537` inline another ~30-field JSON each.

Each block restates the same 15-20 boilerplate fields (`source`, `features`, `manifest_path`, `metadata`, `publish`, `authors`, `categories`, `keywords`, `readme`, `repository`, `homepage`, `documentation`, `edition`, `links`, `default_run`, `rust_version`, `license`, `license_file`, `description`) that the test under question does not exercise.

**Why it matters**: A schema-shape change in cargo metadata (or in this crate's parsing assumptions) requires updating ~15 inline JSON blobs across two files. Reviewers cannot tell at a glance which fields each test actually cares about — the variation is buried in a wall of constant boilerplate. A `fn sample_pkg(name, version, id) -> Value` + `fn sample_metadata_with_pkgs(pkgs) -> Value` test helper (or a builder) would shrink each test body to its semantically-meaningful overrides and make regression coverage scope visible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 metadata/src/tests.rs and ingestor.rs share a single test-helper module (e.g. sample_pkg / sample_metadata_with) that constructs cargo-metadata JSON fixtures
- [x] #2 Each individual test body retains only the fields it specifically exercises; boilerplate fields (license, repository, homepage, etc.) live in the helper
- [x] #3 All existing tests still pass without weakening their assertions
<!-- AC:END -->

---

**Status note (2026-05-19, wave-122):** Left In Progress. The ingestor.rs
tests and the split-out `metadata/src/tests/edge_cases.rs` still re-inline
the ~30-field cargo-metadata JSON skeleton across ~12 call sites. A
follow-up wave should land:

1. a `test_fixtures` module exposing `sample_pkg(name, version, id)` and
   `sample_metadata_with_pkgs(pkgs)` helpers (shared between ingestor and
   tests/* submodules), and
2. a mechanical refactor of each inline `serde_json::json!({...})` call
   site so the boilerplate disappears.

Wave-122 deliberately did not pick this up because the rest of the wave
(14 of 15 member tasks) touched the same files and a partial fixture
refactor would have produced a confusing diff. None of the wave-122
changes regressed the duplication — it remains exactly as it was before.

## Triage Notes

<!-- SECTION:TRIAGE:BEGIN -->
Reviewed in the 2026-08-15 sweep — **status is accurate, left as-is**, but
the remaining scope has narrowed and is recorded here.

Real progress has landed since the report: `metadata/src/tests.rs` was split
into a `tests/` module, and `tests/fixtures.rs` now provides the shared
`sample_metadata()` helper AC #1 asks for. `tests.rs` itself has zero `json!`
literals.

Remaining: 23 `json!` literals still open-code fixture JSON —
`tests/edge_cases.rs` (15), `tests/duplicates.rs` (2), `tests/accessors.rs`
(2), and `ingestor.rs` (4). AC #2 (each test body retains only the fields it
exercises) is therefore not yet met. `edge_cases.rs` is the bulk of it and may
legitimately need bespoke JSON — worth checking before mechanically folding
those into the helper.
<!-- SECTION:TRIAGE:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
## Done — one fixture module, two deliberately different families

```
                          before   after
src/ingestor.rs              632     413
src/tests/edge_cases.rs      397     262
src/tests/accessors.rs       354     343
src/tests/duplicates.rs      167     144
src/tests/fixtures.rs         84       - (moved)
src/test_support.rs            -     409  new
```

`json!` fixture literals: **23 -> 11**, and the 11 survivors are payloads, not
skeletons — a `deps(json!([...]))` argument *is* the subject of its test.

### The finding the task did not anticipate

The triage note flagged that `edge_cases.rs` "may legitimately need bespoke
JSON — worth checking before mechanically folding those into the helper." It
does, and so does `ingestor.rs`, for opposite reasons. A single
`sample_pkg()` returning a fully-populated package would have broken both:

- **View fixtures** (`pkg`, `workspace`) feed `Metadata::from_value`, which
  reads lazily. Most of `edge_cases.rs` asserts on *absent* fields —
  `edition()` falling back to `""`, `license()` returning `None`. A helper
  filling in defaults would leave those tests green while testing nothing.
  So the builder is minimal by construction: it emits only what was set.
- **Ingest fixtures** (`ingest_metadata`, `ingest_dep`) are written to disk
  and read back through DuckDB's `read_json_auto`. Their ~20 empty-string
  fields are load-bearing: a column that is null in every row infers as
  INTEGER and the view's casts then fail. AC #2 read literally ("retain only
  the fields it exercises") would break schema inference here.

Both families live in one module (AC #1) with the contrast documented at the
top — that contrast is the part a future reader needs and neither call site
was stating.

### Design points

- `id` and `manifest_path` derive from name+version, so the package entry and
  `workspace_members` cannot drift apart. `.id(...)` overrides it where the id
  is the subject (duplicate-id, registry packages).
- `.member()` vs `.external()` replaces hand-maintained `workspace_members`
  arrays; `.default_members(&["pkg-a"])` takes names and panics on one that
  was never added, so the list cannot silently point at nothing.
- `write_metadata_json(dir, &value)` folds the write-then-load preamble the
  four ingestor tests each open-coded.

### Left inline on purpose (2 sites)

- `metadata_missing_packages_key` — the absence of the `packages` key is the
  subject; the builder always emits it.
- `metadata_root_package_finds_match_with_backslash_separator` (`#[cfg(windows)]`)
  — needs backslash-separated paths, and a Windows-separator setter would be
  dead code on every other platform, which does not compile clean under
  `-D warnings`.

Both carry a comment saying why, so the next reader does not "finish the job".

### Verification

`cargo test -p ops-metadata`: **85 before, 85 after**, inventory diffed by
name via `--list --include-ignored` — identical, empty in both directions.
No assertion was weakened: `metadata_load_with_sample_data` keeps its
`assert!(!json_path.exists())` cleanup check, which is why
`write_metadata_json` returns the path.

Dropped `#[allow(clippy::too_many_lines)]` from
`crate_dependencies_view_preserves_target_conditional_duplicates` — the test
is 20 lines now and no longer needs the exemption.

### Note for review

`test_support.rs` is 409 lines, just over the ~400 ARCH-1 guideline. Kept as
one module deliberately: the two families are documented together because the
contrast between them is the knowledge, and splitting would either duplicate
that explanation or orphan it.
<!-- SECTION:NOTES:END -->
