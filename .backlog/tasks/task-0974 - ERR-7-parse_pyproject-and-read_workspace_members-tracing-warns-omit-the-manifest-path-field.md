---
id: TASK-0974
title: >-
  ERR-7: parse_pyproject and read_workspace_members tracing warns omit the
  manifest path field
status: Triage
assignee: []
created_date: '2026-05-04 21:57'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions-python/about/src/lib.rs:181-188` (`parse_pyproject` parse-failure warn)
- `extensions-python/about/src/units.rs:62-68` (`read_workspace_members` parse-failure warn)

**What**: Both warn events log `error = %e` and a static recovery string but have no `path` field. The sister sites (`extensions-node/about/src/package_json.rs:88` and `units.rs:100`) both include a `path = ?path.display()` field per the TASK-0818 / TASK-0930 sweep policy. After the python provider switched to `manifest_cache::pyproject_text` (PERF-3 / TASK-0854), the path that was previously implicit in `read_optional_text` no longer flows into the warn record, so an operator running multi-root `ops about` cannot tell which `pyproject.toml` failed to project.

**Why it matters**: Operability regression — when one of N project roots emits a pyproject parse warning, the log line gives the toml error text but not the file location. Diverges from the project-wide policy enforced for node (TASK-0930) and from the broader path-log sweep (TASK-0818, TASK-0944, TASK-0945, TASK-0947). Use `?root.join("pyproject.toml").display()` (Debug-formatted per the ERR-7 sweep so newlines / ANSI in attacker-controlled checkout paths cannot forge log lines).

**Candidate fix**:
```rust
tracing::warn!(
    path = ?root.join("pyproject.toml").display(),
    error = %e,
    recovery = "default-identity",
    "failed to project pyproject.toml into identity shape",
);
```
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both python warn sites include a Debug-formatted path field
- [ ] #2 Test pins that the rendered field escapes embedded newlines / ANSI (mirrors pyproject_path_debug_escapes_control_characters)
<!-- AC:END -->
