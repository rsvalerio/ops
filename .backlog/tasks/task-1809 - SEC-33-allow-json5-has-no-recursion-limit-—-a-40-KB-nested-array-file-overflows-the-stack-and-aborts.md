---
id: TASK-1809
title: >-
  SEC-33: --allow-json5 has no recursion limit — a 40 KB nested-array file
  overflows the stack and aborts
status: Done
assignee:
  - TASK-2004
created_date: '2026-08-27 11:31'
updated_date: '2026-08-28 22:24'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/config-checkers/src/json.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/json.rs:15-26` (`check_json`, the `allow_json5` branch at lines 16-20)

**What**: the two branches of `check_json` have asymmetric DoS protection, and only the strict one is safe.

- Strict (`serde_json::from_slice::<Value>`) enforces `RECURSION_LIMIT = 128` and returns a normal error.
- Lenient (`json5::from_str::<serde_json::Value>`, json5 0.4.1) has no depth bound at all — it is a `pest` recursive-descent grammar, so nesting depth maps one-to-one onto native stack frames.

Verified against the pinned versions with a standalone probe on `"[" * n + "]" * n`:

```
serde_json nesting 200: ok=false err=Some("recursion limit exceeded at line 1 column 128")
json5 nesting  2000: ok=true
json5 nesting 20000, bytes 40000 -> thread 'main' has overflowed its stack
                                    fatal runtime error: stack overflow, aborting   (exit 134)
```

**40 KB** of `[[[[...]]]]` in a `.json` file kills the process. `DEFAULT_MAX_BYTES` is 16 MiB, i.e. ~400x more headroom than needed, so the size gate never engages. A stack overflow is a `SIGABRT`, not a `CheckError` — the checker cannot report the file, and the report/exit-code contract is bypassed entirely.

The blast radius is smaller than the YAML case only because it needs `--allow-json5`; the flag is a documented, first-class CLI option (`crates/cli/src/args.rs:185-187`) intended for repos that use JSONC-style config, which is exactly the population that would enable it repo-wide in CI.

**Why it matters**: SEC-33 — no nesting-depth cap when deserializing untrusted input. The failure mode (native stack exhaustion) is strictly worse than the strict branch's, because it is unrecoverable in-process: no `catch_unwind`, no error variant, no way for the surrounding `run_checker` loop to continue to the next file.

**Fix shape**: json5 0.4.1 exposes no depth knob, so the bound has to be imposed before or around the parse. Cheapest correct option: pre-scan the byte slice for maximum bracket/brace nesting depth and reject past a limit (mirror serde_json's 128 for consistency between the two modes) before calling `json5::from_str`. That is O(n) over bytes already in memory and keeps the two branches behaving the same way on the same input. Alternatives: parse the json5 branch on a dedicated thread with a large, explicitly sized stack plus a depth cap (still needs the cap — it only moves the cliff), or move off json5 0.4.1 to a parser that bounds recursion itself.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The allow_json5 path enforces a maximum nesting depth before or during the parse, so deeply nested input returns CheckError::Parse instead of overflowing the stack
- [x] #2 The depth limit is consistent with the strict branch's effective limit, and both branches are documented as bounded
- [x] #3 A regression test feeds a deeply nested (>= 20000 levels) array to check_json with allow_json5 = true and asserts an Err is returned
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-2004.

`check_json` runs an iterative, byte-level `exceeds_depth_limit` pre-scan
before either parser sees the input, so the bound applies to both branches
(`json::MAX_NESTING_DEPTH = 128`, matching serde_json's `RECURSION_LIMIT`).
The scanner skips string literals (both quote styles) and JSON5 comments so a
bracket inside a value or a comment does not count; it can only ever lower the
measured depth, never invent one. Over the limit yields
`CheckError::Parse(LimitExceeded)` — "input exceeds the nesting depth limit of 128".

AC#2: the limit is now explicit on both branches rather than inherited from
`Value`'s recursion limit, and is documented on the constant and in the module
docs. AC#3: `json::tests::deeply_nested_input_is_rejected_instead_of_overflowing_the_stack`
feeds 20 000 levels through both `allow_json5 = true` and the strict branch;
`nesting_at_the_limit_is_accepted` pins the boundary from the other side.
<!-- SECTION:NOTES:END -->
