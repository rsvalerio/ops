---
id: TASK-1873
title: >-
  READ-10: blanket reason-free #[allow(dead_code)] on whole enums and impl
  blocks, plus #[allow(unused_imports)] on every lib.rs pub use
status: To Do
assignee:
  - TASK-2006
created_date: '2026-08-27 15:31'
updated_date: '2026-08-28 14:16'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/duckdb/src/lib.rs
  - extensions/duckdb/src/error.rs
  - extensions/duckdb/src/connection.rs
  - extensions/duckdb/src/ingestor.rs
  - extensions/duckdb/src/schema.rs
  - extensions/duckdb/src/sql/ingest/dir.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/lib.rs:22-28, 71-73`, `extensions/duckdb/src/error.rs:7`, `extensions/duckdb/src/connection.rs:66`, `extensions/duckdb/src/ingestor.rs:41, 54`, `extensions/duckdb/src/schema.rs:39`, `extensions/duckdb/src/sql/ingest/dir.rs:65`

**What**: Eleven bare suppressions, none carrying a reason, most applied at the widest possible scope:

- `error.rs:7` — `#[allow(dead_code)]` on the entire `DbError` enum (11 variants).
- `connection.rs:66` — `#[allow(dead_code)]` on the entire `impl DuckDb` block, i.e. every constructor and method including `open_readonly` and `id`.
- `ingestor.rs:41, 54` — the whole `SidecarIngestorConfig` struct and its whole impl block.
- `lib.rs:22-28` — `#[allow(unused_imports)]` repeated on four `pub use` statements. A `pub use` in a library crate is a re-export; it is never "unused", so these suppress nothing and read as cargo-culted noise.
- `dir.rs:65` — on `default_data_dir`, which really is dead: `grep -rn --include='*.rs' default_data_dir .` finds only the definition and two re-export lines, no caller anywhere in the workspace.

Per AGENTS.md ("grant the exception at the narrowest scope that works and write the reason next to it — see docs/clippy.md") and READ-10 (prefer `#[expect(lint, reason = "…")]` so the suppression deletes itself once the problem is gone), every one of these is the wrong shape.

**Why it matters**: An item-level allow on a container hides *future* dead code too — add a `DbError` variant nobody constructs, or a `DuckDb` method nobody calls, and the compiler stays silent forever. That is how `default_data_dir` survived: the allow made its deadness invisible. The suppressions also give a reviewer no way to tell an intentional policy exception from leftover scaffolding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The four #[allow(unused_imports)] attributes on lib.rs pub use statements are removed and the crate still builds warning-free
- [ ] #2 Container-level #[allow(dead_code)] on DbError, impl DuckDb and SidecarIngestorConfig is removed or narrowed to the specific items that need it, each with a reason (prefer #[expect(..., reason = "...")])
- [ ] #3 default_data_dir is either deleted along with its re-exports or gains a caller; no allow is left standing in its place
- [ ] #4 cargo clippy --all-targets --workspace -- -D warnings is clean afterwards
<!-- AC:END -->
