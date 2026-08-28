---
id: TASK-1897
title: >-
  SEC-11: OPS_METADATA_MAX_BYTES is accepted unvalidated and unbounded, then
  interpolated into DuckDB's UINTEGER maximum_object_size
status: To Do
assignee:
  - TASK-1999
created_date: '2026-08-27 15:36'
updated_date: '2026-08-28 14:13'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/metadata/src/lib.rs
  - extensions-rust/metadata/src/views.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:66-76` and `extensions-rust/metadata/src/views.rs:68-72`

**What**: The env knob is parsed with no range validation and no feedback, then flows straight into a DuckDB option with a narrower domain.

```rust
std::env::var(METADATA_MAX_BYTES_ENV).ok()
    .and_then(|v| v.parse::<u64>().ok())   // "64MB", "-1", "abc" -> None, silently
    .filter(|v| *v > 0)                    // 0 -> silently dropped
    .unwrap_or(METADATA_MAX_BYTES_DEFAULT)
```

```rust
let opts = format!("maximum_object_size={cap}");   // views.rs:69, any u64
```

Three distinct problems, in severity order:

1. **Silent fallback.** `OPS_METADATA_MAX_BYTES=64MB`, `=0`, `=-1`, or a value with trailing whitespace all resolve to the 64 MiB default with no warning anywhere. The operator raised the cap to work around an over-cap failure, sees the identical failure, and has no signal that the knob was ignored. The value is snapshotted in a `OnceLock`, so there is exactly one moment where a diagnostic could be emitted and it is not used.

2. **No upper bound.** Any non-zero `u64` is accepted, including `u64::MAX`. The cap exists (SEC-33 / TASK-1194) to stop a pathological workspace from OOM-ing `ops about`; a knob that can be set to "unbounded" with no ceiling and no warning silently disables the guard. `i64::try_from(cap).unwrap_or(i64::MAX)` at lib.rs:280 then quietly reinterprets anything above `i64::MAX` as `i64::MAX` — a second silent coercion.

3. **Domain mismatch with DuckDB.** DuckDB documents `read_json`'s `maximum_object_size` as `UINTEGER` (32-bit). A cap above `u32::MAX` therefore does not raise the ingest ceiling — it makes the `CREATE TABLE ... read_json_auto(...)` statement fail with an opaque DuckDB option-conversion error, attributed by `build_views` to `"metadata_raw create"` with nothing pointing back at the env var the operator set. ARCH-9 / TASK-1247 deliberately unified the reader cap and the ingest ceiling on one knob; the unification is only sound over the range both sides accept.

**Why it matters**: SEC-11 (validate range and format at the boundary). This is the one operator-facing tuning surface on a resource guard, and every wrong value it can be given is either silently ignored or converted into an error that does not name it. The layered-validation answer is small: parse, bound to `[1, u32::MAX]` (or whatever both consumers accept), and `tracing::warn!` naming the variable and the value whenever the supplied value is rejected or clamped.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A non-numeric, zero, or out-of-range OPS_METADATA_MAX_BYTES emits a tracing::warn! naming the variable and the offending value before falling back
- [ ] #2 The resolved cap is bounded to a range both consumers accept, so no env value can make the CREATE TABLE statement fail on an option-conversion error
- [ ] #3 The documented upper bound (and DuckDB's maximum_object_size type) is verified against the DuckDB version pinned in scripts/duckdb-pins.txt and recorded in the doc comment
- [ ] #4 Tests cover a malformed value, a zero value, and an above-ceiling value via metadata_raw_create_sql_with_cap / an injectable parse helper (not by mutating process-global env)
<!-- AC:END -->
