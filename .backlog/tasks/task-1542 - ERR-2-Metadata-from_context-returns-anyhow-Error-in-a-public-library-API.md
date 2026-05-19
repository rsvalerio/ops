---
id: TASK-1542
title: 'ERR-2: Metadata::from_context returns anyhow::Error in a public library API'
status: Done
assignee:
  - TASK-1576
created_date: '2026-05-19 15:24'
updated_date: '2026-05-19 17:48'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:135`

**What**: `pub fn from_context(ctx: &mut Context, registry: &DataRegistry) -> Result<Self, anyhow::Error>` is the only fallible constructor on the public `Metadata` type and it surfaces `anyhow::Error` in its signature.

**Why it matters**: ERR-2 forbids `anyhow::Error` and `Box<dyn Error>` in library/crate-public APIs because they erase the error variant set — downstream code cannot match on failure modes without string sniffing. The sister extension code in this repo (e.g. `ops_extension::DataProviderError`) defines a typed error per provider. `Metadata::from_context` should return either `Result<Self, DataProviderError>` (the parent context API already uses this shape) or a crate-local `MetadataError` enum that names the cases (`MissingMetadataValue`, `RegistryProvideFailed { source }`). `anyhow::Error` is fine *inside* the function and at the binary boundary; keep it out of the public type's signature.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Metadata::from_context returns a typed error (e.g. DataProviderError or a crate-local MetadataError) rather than anyhow::Error
- [ ] #2 Downstream consumers can match on failure variants without string inspection
- [ ] #3 The anyhow dependency, if still needed, is confined to internal call sites and the crate's binary surface
<!-- AC:END -->
