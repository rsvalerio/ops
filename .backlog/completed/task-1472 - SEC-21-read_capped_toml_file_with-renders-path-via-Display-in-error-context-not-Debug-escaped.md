---
id: TASK-1472
title: >-
  SEC-21: read_capped_toml_file_with renders path via Display in error context,
  not Debug-escaped
status: Done
assignee:
  - TASK-1478
created_date: '2026-05-16 10:06'
updated_date: '2026-05-17 07:18'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:82-83,93-96`

**What**: The `with_context`/`bail!` formatters render `path.display()` via Display, which preserves control characters in the path; the analogous `for_each_trimmed_line` warn at `text.rs:287` uses `?path.display()` (Debug-formatting) precisely to prevent log/CLI forging — see ERR-7 / TASK-0944 already in the codebase comments.

**Why it matters**: `ops` accepts user-controlled cwds. A `.ops.toml` whose path contains an ANSI ESC or a newline will, on read failure, flow into both anyhow chains rendered by `crate::ui::error` (which does sanitise) and into `tracing::warn!` fields rendered Display-style — defeating the SEC-21 escape contract the rest of the crate enforces.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace path.display() in the with_context/bail! format strings with Debug-rendered output (e.g. format!("...{:?}", path.display())), matching the policy enforced at text.rs:287 and stack/detect.rs:78
- [ ] #2 Add a unit test that constructs a path containing \n / \x1b and asserts the rendered error string contains the escaped form, not the raw byte
- [ ] #3 Audit the rest of config/loader.rs and config/edit.rs for the same path.display()-in-format anti-pattern and standardise
<!-- AC:END -->
