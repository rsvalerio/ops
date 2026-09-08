---
id: TASK-2197
title: >-
  READ-13: ops-metadata crate docs narrate removed code and superseded
  implementations instead of the end state
status: To Do
assignee:
  - TASK-2248
created_date: '2026-09-08 07:15'
updated_date: '2026-09-08 11:01'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/metadata/src/lib.rs
  - extensions-rust/metadata/src/ingestor.rs
priority: low
ordinal: 110000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:4`, `extensions-rust/metadata/src/lib.rs:100`, `extensions-rust/metadata/src/lib.rs:186`, `extensions-rust/metadata/src/ingestor.rs:225`

**What**: rustdoc-visible `//!` and `///` blocks in this crate document the
journey rather than the API:

- `lib.rs:4-24` — a 21-line `# No typed accessor layer (ARCH-9 / TASK-1898)`
  section on the crate root, describing a `types.rs` that no longer exists,
  its line count, that it had "nine blanket `#[allow(dead_code)]` attributes",
  and the reasoning for deleting it. This is the first thing `cargo doc`
  renders for the crate.
- `lib.rs:100-112` (`resolve_metadata_max_bytes`) — "Previously any
  unparseable, zero, or oversized value resolved silently to …".
- `lib.rs:186-194` (`CARGO_METADATA_ARGS`) — "The previous pin read
  `include_str!("../lib.rs")` and searched it for the literal …", explaining a
  test that was replaced.
- `ingestor.rs:225-235` (`StagedFile`) — "… instead of the single pre-`Ok` call
  site this replaced".

**Why it matters**: READ-13 — docs describe the end state, not the change that
produced it. These blocks are meaningless to a reader using the crate, go stale
on the next edit while looking authoritative (the `lib.rs:18` claim about
`crates/cli/src/main.rs` is already load-bearing prose about a *different*
crate's file), and the removal essay documents code the reader cannot see. The
enduring content — "consumers read the provider's JSON shape with `serde_json`;
see `MetadataProvider::schema`" and the validation/clamping policy itself — is
one or two sentences. The rest belongs in the PR description or an ADR under
`.backlog/decisions/`.

Note this is about **rustdoc** blocks specifically; the `//` line comments that
cite rule IDs at the point of a non-obvious decision are the project's
convention and are not in scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The crate-root //! docs describe what ops-metadata provides and how to consume it, with no narration of the removed typed-accessor layer
- [ ] #2 The /// blocks on resolve_metadata_max_bytes, CARGO_METADATA_ARGS and StagedFile state the current contract without describing what they replaced
- [ ] #3 Any rationale worth keeping is moved to .backlog/decisions/ or the relevant task, not deleted silently
<!-- AC:END -->
