---
id: TASK-0983
title: 'DUP-3: stack_emoji and language_emoji duplicate language-to-glyph mapping'
status: Done
assignee: []
created_date: '2026-05-04 21:58'
updated_date: '2026-05-04 23:08'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/format.rs:29-41` (stack_emoji) and `crates/core/src/project_identity/format.rs:56-87` (language_emoji)

**What**: Both functions answer "give me the glyph for this language label" and agree on six entries — Rust 🦀 (\\u{1f980}), Go 🐹 (\\u{1f439}), Python 🐍 (\\u{1f40d}), Java ☕ (\\u{2615}), Terraform 💠 (\\u{1f4a0}), Ansible 🅰️ (\\u{1f170}\\u{fe0f}). They differ on (a) the JS/Node mapping (stack ⬢ \\u{2b22} vs language 🟨 \\u{1f7e8}) and (b) the fallback glyph (📚 \\u{1f4da} vs 📄 \\u{1f4c4}). Adding a new language to the about card today requires editing two tables and remembering which arm covers which call site.

**Why it matters**: silent drift is already present (Node renders one of two emojis depending on whether the user reads the stack field or the codebase breakdown for the same project). Consolidate the canonical mapping into a single `language_glyph(name) -> &'static str` and let each caller post-process for the deliberate stack-vs-codebase rendering difference (or accept one shared glyph). Keeps future per-language rendering changes from re-introducing inconsistency.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single language-to-glyph table; stack_emoji and language_emoji either share it or document the deliberate divergence inline
- [ ] #2 Node/JavaScript renders consistently across stack and codebase blocks (or divergence documented with rationale)
- [ ] #3 Adding a new language requires editing exactly one mapping
<!-- AC:END -->
