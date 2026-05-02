---
id: TASK-0926
title: >-
  SEC-33: Cargo.toml readers in extensions-rust have no byte cap (DoS via
  oversized manifest)
status: Done
assignee: []
created_date: '2026-05-02 15:32'
updated_date: '2026-05-02 16:53'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:217-234, 373-403`; `extensions-rust/about/src/units.rs:113-153`

**What**: `CargoTomlProvider::provide`, `manifest_declares_workspace`, and `read_crate_metadata` all call `std::fs::read_to_string(...)` on workspace-root and per-crate `Cargo.toml` paths with no size limit. The walk in `find_workspace_root` reads up to MAX_ANCESTOR_DEPTH (64) candidate manifests, and `read_crate_metadata` is called once per declared workspace member. A multi-GiB file at any of these paths, or a symlink pointing at `/dev/zero`, will be slurped into a single allocation. TASK-0831 fixed the analogous gap in `extensions/about/src/manifest_io.rs` (Java/Node providers) but did not touch the Rust extensions, which use raw `std::fs::read_to_string` instead of `read_optional_text`.

**Why it matters**: `ops about` runs in user-controlled working directories. An adversarial repository (cloned for inspection, or a workspace member glob pointing at an attacker-controlled path) makes `ops about` / `ops cargo-toml` provider OOM or stall the CLI. Higher impact than TASK-0831 because the Cargo.toml walk fans out across every workspace member.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 New shared helper (or reuse of read_optional_text semantics) caps Cargo.toml reads at a documented maximum (e.g. 4 MiB) using File::open + Read::take(cap).read_to_string, returning a typed too-large outcome (NotFound-equivalent for the walk, warn for the per-crate reader).
- [ ] #2 All three call sites in extensions-rust/cargo-toml/src/lib.rs (provide, manifest_declares_workspace) and extensions-rust/about/src/units.rs::read_crate_metadata route through the capped reader.
- [ ] #3 New test creates a >cap manifest and proves the helper bails without reading past the cap; existing find_workspace_root behaviour for missing/normal files unchanged.
- [ ] #4 ops verify and ops qa pass with no new clippy warnings.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Routed all three call sites through new ops_core::text::read_capped_to_string (4 MiB cap, OPS_MANIFEST_MAX_BYTES override). provide() propagates oversize as DataProviderError; manifest_declares_workspace and read_crate_metadata downgrade to false/default after debug log. Helper unit-tested in ops-core text.rs. ops verify + full test suite pass.
<!-- SECTION:NOTES:END -->
