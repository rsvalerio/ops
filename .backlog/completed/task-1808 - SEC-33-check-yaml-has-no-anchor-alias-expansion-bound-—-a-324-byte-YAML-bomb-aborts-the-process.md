---
id: TASK-1808
title: >-
  SEC-33: check-yaml has no anchor/alias expansion bound — a 324-byte YAML bomb
  aborts the process
status: Done
assignee:
  - TASK-2004
created_date: '2026-08-27 11:31'
updated_date: '2026-08-28 22:24'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/config-checkers/src/yaml.rs
  - extensions/config-checkers/src/lib.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/yaml.rs:14-19` (`check_yaml`), gated by `extensions/config-checkers/src/lib.rs:42` (`DEFAULT_MAX_BYTES`)

**What**: `check_yaml` hands the whole file to `Yaml::load_from_str`, whose loader materialises every alias by *cloning* the anchored node (`saphyr-0.0.6/src/loader.rs:294-311`: `Event::Alias(id) => anchor_map.get(&id)` then `insert_new_node` does `anchor_map.insert(anchor_id, node.clone())`). Nested anchors therefore expand multiplicatively — the classic "billion laughs" YAML bomb. The crate's only defence is the `DEFAULT_MAX_BYTES` byte cap (16 MiB), which measures the *input* size and says nothing about the expanded tree.

Verified against saphyr 0.0.6 (the exact pinned version, `Cargo.lock`) with a standalone probe: 9 nested anchor levels, each aliasing the previous 9 times.

```
input bytes = 324
memory allocation of 1 bytes failed
exit=134   (SIGABRT)
```

A **324-byte** `.yaml` file exhausts the address space. Because Rust's allocation failure path aborts, this is not a catchable parse error — the whole `ops` process dies with SIGABRT, so the checker cannot even report which file did it.

**Why it matters**: SEC-33 — resource consumption is unbounded on untrusted input. `ops check-yaml` is a pre-commit/CI validator whose entire job is to run over files it did not write. On a CI runner validating a contributor's branch, one committed `.yaml` file is a guaranteed OOM-kill of the job; on a developer machine it takes the box's memory with it. The module doc comment on `DEFAULT_MAX_BYTES` explicitly claims to be "a defence against accidental or malicious oversized inputs triggering an allocator/parser DoS on CI runners and pre-commit hosts" — that claim is false for this input class, which makes the gap worse than an unnoticed one.

**Fix shape**: the byte cap cannot express this; the bound has to be on expansion. Options, roughly in order of preference: (1) drive `saphyr_parser::Parser` at the *event* level instead of `Yaml::load_from_str` — a parse-only validator never needs the materialised tree, and an event-stream walk with a counter caps total events and nesting depth in O(1) memory, which also fixes the input-size sensitivity generally; (2) if the loaded tree is kept, reject documents containing aliases outright, or count alias events and fail past a threshold; (3) at minimum, cap nesting depth and total node count. Whatever is chosen, `DEFAULT_MAX_BYTES`'s doc comment must stop claiming a protection it does not provide.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 check_yaml bounds alias/anchor expansion (event-level parse, alias rejection, or an explicit node/depth cap) so it cannot allocate unboundedly
- [x] #2 A regression test feeds a nested-anchor YAML bomb (a few hundred bytes) to check_yaml and asserts it returns CheckError::Parse rather than aborting
- [x] #3 The DEFAULT_MAX_BYTES doc comment no longer claims to defend against parser DoS it cannot bound, and points at the mechanism that does
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-2004 (branch code-review/TASK-2004).

`check_yaml` now drives `saphyr_parser::Parser` at the event level instead of
`Yaml::load_from_str`, so no node is ever materialised and memory is
O(depth + anchors) whatever the document would expand to. On top of that an
`ExpansionBudget` computes the expanded node count a loader *would* produce
(an alias costs whatever its anchor expands to) and rejects past
`yaml::MAX_EXPANDED_NODES` (20M); nesting is capped at
`yaml::MAX_NESTING_DEPTH` (128, matching the JSON checker).

Dependency change: the crate now takes `saphyr-parser` directly and drops
`saphyr` (which only re-exported `ScanError`/`Marker`, not `Parser`).
Accept/reject parity holds: `Yaml::load_from_str` surfaces only the parser's
own `ScanError`s, so the same inputs fail with the same messages.

AC#2 regression test: `yaml::tests::nested_anchor_bomb_is_rejected_instead_of_aborting_the_process`
(a <500-byte nine-level bomb) asserts `CheckError::Parse` with
"input exceeds the expanded node count limit of 20000000".
AC#3: `DEFAULT_MAX_BYTES`'s doc comment now says it bounds input size only and
points at `json::MAX_NESTING_DEPTH` and `yaml::MAX_EXPANDED_NODES`.
<!-- SECTION:NOTES:END -->
