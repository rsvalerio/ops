---
id: TASK-1519
title: >-
  PATTERN-1: deps LicenseEntry/BanEntry/SourceEntry public type aliases collapse
  to DenyEntry, eroding compile-time class distinction
status: Done
assignee:
  - TASK-1648
created_date: '2026-05-19 07:28'
updated_date: '2026-05-25 19:03'
labels:
  - code-review-rust
  - pattern
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/lib.rs:73-76`

**What**: Three public type aliases resolve to the same struct:

```rust
pub type LicenseEntry = DenyEntry;
pub type BanEntry = DenyEntry;
pub type SourceEntry = DenyEntry;
```

`DenyResult` then exposes four `Vec<…>` fields where three of those vectors carry the *same* `DenyEntry` type at the type level — the only difference is which struct field name they're parked under. As a result the type system cannot prevent `result.bans.push(LicenseEntry { … })`, and a future refactor that swaps `bans` and `licenses` extraction (e.g. inside `push_diagnostic`, lines 282-316 of `parse/deny.rs`) would compile cleanly while silently sending license findings into the bans bucket.

This contradicts the project's stated design philosophy (`rules.md`: "Use types to represent states, not flags. Encode preconditions in the type system"). The DUP-3/TASK-0972 unification of `severity_icon` / `colorize_severity` already moved severity into a typed `SeverityClass` enum — apply the same posture to diagnostic class.

Recommended fix: introduce a `newtype` per class (`pub struct LicenseEntry(pub DenyEntry);` or distinct structs with the same field shape) so `DenyResult.bans: Vec<BanEntry>` and `DenyResult.licenses: Vec<LicenseEntry>` are non-interchangeable. The serde representation can remain identical.

**Why it matters**: PATTERN-1 (use the type system over flag-like type aliases). Adjacent: API-4 (public types shape downstream usage). The comment on line 73 ("Backwards-compatible type aliases.") suggests the aliases pre-date the diagnostic-class split — they are no longer earning their keep.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 LicenseEntry, BanEntry, SourceEntry no longer alias to DenyEntry; each is a distinct nominal type (newtype or struct)
- [ ] #2 DenyResult fields keep their existing names but carry their now-distinct entry types, and push_diagnostic in parse/deny.rs cannot cross-mix classes (would now fail to compile)
- [ ] #3 JSON serialization shape stays byte-identical — confirmed by the existing deps_report_serialization_round_trip test
<!-- AC:END -->
