---
id: TASK-1604
title: 'PATTERN-1: allow_jsonc flag uses json5 parser, which is a superset of JSONC'
status: Done
assignee:
  - TASK-1636
created_date: '2026-05-22 06:43'
updated_date: '2026-05-22 12:17'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/json.rs:5-15`, `extensions/config-checkers/src/lib.rs:58-75`

**What**: `CheckerOptions::allow_jsonc` is named and documented as enabling JSONC (JSON with Comments — the VS Code/`tsconfig.json` dialect: `//` and `/* */` comments plus trailing commas). The implementation routes through `json5::from_str`, but JSON5 is a strict superset of JSONC and additionally accepts unquoted object keys, single-quoted strings, hex/`Infinity`/`NaN` numbers, leading/trailing decimal points, and line continuations.

**Why it matters**: A user enabling `--allow-jsonc` to tolerate comments in `tsconfig.json` will also silently accept files that are not valid JSONC and would be rejected by every JSONC consumer (VS Code, `jsonc-parser`, `serde_jsonc`). That defeats the purpose of a parse-validator: invalid configs ship to production and only break at the real consumer. The flag name promises JSONC; the implementation delivers JSON5. Fix is either to rename the flag (and docs) to `allow_json5`, or swap the parser for a JSONC-only crate (e.g. `serde_jsonc`, `jsonc-parser`).

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Decision recorded: either rename flag/docs to reflect JSON5 semantics, or swap parser to a JSONC-only implementation
- [x] #2 If renamed: public API, CLI flag, and module-level rustdoc all consistently say JSON5; if parser swapped: a regression test asserts a JSON5-only construct (e.g. unquoted key, single-quoted string) is rejected under allow_jsonc=true
- [x] #3 Existing test 'jsonc_fails_strict_passes_with_flag' updated to match the chosen semantics
<!-- AC:END -->
