---
id: TASK-1758
title: >-
  DUP-3: contains_control_chars is a verbatim copy of the node provider's helper
  and carries a redundant DEL clause
status: To Do
assignee:
  - TASK-1992
created_date: '2026-08-27 11:19'
updated_date: '2026-08-28 14:11'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions-python/about/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:377-379`, duplicating `extensions-node/about/src/repo_url.rs:20-22`

**What**: Both crates define, character for character:

```rust
fn contains_control_chars(raw: &str) -> bool {
    raw.chars().any(|c| c.is_control() || c == '\u{007f}')
}
```

Two separate problems in three lines:

1. **Duplication.** The two copies implement the *same* stated policy — the Python doc comment (`lib.rs:370-376`) even says "Sister policy to `extensions-node/about::repo_url::contains_control_chars` (TASK-1165)". The crate already routes its other shared string policy through a single source location: `trim_nonempty` was centralised into `ops_about::text_util` by DUP-3 / TASK-1258 for exactly this reason (`lib.rs:385-388`). The control-char guard is the identical case and was left behind. Divergence here is a security divergence, not a cosmetic one: a fix applied to one copy silently leaves the other stack exposed.

2. **The `|| c == '\u{007f}'` clause is dead.** `char::is_control` returns true for the whole Unicode `Cc` category, which is `U+0000..=U+001F` **and** `U+007F..=U+009F`. DEL is therefore already matched by `c.is_control()`. The clause suggests to a reader that `is_control` has a gap it does not have, and the doc comment above it repeats the same misconception ("plus the broader `char::is_control` set covering C1"), inverting the actual relationship.

**Why it matters**: DUP-3 severity here is raised by the subject matter — this is the SEC-2 / TASK-1207 sanitisation boundary for attacker-controlled manifest values. Two copies means the SEC-11 scheme-allowlist work (TASK-1755 for Python, TASK-1722 for Node) will likewise be applied twice or, more likely, once.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 contains_control_chars lives in one place (ops_about::text_util, alongside trim_nonempty) and both the Python and Node providers call it
- [ ] #2 The redundant `|| c == '\\u{007f}'` clause is removed and the doc comment states correctly that char::is_control already covers C0, DEL and C1
- [ ] #3 A test pins that U+007F and a C1 code point (e.g. U+0085) are both rejected, so the simplification is not a behaviour change
- [ ] #4 The Python provider's local copy is deleted, not left as a wrapper
<!-- AC:END -->
