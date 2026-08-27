---
id: TASK-1803
title: >-
  TEST-9: the parser has eight regression-fixed bugs and no property test,
  despite proptest already being a workspace dependency
status: Triage
assignee: []
created_date: '2026-08-27 11:26'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/cargo-update/src/tests.rs
  - extensions-rust/cargo-update/Cargo.toml
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/tests.rs` (whole suite), covering `extensions-rust/cargo-update/src/lib.rs:100-168` (`parse_update_output`) and `:183-236` (`strip_ansi`)

**What**: Every test in this crate is an example-based test over a
hand-written literal. That suite has been extended eight separate times by
eight separate bug reports, each found by inspection rather than by testing:

| Task | Bug |
|---|---|
| TASK-0472 | verb-prefixed line dropped with no log |
| TASK-0613 | `Updating` trailing tokens glued into the version |
| TASK-0882 | `bytes[i] as char` corrupted every multi-byte UTF-8 character |
| TASK-0949 | `Adding`/`Removing` trailing tokens glued into the version |
| TASK-0970 | needless allocation on the no-escape path |
| TASK-1028 | truncated CSI drained the iterator to EOF, swallowing visible text |
| TASK-1030 | `Updatingxyz` matched as a verb |
| TASK-1054 | `contains("index")` dropped every crate named `*index*` |

Six of the eight are input-shape bugs on a byte-oriented scanner over
untrusted external output — exactly TEST-9's stated sweet spot ("parser
correctness: arbitrary input never panics", "large input space, edge cases hard
to enumerate manually"). `proptest` is already a workspace dependency (root
`Cargo.toml`), so adoption cost is one dev-dependency line.

Properties worth pinning, each of which would have caught at least one of the
rows above:

- `parse_update_output(arbitrary bytes)` never panics and never loops (covers TASK-1028's EOF drain).
- `strip_ansi(s) == s` for any `s` containing no `\x1b` (covers TASK-0882's UTF-8 corruption and TASK-0970's fast path in one property).
- `strip_ansi` output never contains a CSI sequence, and its visible-character subsequence is preserved for arbitrary interleavings of text and escapes (covers TASK-1028).
- Round-trip: rendering an arbitrary `UpdateEntry` back into a cargo-shaped line and re-parsing yields the same entry (covers TASK-0613 / TASK-0949 / TASK-1030 / TASK-1054 as a family instead of one literal at a time).
- `entries.len() == update_count + add_count + remove_count` for any input — an invariant the current suite only checks on six hand-written fixtures.

**Why it matters**: not that the existing tests are wrong, but that the crate's
demonstrated failure mode is "an input shape nobody thought to write down".
Eight rounds of example-based patching is the evidence that example-based
testing is not converging here; the last hand-written fixture will not be the
last bug.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 proptest is added as a dev-dependency via the workspace entry
- [ ] #2 A property test asserts parse_update_output never panics on arbitrary byte input
- [ ] #3 A property test asserts strip_ansi is the identity on escape-free input (including non-ASCII) and leaves no CSI sequence in its output
- [ ] #4 A property test asserts entries.len() equals update_count + add_count + remove_count for arbitrary input
- [ ] #5 A generated-line round-trip property covers the verb/version/trailing-token shapes currently pinned one literal at a time
<!-- AC:END -->
