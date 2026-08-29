---
id: TASK-1805
title: >-
  ERR-1: from_env lossy-renders a non-UTF-8 TMPDIR through Path::display(),
  contradicting its own documented Errors contract
status: Done
assignee:
  - TASK-1984
created_date: '2026-08-27 11:29'
updated_date: '2026-08-29 00:36'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - crates/core/src/expand.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:400-415`

**What**: `Variables::from_env` documents under `# Errors`:

> [`ExpandError::NotUnicode`] if `ops_root` **or `TMPDIR`** is not valid Unicode. ERR-1 / TASK-1462: surfaced rather than lossily rendered so a corrupt root cannot flow into a spawned subprocess.

The `OPS_ROOT` half honours that: `cached_ops_root_arc` calls `key.to_str().ok_or_else(...)` and returns an `ExpandError`. The `TMPDIR` half does the exact opposite:

```rust
let tmpdir = TMPDIR_DISPLAY
    .get_or_init(|| Arc::<str>::from(std::env::temp_dir().display().to_string()))
    .clone();
```

`Path::display()` is *defined* to substitute U+FFFD for non-UTF-8 bytes. A `TMPDIR` that is not valid UTF-8 therefore silently becomes a corrupted `Arc<str>` that `try_expand` — the strict path whose whole purpose (TASK-0450) is to refuse exactly this — hands straight into subprocess argv, cwd, and env values. `try_expand` cannot catch it because the corruption already happened at builtin-construction time, before `shellexpand` ever sees the value.

Two secondary defects in the same area:

1. `ExpandError` is a **struct** (`{ var_name, cause }`), not an enum, so the `ExpandError::NotUnicode` path named in the doc comment does not exist. The rustdoc intra-doc link at line 400 is broken, and the same phantom variant is cited at lines 171, 379, 406, and 830.
2. Because `TMPDIR_DISPLAY` is a process-lifetime `OnceLock`, the corrupted value is cached for the rest of the process — the first caller poisons it for everyone.

**Why it matters**: This is the identical defect class that TASK-1462 closed for `OPS_ROOT`, left open on the sibling builtin. `ops` runs in user-controlled environments; a `TMPDIR` carrying non-UTF-8 bytes (a legal Unix path) makes `$TMPDIR/...` expand to a path containing replacement characters, which then materialises on disk or is passed to a spawned command as a *different* path than the operator configured. The documented contract says this is impossible, so callers have no reason to defend against it.

<!-- scan confidence: verified by reading; the `# Errors` doc at expand.rs:398-402 and the `display()` call at expand.rs:411-413 are directly contradictory -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Variables::from_env returns an ExpandError (rather than a U+FFFD-substituted value) when std::env::temp_dir() is not valid UTF-8, matching the OPS_ROOT branch
- [x] #2 The TMPDIR OnceLock caches only a successfully-validated UTF-8 rendering, so one failed lookup cannot poison later callers with a corrupt value
- [x] #3 All five doc references to the non-existent ExpandError::NotUnicode variant are corrected to name the real type, and cargo doc emits no broken intra-doc-link warning for expand.rs
- [x] #4 A regression test asserts that a non-UTF-8 TMPDIR (OsString from bytes on Unix) causes from_env to return Err rather than a lossy Arc<str>
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-1984. `from_env` now routes TMPDIR through `cached_tmpdir_arc` / `tmpdir_arc_from`, which returns an ExpandError carrying VarError::NotUnicode instead of lossy-rendering via Path::display(); only a validated UTF-8 rendering ever reaches the TMPDIR_DISPLAY OnceLock, so a refused lookup cannot poison later callers. All five phantom `ExpandError::NotUnicode` doc references (expand.rs 171/379/400/406 and the runner rationale) now name the real type; `cargo doc -p ops-core` emits no unresolved-link warning for expand.rs (the one remaining warning there is the pre-existing private-item link to TMPDIR_DISPLAY). Tests: from_env_rejects_non_utf8_tmpdir, non_utf8_tmpdir_does_not_poison_the_cache, driven by a thread-local TMPDIR override seam so a synthetic non-UTF-8 path cannot leak into parallel tests.
<!-- SECTION:NOTES:END -->
