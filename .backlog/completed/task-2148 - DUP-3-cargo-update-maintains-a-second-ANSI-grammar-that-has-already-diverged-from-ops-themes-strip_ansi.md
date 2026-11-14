---
id: TASK-2148
title: 'DUP-3: cargo-update maintains a second ANSI grammar that has already diverged from ops-theme''s strip_ansi'
status: Done
assignee: []
created_date: '2026-09-08 07:03'
updated_date: '2026-09-09 18:58'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2242'
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
  - crates/theme/src/style/strip.rs
priority: medium
ordinal: 61000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:257`-`408` (`strip_ansi`, `consume_escape`, `consume_csi`, `consume_osc`, `consume_nf`, `CSI_SCAN_CAP`, `OSC_SCAN_CAP`, `EscapeScan`)

**What**: The crate carries a private ~150-line ANSI escape-stripping grammar. The workspace already exposes one as public API: `ops_theme::strip_ansi` (`crates/theme/src/lib.rs:30`, implemented in `crates/theme/src/style/strip.rs` behind the single `AnsiPieces` iterator that `visible_width` / `truncate_to_width` also consume — itself consolidated under DUP-1 / TASK-0978 precisely so the grammar lives in one place).

The two copies have already diverged. `ops-theme` handles:

- 8-bit C1 introducers `U+009B` (CSI), `U+009D` (OSC), `U+0090` (DCS), `U+0098` (SOS), `U+009E` (PM), `U+009F` (APC), plus `U+009C` (ST) as a terminator (SEC-11 / TASK-1967);
- the `ESC P` / `ESC X` / `ESC ^` / `ESC _` string-introducer families;
- bare C0/C1/DEL control characters and tab normalisation (CL-3 / TASK-2019).

The cargo-update copy handles only `ESC [` (CSI), `ESC ]` (OSC) and the nF two-character forms. Every other escape family reaches the field validator as raw bytes.

The divergence also runs the other way: cargo-update's copy deliberately *preserves* truncated/unterminated escapes (PATTERN-1 / TASK-1028) and bounds each scan (`CSI_SCAN_CAP` / `OSC_SCAN_CAP`), behaviour `ops-theme` does not have, so this is a reconciliation rather than a drop-in substitution.

**Why it matters**: Two independently-maintained parsers for the same grammar in one workspace is the DUP-3 failure mode, and here it is not hypothetical — this copy has needed three separate patches (ERR-1 / TASK-0882 non-ASCII corruption, SEC-21 / TASK-1790 OSC and two-character escapes, PATTERN-1 / TASK-1028 unbounded scan) for classes the `ops-theme` grammar had already covered or has since covered independently. The next hardening of either copy will not reach the other. The current gap is contained only because `is_control_free` rejects any field still carrying a control codepoint (so an unstripped `U+009B` sequence produces a rejected line and a warn rather than a poisoned entry) — i.e. correctness today rests on the validator, not on the stripper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The ANSI-stripping grammar exists in exactly one place in the workspace; cargo-update does not define its own escape-consuming functions
- [x] #2 The surviving implementation covers every family both copies covered: ESC-prefixed CSI/OSC/nF, the DCS/SOS/PM/APC string introducers, and the 8-bit C1 equivalents
- [x] #3 The truncated/unterminated-escape preservation and per-scan bounds that TASK-1028 installed are preserved (or their loss is justified in the task) rather than silently dropped by the switch
- [x] #4 The existing cargo-update strip_ansi tests (OSC-8, two-character escapes, truncated CSI/OSC, non-ASCII round-trip, the proptest identity and complete-CSI properties) still pass against the shared implementation

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Reconciled into ops-theme: AnsiPieces now yields Escape/Raw/Char pieces with bounded scans (CSI 64 / string 1024); new ops_theme::strip_ansi_preserving_raw is the policy cargo-update consumes (truncated/stray escapes and controls preserved verbatim per TASK-1028). Grammar deltas vs the old cargo-update copy, all deliberate: complete 8-bit C1 sequences are now stripped instead of reaching the validator raw (SEC-11 stance, AC#2); ESC+space is never treated as an nF intermediate (TASK-1790 protection kept); an ESC inside an OSC body now ends the sequence (theme stance) instead of continuing the scan.
<!-- SECTION:NOTES:END -->
