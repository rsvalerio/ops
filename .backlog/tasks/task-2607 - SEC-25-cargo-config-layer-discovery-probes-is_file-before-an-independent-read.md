---
id: TASK-2607
title: 'SEC-25: cargo config layer discovery probes is_file() before an independent read'
status: To Do
assignee: []
created_date: '2026-10-10 20:51'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - sec
dependencies: []
parent_task_id: 'TASK-2623'
modified_files:
  - extensions/about/src/machine.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-25:extensions/about/src/machine.rs:config_layers'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/machine.rs:153`

**What**: `config_layers` selects a config file with `["config", "config.toml"].iter().map(|name| dir.join(name)).find(|p| p.is_file())` and then `read_layer(path, ...)` re-resolves the same path with `std::fs::read_to_string` — a check-then-read window in which the file (or a path component) can be swapped for a symlink. The same shape is in `push_with_includes` at `machine.rs:186`: `if optional && !path.is_file() { continue; }` followed by `read_layer(path, base)` reading the path again.

**Why it matters**: SEC-25: any exists/is_file probe followed by an independent open of the same path is racy; the fix is to perform the read directly and let NotFound mean absent — `read_layer` already treats read errors as skip-with-warning, so the precedence selection can try `config` and fall back to `config.toml` on read failure, and the optional-include check can simply drop the pre-probe. Notably, this crate eliminated exactly this window on its manifest paths (`workspace.rs` docs: "there is no window between an `exists()` probe and a later open"; `manifest_io.rs` reads through `read_capped_to_string`, which refuses swapped symlinks) — the cargo-config path predates that posture. Severity Low: `about machine` is a read-only reporter and the files are the user's own cargo config, so the consequence of a race is a wrong config value in a report, not data loss.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 config_layers selects between config/config.toml by attempting the read (NotFound/parse-failure falls back) rather than an is_file pre-probe
- [ ] #2 push_with_includes drops the is_file pre-probe for optional includes and decides optionality from the read result
<!-- AC:END -->
