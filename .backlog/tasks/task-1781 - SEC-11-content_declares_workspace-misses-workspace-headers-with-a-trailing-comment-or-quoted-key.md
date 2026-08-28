---
id: TASK-1781
title: >-
  SEC-11: content_declares_workspace misses [workspace] headers with a trailing
  comment or quoted key
status: To Do
assignee:
  - TASK-1994
created_date: '2026-08-27 11:23'
updated_date: '2026-08-28 14:12'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/cargo-toml/src/workspace_root.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/workspace_root.rs:325` (`content_declares_workspace`), reached from `manifest_declares_workspace` (`:304`) and both `find_workspace_root*` walks.

**What**: TASK-1512 replaced the `toml::Value` parse with a hand-rolled line scan. The scan only accepts a table header when the *entire trimmed line* is bracketed:

```rust
if let Some(inside) = trimmed.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
```

Valid TOML that this rejects:

- `[workspace] # the workspace root` — `strip_suffix(']')` returns `None` because the line ends in `t`, so the header is never inspected. A trailing comment on a table header is ordinary, valid TOML.
- `["workspace"]` / `[ "workspace" ]` — quoted bare keys are valid TOML; `key` becomes `"workspace"` (with quotes) and never equals `workspace`.
- `[workspace] # comment` inside `[workspace.metadata]` form has the same problem (`[workspace.package] # shared`).

Third, related silent-skip path in the same function: `manifest_declares_workspace` returns `false` for *any* non-`NotFound` read error, including a manifest that exceeds the `read_capped_to_string` byte cap — a legitimately large workspace root is skipped rather than reported.

**Why it matters**: a false negative here does not fail loudly — the walk simply keeps climbing. Two consequences:

1. **Wrong root selected.** This is the exact TASK-0501 regression (running `ops about` from inside a member crate silently produced empty units/coverage) reintroduced for any workspace whose root manifest comments its `[workspace]` header. Every downstream provider (about, deps, units, coverage, create-review-tasks) then targets the wrong manifest.
2. **Security-adjacent.** The SEC-25 / TASK-1204 threat model documented on `find_workspace_root` above this function assumes the real root terminates the walk. A false negative makes the walk continue *past* the legitimate root into higher ancestors — precisely the region the doc comment identifies as attacker-plantable. The strict variant's canonicalization check does not help: the planted ancestor is on the canonical chain.

Existing tests (`content_declares_workspace_*` in `src/tests/find_root.rs:305-341`) cover only bare headers, sub-tables, and string-value false positives; none exercises a commented or quoted header.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 content_declares_workspace returns true for a line of the form '[workspace] # comment' and '[workspace.package]   # shared metadata'
- [ ] #2 content_declares_workspace returns true for a quoted table key: ["workspace"] and [ "workspace" ]
- [ ] #3 Existing false-positive guarantees are preserved: [workspace] inside a triple-quoted basic or literal multi-line string, and a '# [workspace]' comment line, still return false
- [ ] #4 A read failure other than NotFound (e.g. the read_capped_to_string byte cap) is surfaced or logged at warn level rather than being indistinguishable from 'no workspace declared'
- [ ] #5 Regression tests added in src/tests/find_root.rs for each accepted and rejected shape above
<!-- AC:END -->
