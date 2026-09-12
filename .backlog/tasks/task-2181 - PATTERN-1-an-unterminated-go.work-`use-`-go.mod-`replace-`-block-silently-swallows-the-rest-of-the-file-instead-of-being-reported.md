---
id: TASK-2181
title: 'PATTERN-1: an unterminated go.work `use (` / go.mod `replace (` block silently swallows the rest of the file instead of being reported'
status: Done
assignee: []
created_date: '2026-09-08 07:13'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - pattern
dependencies: []
parent_task_id: 'TASK-2238'
modified_files:
  - extensions-go/about/src/go_work.rs
  - extensions-go/about/src/go_mod.rs
priority: medium
ordinal: 94000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_work.rs:20` (`parse_use_dirs`), `extensions-go/about/src/go_mod.rs:37` (`parse`)

**What**: Both parsers track block state in a local flag/enum (`in_use_block`, `block: Option<Block>`) and never check that the block was closed when the loop ends. A manifest whose `)` is missing therefore parses as "every remaining line belongs to the block":

- `go.work`:
  ```
  go 1.21

  use (
  	./api
  go 1.22
  replace ex.com/a => ../b
  ```
  yields `dirs == ["./api", "go 1.22", "replace ex.com/a => ../b"]`. Each becomes a `ProjectUnit` in `modules::collect_units`, so the module count is inflated and `unit_from_use_dir` performs a `cwd.join("go 1.22")` / `cwd.join("replace ex.com/a => ../b")` `go.mod` probe against nonsense paths.
- `go.mod`: the same shape inside an unterminated `replace (` block routes every following line through `parse_replace_directive`, so a trailing `go 1.22` is dropped and the toolchain version vanishes from the About card with no diagnostic.

The crate already treats every other malformed-block shape as worth a diagnostic — `go_work.rs:52` warns and skips on a *nested* `use(` opener — so silence on the unterminated case is an inconsistency in the same function, not a deliberate policy.

**Why it matters**: A hand-edited or truncated manifest produces confidently wrong output (inflated module counts, missing Go version, filesystem probes on paths derived from arbitrary manifest text) with nothing in the logs to explain it. This is the same failure class as TASK-1724 and TASK-1216, which were both fixed by making the structural mismatch visible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_use_dirs and go_mod::parse detect a block still open at EOF and emit exactly one tracing::warn! naming the manifest and the unterminated directive
- [ ] #2 Lines consumed by an unterminated block are not surfaced as use directives / local replaces (or the behaviour is explicitly documented as recovery), so module counts are not inflated by manifest prose
- [ ] #3 No go.mod probe is issued for a path derived from a line absorbed by an unterminated block
- [ ] #4 Tests cover an unterminated use ( block in go.work and an unterminated replace ( block in go.mod, asserting both the diagnostic and the resulting directive list
<!-- AC:END -->
