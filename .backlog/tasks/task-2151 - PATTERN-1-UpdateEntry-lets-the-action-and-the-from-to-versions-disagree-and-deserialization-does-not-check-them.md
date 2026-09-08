---
id: TASK-2151
title: >-
  PATTERN-1: UpdateEntry lets the action and the from/to versions disagree, and
  deserialization does not check them
status: To Do
assignee:
  - TASK-2243
created_date: '2026-09-08 07:03'
updated_date: '2026-09-08 10:57'
labels:
  - code-review-rust
  - pattern
dependencies: []
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
priority: medium
ordinal: 64000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:48`-`57`

**What**: `UpdateEntry` pairs `action: UpdateAction` with `from: Option<String>` and `to: Option<String>`, where the action fully determines which of the two must be present:

- `Update` / `Downgrade` — both `Some`
- `Add` — `from` is `None`, `to` is `Some`
- `Remove` — `from` is `Some`, `to` is `None`

Nothing in the type enforces that. Sixteen `(from, to)` presence combinations are representable per action; four are valid. The doc comments state the rule in prose ("None for Add actions", "None for Remove actions") rather than in the type.

The parser is careful to only build valid combinations, but the type also derives `Deserialize` and is read back from a cache (`CargoUpdateResult` is documented at line 66-68 as being consumed from the about page's cache), so an older or hand-edited payload such as `{"action":"add","from":"1.0.0","to":null}` deserializes without complaint into a state the parser can never produce.

An enum carrying exactly the versions each action has — e.g. `Update { from, to }` / `Downgrade { from, to }` / `Add { to }` / `Remove { from }` — makes the invalid combinations unrepresentable and keeps the same serialized JSON shape reachable via serde attributes.

**Why it matters**: Every consumer of the provider JSON has to defend against combinations that are supposed to be impossible, and there is no single place that says which ones are. The failure mode is silent: an entry whose action says `Add` but whose `from` is populated renders as a downgrade-shaped row on the about page with nothing flagged, and no test can cover a state the parser cannot reach but the deserializer accepts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 An UpdateEntry whose action and version presence disagree is unrepresentable in the type, not merely undocumented
- [ ] #2 Deserializing a payload with a mismatched action/version combination fails or is normalized, rather than producing a silently invalid entry
- [ ] #3 The serialized JSON shape the about page consumes is unchanged, or the change is covered by the existing serde_default back-compatibility test
- [ ] #4 A test covers each action's version presence on both the serialize and deserialize direction
<!-- AC:END -->
