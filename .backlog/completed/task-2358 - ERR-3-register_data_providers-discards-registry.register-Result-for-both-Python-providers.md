---
id: TASK-2358
title: 'ERR-4: register_data_providers discards registry.register Result for both Python providers'
status: Done
assignee: []
created_date: '2026-10-04 14:09'
updated_date: '2026-10-04 14:31'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-4:extensions-python/about/src/lib.rs:AboutPythonExtension::register_data_providers'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:41-42`

**Rule note**: no rule names a discarded `Result` exactly; filed under ERR-4 (error handling without diagnostics) as nearest.

**What**: `let _ = registry.register(DATA_PROVIDER_NAME, ...)` and `let _ = registry.register(units::PROVIDER_NAME, ...)` drop the registration result. A duplicate-name or rejected registration leaves the Python identity/units providers silently missing, with no log.

**Why it matters**: A failed registration surfaces only as an empty About card; at minimum a `tracing::warn!` naming the provider and error is needed. <!-- scan confidence: confirm the register Result type and whether the macro signature permits propagating the error -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A registration failure is logged (tracing::warn! with provider name and error) or propagated
- [ ] #2 A test covers a duplicate registration not being silent
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Closed as invalid: let _ = registry.register(...) is a deliberate, documented pattern (the must_use message on register names it as the accepted way to ignore a duplicate), used in about 40 places across the extension crates. No code change.
<!-- SECTION:NOTES:END -->
