---
id: TASK-1840
title: >-
  ERR-1: partial cargo-deny decode loss is invisible — the exit-1 guard only
  catches total decode failure, so a per-code schema change silently drops whole
  diagnostic classes
status: Done
assignee:
  - TASK-1997
created_date: '2026-08-27 15:23'
updated_date: '2026-08-28 20:33'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/deps/src/parse/deny.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/deny.rs:173-208` (`decode_diagnostic`), `:241-263` (`parse_deny_output`), `:99-110` (the exit-1 zero-diagnostics guard)

**What**: `parse_deny_output` drops undecodable diagnostics one line at a time and returns only what survived — it never reports how many lines it threw away. There are three independent drop paths:

1. `decode_diagnostic` → `serde_json::from_str` fails: logged at **debug** (`ERR-1: skipping malformed cargo-deny JSON line`).
2. `decode_diagnostic` → `let code = fields.code?` (`:189`): the diagnostic has no `code` field. **Nothing is logged at all** — this is the only drop path in the whole crate with no tracing breadcrumb. Pinned by `deny/tests.rs::parse_deny_no_code_field_skipped`.
3. `parse_deny_output` → `classify_code` returns `None`: logged at **debug** (`TASK-0436: … unknown code (possible schema drift)`).

The only guard above them is `interpret_deny_result`'s exit-1 check (`:99-110`), which bails **only when all four vectors are empty**:

```rust
if parsed.advisories.is_empty()
    && parsed.licenses.is_empty()
    && parsed.bans.is_empty()
    && parsed.sources.is_empty()
{ anyhow::bail!("… stderr decoded zero diagnostics …"); }
```

So the gate is protected against *total* decode failure (TASK-0958) and against *zero* diagnostics (TASK-0612) — but a **partial** loss passes straight through. cargo-deny's diagnostic stream is heterogeneous: advisories, licenses, bans, and sources are emitted by four different check implementations and carry different field shapes. A change that affects one of them — a renamed `code` value, `code` moved under a nested object, a new advisory category — drops exactly that class while the other three keep decoding, the `is_empty()` conjunction stays false, and `interpret_deny_result` returns `Ok`.

The worst concrete case: cargo-deny renames or restructures the advisory codes (`vulnerability` / `unmaintained` / `unsound` / `yanked` — `classify_code` at `:26-36` matches them as exact strings). Every advisory now falls into drop path 3, `result.advisories` is empty, but a single unrelated `duplicate` ban still decodes. `interpret_deny_result` returns `Ok`, `has_issues` sees no advisories, `ops deps` renders **"Advisories: None"** in green and exits 0 — while an unpatched RUSTSEC vulnerability is sitting in the tree. The only trace is a `tracing::debug!` that `ops deps` does not surface to the operator.

The parser already tracks exactly the counter needed to close this. `parse/upgrade.rs` solved the same problem with `UpgradeParseDiagnostics { body_lines, entries_emitted }` and `check_row_shape_drift` — "we saw N candidate rows and emitted zero entries" bails. `parse_deny_output` needs the equivalent: count candidate diagnostic lines (`type == "diagnostic"`) versus entries pushed, and let `interpret_deny_result` fail closed when the ratio drifts, rather than only when the total hits zero.

**Why it matters**: this crate's entire reason to exist is being authoritative about supply-chain findings, and every other fail-open path in it has already been hardened (TASK-0386 exit 2, TASK-0598 signal kill, TASK-0612 empty stderr, TASK-0958 text-mode stderr). This is the remaining hole and it is the most likely one to be hit, because it does not require cargo-deny to change its output *format* — only one field or one code string in one check. A dropped advisory is indistinguishable from a clean advisory section.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 parse_deny_output tracks how many lines were candidate diagnostics versus how many became entries, mirroring UpgradeParseDiagnostics in parse/upgrade.rs
- [x] #2 interpret_deny_result fails closed when diagnostics were seen but a non-trivial share of them could not be decoded or classified, not only when the decoded total is zero
- [x] #3 The bail message distinguishes partial decode loss from the existing zero-diagnostics (TASK-0958) and empty-stderr (TASK-0612) cases and reports the seen/decoded counts
- [x] #4 decode_diagnostic logs a tracing breadcrumb when it drops a diagnostic for a missing code field, matching the other two drop paths
- [x] #5 A test drives interpret_deny_result(Some(1), <stderr with one decodable ban plus several advisory lines carrying an unrecognised code>) and asserts it errs instead of returning a report with an empty advisories section
- [x] #6 parse_deny_no_code_field_skipped and the other existing deny/tests.rs cases still pass, updated only where they pinned the silent-drop behaviour
<!-- AC:END -->
