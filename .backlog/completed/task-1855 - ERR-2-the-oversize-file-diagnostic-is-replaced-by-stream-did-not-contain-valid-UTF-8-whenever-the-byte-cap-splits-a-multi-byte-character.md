---
id: TASK-1855
title: >-
  ERR-2: the oversize-file diagnostic is replaced by 'stream did not contain
  valid UTF-8' whenever the byte cap splits a multi-byte character
status: Done
assignee:
  - TASK-1983
created_date: '2026-08-27 15:28'
updated_date: '2026-08-28 23:53'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - crates/core/src/text.rs
  - crates/core/src/config/loader/mod.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:214-231` (`read_capped_to_string_with`), same defect at `crates/core/src/config/loader/mod.rs:116-133` (`read_capped_toml_file_with`)

**What**: The documented contract is a specific, actionable message:

```rust
/// On a file larger than the cap, returns `Err` with `ErrorKind::InvalidData`
/// (and a message naming the cap) without holding the full content in memory:
/// the read is bounded by `Read::take(cap + 1)`.
```

The implementation validates UTF-8 **before** it checks the size:

```rust
    let limit = cap.saturating_add(1);
    (&mut file)
        .take(limit)
        .read_to_string(&mut buf)
        .map_err(|e| with_path(&e, path))?;

    if u64::try_from(buf.len()).unwrap_or(u64::MAX) > cap {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "file exceeds {cap}-byte cap at {} (override via {MANIFEST_MAX_BYTES_ENV})",
```

`read_to_string` decodes the truncated `cap + 1` byte window and fails if it is not valid UTF-8. When byte `cap` lands inside a multi-byte sequence — which any oversized file containing non-ASCII will do with probability proportional to how much non-ASCII it has — `read_to_string` bails and the `?` propagates, so the cap branch below is **dead for that input**.

Reproduced with a standalone run of the exact code path (`cap = 4`, file = `"aaa€"`): `ERR kind=InvalidData msg=stream did not contain valid UTF-8`.

**Why it matters**: `ErrorKind::InvalidData` is preserved either way, so classification-by-kind still works — but the operator-facing message does not. An oversized `Cargo.toml` / `package.json` with any non-ASCII near the boundary reports *"manifest is corrupt"* instead of *"manifest is too big, raise `OPS_MANIFEST_MAX_BYTES`"*, and `for_each_trimmed_line_with`'s warn log repeats the misleading text. The `.ops.toml` copy in the loader is more visible still: the user sees

```
failed to read config file: "…": stream did not contain valid UTF-8
```

with no mention of `OPS_TOML_MAX_BYTES` and no hint that the file is simply large. The whole point of naming the override env var in the message is to make the cap self-service; the branch that does so is skipped exactly when the file is big enough to need it.

Fix: read bytes first (`take(limit).read_to_end`), apply the size check against the byte count, and only then `String::from_utf8`. That orders the two checks correctly and costs nothing.

Neither file has a test at the multi-byte cap boundary — the existing cases (`read_capped_to_string_oversize_returns_invalid_data`, `..._at_cap_returns_content`) fill with ASCII `b'a'` and therefore cannot trip it.

<!-- scan confidence: verified by reading text.rs:214-231 and config/loader/mod.rs:116-133, and by running a standalone repro of the same read path -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The size check runs before UTF-8 validation in both read_capped_to_string_with and read_capped_toml_file_with, so an oversized file always reports the cap message and the override env var
- [x] #2 A file that is genuinely invalid UTF-8 and under the cap still reports the UTF-8 error, unchanged
- [x] #3 Both files gain a regression test whose content places a multi-byte character across the cap boundary and asserts the error names the cap and the override env var
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Both readers now `take(cap + 1).read_to_end` into a `Vec<u8>`, check the byte count against the cap, and only then `String::from_utf8` — so an oversized file always reports the cap and the override env var, and an under-cap file that is genuinely invalid UTF-8 still errors with `InvalidData` (`text.rs`) / the same read context (`loader/mod.rs`). Four new tests, two per file: `..._oversize_multibyte_boundary_reports_cap` (cap 4, content `"aaa€"` so the window ends mid-sequence) and `..._under_cap_invalid_utf8_still_errors`.
<!-- SECTION:NOTES:END -->
