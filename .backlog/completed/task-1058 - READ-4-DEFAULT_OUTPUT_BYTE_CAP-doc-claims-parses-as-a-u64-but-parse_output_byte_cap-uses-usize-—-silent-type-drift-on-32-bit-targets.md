---
id: TASK-1058
title: >-
  READ-4: DEFAULT_OUTPUT_BYTE_CAP doc claims 'parses as a u64' but
  parse_output_byte_cap uses usize — silent type drift on 32-bit targets
status: Done
assignee: []
created_date: '2026-05-07 21:04'
updated_date: '2026-05-07 23:30'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:110-111` (doc) vs `:147-165` (parser)

**What**: The doc-comment on `DEFAULT_OUTPUT_BYTE_CAP` says:

> Override at runtime via \`OPS_OUTPUT_BYTE_CAP\` (parses as a u64; values \`<=0\` are ignored and fall back to the default).

But the parser `parse_output_byte_cap` and the public `output_byte_cap()` accessor both return `usize`, and the parse step is `s.parse::<usize>()`. On a 32-bit target (`usize == u32`) an operator who reads the doc and sets `OPS_OUTPUT_BYTE_CAP=8589934592` (8 GiB, valid u64) gets a parse error and silently falls back to the 4 MiB default — with the warn from TASK-0840, but the *error message* still says "failed to parse as usize" rather than "exceeds 32-bit usize on this platform; the doc claims u64 support."

Two fixes are reasonable, pick one and align the other:
1. Change the doc to say "parses as `usize`" (truthful but constrains 32-bit operators).
2. Change `parse_output_byte_cap` to parse as `u64`, then fall back to default with a clear "value exceeds usize on this platform" warning when `u64 > usize::MAX as u64`.

**Why it matters**: The asymmetry is invisible on every CI target (all 64-bit), but the doc is a contract — anyone targeting an embedded / WASM ops build hits a confusing fallback. Also the `>=DEFAULT_OUTPUT_BYTE_CAP * usize_size` peak-RSS warning is computed in `usize`, so the entire chain assumes 64-bit. If 32-bit is *not* a supported target, the doc lying about `u64` is the only fix needed.

**Severity rationale**: Low — purely a doc/contract drift, no observable production behaviour change on the supported (64-bit) targets. Filed because it's the kind of two-line drift that becomes a 3-day debug session for the first 32-bit operator.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pick one of the two fixes above and apply it consistently between parse_output_byte_cap doc and impl,Add a unit test that pins the chosen contract (either 'u64 with platform fallback' or 'usize'),Audit the same drift on parse_max_bytes for OPS_MANIFEST_MAX_BYTES (text::manifest_max_bytes already uses u64 — confirm parity)
<!-- AC:END -->
