---
id: TASK-1836
title: >-
  CL-3: the last fixed column is clamped to the `====` separator's width,
  silently truncating any value wider than its header token
status: To Do
assignee:
  - TASK-1997
created_date: '2026-08-27 15:22'
updated_date: '2026-08-28 14:13'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/deps/src/parse/upgrade.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/upgrade.rs:302-324` (`separator_columns`), `:210-221` (`slice_column`), `:246-266` (`parse_upgrade_row`)

**What**: `separator_columns` derives each column's `end` from the *next* column's `start`, and gives the **last** column `end = line.len()` — where `line` is the `====` separator row, not the data row:

```rust
let len = line.len();                      // length of the SEPARATOR row
cols.iter()
    .zip(cols.iter().skip(1).map(|c| c.0).chain(std::iter::once(len)))
    .map(|(&(start, _), end)| (start, end))
    .collect()
```

This rests on an undocumented invariant — *"the `=` run in the separator row is at least as wide as every value in that column"* — which cargo-edit does **not** hold to. The crate's own fixtures prove it: in `table_tests.rs::parse_upgrade_table_basic` the `latest` header is 6 chars with a 6-wide `======` separator while its values are 7 chars (`1.0.228`), and in `parse_upgrade_table_with_notes` the `note` separator is `====` (4) while the value is `incompatible` (12). cargo-edit sizes the `=` run to the **header token length**, not the column content width.

For every *interior* column that is harmless: `end` chains forward to the next column's `start`, so an over-wide value still fits. The **final** fixed column has nothing to chain to, so it is clamped to the separator row's total length and the overflow is dropped by `slice_column`'s `line.get(start..end)?.trim()`.

Concretely, with the standard 5-column (no `note`) table cargo-edit emits:

```
name   old req compatible latest  new req
====   ======= ========== ======  =======
serde  1.0.100 1.0.228    1.0.228 1.10.100
```

the separator row is 41 bytes, column 4 is `(34, 41)`, and `new_req` decodes as **`1.10.10`** instead of `1.10.100`. Verified by replaying `separator_columns`' exact algorithm against that input. Any `new req` longer than the 7-char `new req` header token truncates: `1.10.100`, `1.0.228-beta.1`, `>=1.2, <2.0`, a 4-digit patch, a prerelease or build-metadata suffix.

Note the truncation is *silent by construction*: the row still fills all five columns, so `parse_upgrade_row` returns `Some`, `entries_emitted` increments, and neither `check_header_drift` nor `check_row_shape_drift` (nor the missing-separator guard tracked in TASK-1817) sees anything wrong. `clamp_to_char_boundaries` also stays quiet because `end.min(len)` never trips the `e != clamped_end` warn on an ASCII line. Nothing is logged at any level.

The `note` column escapes this only because `slice_note` (`:233-244`) deliberately reads `start..line.len()` of the **data** line. The fix is the same trick for the final fixed column — the last column should extend to the end of the *data* row, not the separator row.

**Why it matters**: `ops deps` exists to tell an operator which version to upgrade to. This silently prints a version that does not exist (`1.10.10` for a crate published at `1.10.100`), and the same truncated string is persisted into the cached `DepsReport` JSON that `ops about deps` and any downstream consumer read. It is worse than a parse failure: a dropped row is visible as a missing entry, whereas a truncated version looks like a perfectly ordinary answer. The hardening already done on this parser (TASK-0913, TASK-1074, TASK-1202, TASK-1026) all guards against *rows disappearing*; none of it guards against a row that is present and wrong.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 separator_columns (or its callers) no longer bound the final column by the separator row's length; the last fixed column reads to the end of the data row like slice_note already does
- [ ] #2 A test drives parse_upgrade_table over a 5-column table whose new req value is wider than the 'new req' header token (e.g. 1.10.100) and asserts new_req round-trips in full
- [ ] #3 A test covers the same case with a 6-column table (note present) to confirm the note column still captures multi-word text and the new req column is not widened into the note
- [ ] #4 Existing table_tests fixtures (parse_upgrade_table_basic, _with_notes, _multi_word_note, _non_ascii_row_does_not_panic) still pass unchanged
- [ ] #5 The invariant the column geometry relies on is stated in a comment on separator_columns so the next reader does not have to rediscover that the = run is header-token-width, not column-width
<!-- AC:END -->
