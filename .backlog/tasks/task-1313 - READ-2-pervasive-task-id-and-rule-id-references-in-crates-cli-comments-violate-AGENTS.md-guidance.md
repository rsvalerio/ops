---
id: TASK-1313
title: >-
  READ-2: pervasive task-id and rule-id references in crates/cli comments
  violate AGENTS.md guidance
status: Done
assignee:
  - TASK-1387
created_date: '2026-05-11 20:25'
updated_date: '2026-05-13 07:58'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/` — 180 occurrences across 20 files

**What**: Source comments throughout `crates/cli/src/` cite backlog task IDs and code-review rule IDs as part of the explanation, e.g.:

- `crates/cli/src/tools_cmd.rs:25` — `// TEST-25 / TASK-1295: rendering split out so tests feed a deterministic Vec<ToolInfo>...`
- `crates/cli/src/tools_cmd.rs:35` — `// DUP-3 / TASK-1235: column padding routes through the shared ...`
- `crates/cli/src/tools_cmd.rs:44` — `// READ-7 / TASK-0896: ToolStatus is #[non_exhaustive], so the ...`
- `crates/cli/src/theme_cmd.rs:89` — `// READ-2 (TASK-0936): theme names are user-supplied via [themes.<name>] ...`
- `crates/cli/src/run_cmd.rs:82` — `// ARCH-3 / TASK-1285: build_runner used to accept a verbose: bool that it never read`
- `crates/cli/src/subcommands.rs:197` — `// TASK-1282: collapsed run_before_* wrappers ...`

Distribution (file: count via `rg 'TASK-\d+|READ-\d+|TEST-\d+|DUP-\d+|API-\d+|ARCH-\d+|PATTERN-\d+|ERR-\d+|CONC-\d+' crates/cli/src`):
run_cmd.rs 18, extension_cmd.rs 14, theme_cmd.rs 14, init_cmd.rs 14, subcommands.rs 13, registry/tests.rs 15, run_cmd/tests.rs 14, registry/registration.rs 12, registry/discovery.rs 10, help.rs 10, hook_shared.rs 8, tools_cmd.rs 7, main.rs 6, new_command_cmd.rs 6, about_cmd.rs 5, plus 5 more files.

**Why it matters**: AGENTS.md (root) explicitly says: *"Don't reference the current task, fix, or callers (\"used by X\", \"added for the Y flow\", \"handles the case from issue #123\"), since those belong in the PR description and rot as the codebase evolves."* These comments will be increasingly opaque as task numbers age out; a future maintainer who reads `READ-7 / TASK-0896` has to leave the file (and the repo) to learn what the rule and task said. The genuine *why* (e.g. "ToolStatus is `#[non_exhaustive]`") is usually present but buried under the rule-ID prefix.

Cleanup should preserve any genuinely-load-bearing *why* sentence and drop the `TEST-25 / TASK-1295:` / `READ-2 (TASK-0936):` / `ARCH-3 / TASK-1285:` prefixes. Comments that only restate "this was added by task X" can be deleted outright.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Comments in crates/cli/src/ no longer prefix-tag explanations with rule IDs (READ-*, TEST-*, DUP-*, ARCH-*, etc.) or backlog task numbers (TASK-NNNN)
- [ ] #2 Where the comment carried a real 'why', that sentence is preserved (or replaced with a clearer, ID-free phrasing); comments that only said 'added by TASK-X' are removed
- [ ] #3 rg 'TASK-\d+|(READ|TEST|DUP|API|ARCH|PATTERN|ERR|CONC)-\d+' crates/cli/src returns zero hits (or only obvious unrelated matches such as license headers)
<!-- AC:END -->
