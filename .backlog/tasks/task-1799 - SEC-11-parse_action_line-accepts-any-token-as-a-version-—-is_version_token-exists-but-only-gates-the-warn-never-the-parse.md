---
id: TASK-1799
title: >-
  SEC-11: parse_action_line accepts any token as a version — is_version_token
  exists but only gates the warn, never the parse
status: To Do
assignee:
  - TASK-1995
created_date: '2026-08-27 11:25'
updated_date: '2026-08-28 14:12'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
  - extensions-rust/cargo-update/src/tests.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:332-398` (`parse_action_line`), `:88-90` (`strip_v_prefix`), `:323-326` (`is_version_token`)

**What**: The crate already has a version-shape predicate —
`is_version_token` (`v` followed by an ASCII digit, `:323`) — but it is used
only by `starts_with_known_verb` to decide whether to *log* a drift warning.
The parse path never validates anything: whatever token sits in the version
position is passed through `strip_v_prefix` and stored as the version.

Concrete results, all reachable from real cargo output shapes:

- `Adding new-crate (locked) v0.1.0` -> `to: Some("(locked)")`. The token order
  is different from the shape TASK-0949 anticipated, so the trailing-token
  warn fires on `v0.1.0` while `(locked)` is silently published as the version.
- `Updating serde v1.0.0 -> latest` -> `to: Some("latest")`.
- `Adding foo v` -> `strip_v_prefix("v")` returns `""` (pinned by
  `strip_v_prefix_just_v`, `tests.rs:533`), so `to: Some("")` — an empty string
  presented to consumers as a valid version.
- `Removing old-crate ???` -> `from: Some("???")`.

Layer 3 of SEC-11 (format validation) is simply absent at this boundary, even
though the predicate that implements it is sitting ten lines up the file.

**Why it matters**: this is the crate's system boundary — the values come from
a subprocess whose output is shaped by crate names, version strings and
registry metadata. The parsed values are serialised into the provider JSON
(`:463`) and rendered by the about page with no downstream validation. The
crate's stated design elsewhere is loud-on-drift (three `tracing::warn!` sites);
here it is silent-and-wrong, publishing a plausible-looking field that no
consumer can distinguish from a real version. `Some("")` is the worst case: a
present-but-empty version reads as "known" to every consumer that checks
`is_some()`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 parse_action_line validates the version position with is_version_token (or an equivalent shape check) before accepting the line
- [ ] #2 A token that does not look like a version produces no entry and reaches the existing format-drift warn, rather than being published as the version
- [ ] #3 strip_v_prefix can no longer yield an empty version into UpdateEntry.from/.to
- [ ] #4 Tests cover 'Adding foo v', a non-version token in the version position, and confirm the legitimate v-prefixed and bare-numeric forms still parse (parse_no_v_prefix_passthrough must keep passing or be consciously re-specified)
<!-- AC:END -->
