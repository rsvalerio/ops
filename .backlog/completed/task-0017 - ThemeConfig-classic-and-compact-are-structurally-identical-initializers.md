---
id: TASK-0017
title: 'ThemeConfig::classic and ::compact are structurally identical initializers'
status: Done
assignee: []
created_date: '2026-04-10 22:45:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-code-duplication
  - CD
  - DUP-2
  - low
  - crate-core
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/core/src/config/theme_types.rs:95-141`
**Anchor**: `fn classic`, `fn compact`
**Impact**: `ThemeConfig::classic()` and `ThemeConfig::compact()` each construct a `Self { … }` literal with all 17 fields in the same order, differing only in the string/char literal values. This is a textbook DUP-2 pattern (similar-structure functions differing only in literals), though with only 2 functions it sits below the default threshold of 3+. The duplication is mitigated by the `#[cfg(any(test, feature = "test-support"))]` gate — these are test-support constructors, not production hot paths.

**Notes**:
Both functions share identical structure:
- Same 17 fields in the same order
- Same types (`.into()` for strings, direct literals for chars/numbers)
- `icon_running: String::new()` and `left_pad: 1` are identical in both

The duplication is low-impact today (2 variants), but will scale linearly if more built-in themes are added. A table-driven approach or `Default` + struct-update syntax could reduce repetition:

```rust
// Option A: struct-update from a base
fn compact() -> Self {
    Self {
        icon_pending: "\u{25CB}".into(),
        // ... only fields that differ from classic
        ..Self::classic()
    }
}

// Option B: theme definitions as data (if many themes expected)
```

Rated low because: (a) only 2 functions, below the 3+ threshold; (b) both are `cfg(test/test-support)` gated; (c) the built-in theme definitions are also present in `.default.ops.toml` (the TOML source of truth), so these Rust constructors are secondary. Per DUP-9, the current explicit form is acceptable if no further themes are planned.
<!-- SECTION:DESCRIPTION:END -->
