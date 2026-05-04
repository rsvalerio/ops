---
id: TASK-0977
title: >-
  ERR-7: about::units::read_crate_metadata logs path/error via Display, log
  injection from attacker-controlled workspace member paths
status: Triage
assignee: []
created_date: '2026-05-04 21:58'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/units.rs:120-124, 133-137`

**What**: `read_crate_metadata` emits two tracing breadcrumbs that format the candidate `Cargo.toml` path and IO/parse error through Display:
```
tracing::debug!(path = %crate_toml_path.display(), error = %e, "failed to read crate manifest");
tracing::warn!(path = %crate_toml_path.display(), error = %e, "failed to parse crate manifest as TOML");
```
The path is `cwd.join(member).join("Cargo.toml")` where `member` is an entry from the workspace `[workspace].members` list of an arbitrary cloned repo. Members are attacker-controlled — a Cargo.toml that declares e.g. `members = ["a\n[FAKE-LOG] forged\u{1b}[31m"]` lands in operator logs verbatim because Display does not escape control characters or ANSI escapes. Same anti-pattern fixed in TASK-0941 (about query.rs glob walk), TASK-0944 (core text helper), TASK-0945 (stack detection), TASK-0947 (cargo-toml walk), TASK-0965 (core config loader).

**Why it matters**: `ops about` runs against any cloned repo and the units provider walks every workspace member, so a malicious upstream Cargo.toml can forge log records (fake severity lines, hide subsequent diagnostics, inject ANSI escapes) at debug+warn level. Sweep gap relative to TASK-0941/0947/0965.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both tracing breadcrumbs format path / error via Debug (?) instead of Display (%)
- [ ] #2 Regression test exercises read_crate_metadata against a path containing \n and \x1b and asserts captured log lines escape both
<!-- AC:END -->
