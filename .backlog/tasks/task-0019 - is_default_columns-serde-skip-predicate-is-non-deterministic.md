---
id: TASK-0019
title: is_default_columns serde skip predicate is non-deterministic
status: Done
assignee: []
created_date: '2026-04-10 23:30:00'
updated_date: '2026-04-11 09:57'
labels:
  - rust-code-quality
  - CQ
  - CL-3
  - READ-5
  - low
  - crate-core
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/config/mod.rs:175-183`
**Anchor**: `fn default_columns`, `fn is_default_columns`
**Impact**: `default_columns()` reads the current terminal width via `terminal_size::terminal_size()` and computes `width * 9 / 10`. The `is_default_columns` predicate, used by `#[serde(skip_serializing_if = "is_default_columns")]` on `OutputConfig.columns`, compares the stored value against this runtime-dependent default. This means serialization behavior depends on the terminal width at serialization time — a config written at 120 columns wide will serialize differently than one written at 80 columns wide, even if the `columns` value hasn't changed.

**Notes**:
CL-3: "Avoid implicit assumptions — make preconditions explicit." READ-5: "Make invariants explicit."

Concrete scenario: User runs `ops init` in a 120-column terminal. `default_columns()` returns 108. The `columns` field is set to 108 and `is_default_columns` returns true, so `columns` is omitted from the serialized TOML. Later, the user opens the same `.ops.toml` in an 80-column terminal. `default_columns()` now returns 72, so the effective columns value silently changes from 108 to 72 without the user changing anything.

Fix: Use a fixed default (e.g., 80) for serialization skip detection, and resolve terminal width only at runtime display time:
```rust
const DEFAULT_COLUMNS: u16 = 80;

fn default_columns() -> u16 {
    terminal_size::terminal_size()
        .map(|(w, _)| w.0 * 9 / 10)
        .unwrap_or(DEFAULT_COLUMNS)
}

fn is_default_columns(v: &u16) -> bool {
    *v == DEFAULT_COLUMNS
}
```
This way, the skip predicate is deterministic, and the terminal-responsive behavior is preserved as the runtime default.
<!-- SECTION:DESCRIPTION:END -->
