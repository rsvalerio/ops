---
id: TASK-2224
title: 'API-14: ops-about-terraform''s only public type has no doc summary'
status: To Do
assignee:
  - TASK-2247
created_date: '2026-09-08 07:22'
updated_date: '2026-09-08 11:00'
labels:
  - code-review-rust
  - api-design
dependencies: []
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 131000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:51`

**What**: The crate exposes exactly one public item and it carries no `///` summary:

```rust
#[non_exhaustive]
pub struct AboutTerraformExtension;
```

The crate-level `//!` docs describe the parsing policy at length but never say what this type is or that it is the `linkme`-registered `Extension` entry point. Everything else in the module is correctly private, so this single item is the whole rustdoc surface — and it renders as a bare name.

**Why it matters**: `cargo doc` for this crate produces one undescribed struct. The same defect was filed per-crate for the sibling about extensions — TASK-2163 (`ops-about-rust`) and TASK-2187 (`ops-about-go`) — and the fix must land in this file to close it here. `ops-about-python` and `ops-about` (TASK-2071) share the shape; fixing them consistently gives the extension host's public surface a uniform description.

**Note**: unlike the Rust and Go twins, this crate has no `pub` crate-internal helpers to demote — the doc summary is the entire finding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 AboutTerraformExtension carries a doc summary saying what it is and that it is the registered Terraform about extension
- [ ] #2 Wording is consistent with the summaries added for TASK-2163 and TASK-2187
<!-- AC:END -->
