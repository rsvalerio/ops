---
id: TASK-1729
title: >-
  PATTERN-1: strip_trailing_yaml_comment confuses bytes with chars —
  char::from(b) misreads UTF-8 continuation bytes as whitespace
status: Triage
assignee: []
created_date: '2026-08-27 11:12'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - extensions-node/about/src/units.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:326-346` (`strip_trailing_yaml_comment`), specifically line 343

**What**: The function iterates `s.as_bytes()` but interprets each raw byte as a character:

```rust
prev_ws = char::from(b).is_whitespace();
```

`char::from(u8)` maps a byte to U+0000..U+00FF (Latin-1), which is only correct for ASCII. For a multi-byte UTF-8 scalar the loop sees *continuation bytes* (0x80..0xBF) and converts them to C1/Latin-1 code points. Two of those are `White_Space=Yes`: U+0085 (NEL) and U+00A0 (NBSP). So any character whose final UTF-8 byte is 0xA0 or 0x85 sets `prev_ws = true`, and a `#` immediately after it is then misclassified as the start of a YAML comment and the value is truncated.

Concrete case: `packages: ["Ġ#literal"]`. U+0120 encodes as `C4 A0`; the trailing `A0` becomes NBSP, `prev_ws` is set, and the following `#` truncates the entry to `"Ġ` — a glob that matches nothing. The same holds for U+30A0, U+4E20, and every other scalar ending in a 0xA0 or 0x85 continuation byte. `pnpm_hash_inside_quotes_is_not_a_comment` (`units.rs:691`) pins the ASCII case only, so the bug is invisible to the suite.

The quote-tracking arms in the same loop are safe by accident (`'` and `"` are ASCII and continuation bytes are all >= 0x80), so `prev_ws` is the sole defect.

**Why it matters**: `pnpm-workspace.yaml` is external input and non-ASCII directory names are legal. The failure is silent — the truncated glob simply resolves to no members, and the About card reports a workspace as empty with no diagnostic. It is the same class of silently-wrong-glob problem that PATTERN-1 / TASK-1061, TASK-1084 and TASK-1168 were each filed for, and the module already carries a stated policy of keeping hand-rolled-parser failures visible rather than silent.

**Fix shape**: iterate `s.char_indices()` instead of `s.as_bytes().iter().enumerate()` and match on `char` (`'\''`, `'"'`, `'#'`), setting `prev_ws = c.is_whitespace()`. The returned `&str` slicing already uses a byte index, which `char_indices` supplies directly, so the `s.get(..i).unwrap_or(s)` recovery can become a plain `s[..i]`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 strip_trailing_yaml_comment iterates chars (char_indices), not raw bytes, and computes prev_ws from a char
- [ ] #2 A pnpm packages entry containing a multi-byte scalar whose UTF-8 encoding ends in 0xA0 (e.g. U+0120) followed by a literal # inside quotes is not truncated
- [ ] #3 Existing comment-stripping tests (quoted item, unquoted item, hash inside quotes) still pass
<!-- AC:END -->
