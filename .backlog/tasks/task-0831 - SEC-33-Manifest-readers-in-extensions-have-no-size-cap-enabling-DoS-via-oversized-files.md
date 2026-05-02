---
id: TASK-0831
title: >-
  SEC-33: Manifest readers in extensions-* have no size cap, enabling DoS via
  oversized files
status: Done
assignee: []
created_date: '2026-05-02 09:11'
updated_date: '2026-05-02 11:58'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_io.rs:33` (called by `extensions-java/about/src/maven/pom.rs:80`, `extensions-node/about/src/package_json.rs:77`, `extensions-node/about/src/units.rs:80,104`)

**What**: `read_optional_text` calls `std::fs::read_to_string` with no size limit. Every manifest parser (`pom.xml`, `package.json`, `pnpm-workspace.yaml`, `gradle.properties` via `for_each_trimmed_line`) inherits this. A multi-GB file at any of these paths, or a symlink pointing to `/dev/zero`, will be slurped into memory in a single allocation.

**Why it matters**: `ops about` runs as part of an interactive CLI in user-controlled working directories. An adversarial repository (cloned for inspection) can make `ops about` allocate arbitrary RAM and either OOM or stall the tool.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_optional_text accepts (or hardcodes) a max byte cap (e.g., 4 MiB) and uses File::open + take(cap).read_to_string, returning None (with tracing::warn!) when exceeded
- [ ] #2 Test with a synthetic >cap file proves the helper bails without reading past the cap
- [ ] #3 Documented cap is referenced from each call site that previously said 'trusted manifest'
<!-- AC:END -->
