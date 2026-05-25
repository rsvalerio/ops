---
id: TASK-1321
title: >-
  READ-5: run_theme_select writes happy-path output via std::io::stdout(),
  breaking the _to testability pattern
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-11 20:55'
updated_date: '2026-05-12 23:23'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/theme_cmd.rs:168-180`

**What**: `run_theme_select_with_tty_check` writes its two success messages (`"Theme already set to '{name}'"` and `"Theme set to '{name}'"`) directly via `writeln!(std::io::stdout(), ...)` rather than threading a `&mut dyn Write` from a callable `_to` variant. Every other rendered handler in the cli crate (`run_theme_list` → `run_theme_list_to`, `run_extension_list` → `run_extension_list_to`, `run_tools_list` → `run_tools_list_to`, `run_tools_check` → `run_tools_check_to`, `run_init` → `run_init_to`) routes the output through an injectable writer so the happy-path text is unit-testable. theme-select is the lone holdout, and the rustdoc on `run_theme_select` (lines 124-133) explicitly calls this out as a "Testing Limitation (TQ-017)".

**Why it matters**: Two consequences:
1. The "already set" / "set to" happy-path messages have no unit-test coverage at all. The TTY-required path is the obvious blocker for the *interactive picker*, but the post-selection output formatting is not interactive — it's a deterministic format-and-write. A future refactor that drops the trailing single-quote, swaps the message order, or regresses on the "no-op when already set" short-circuit would not break any test.
2. The asymmetry signals to future contributors that direct-to-stdout is acceptable in this module, which is exactly what `_to` variants exist to prevent.

The fix is the same shape that `run_theme_list` already follows: split into `run_theme_select_with_tty_check<F, W: Write>(... w: &mut W)` and let the public entry pass `&mut std::io::stdout()`. The inquire prompt itself can stay where it is — only the post-prompt writelns need the seam.

<!-- scan confidence: candidates to inspect -->
candidates: crates/cli/src/theme_cmd.rs:169-180 (the two writeln! sites).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a  variant (or equivalent injectable writer parameter) so the post-prompt success messages can be unit-tested without spinning up a TTY
- [ ] #2 New unit test asserts the 'Theme already set to ...' and 'Theme set to ...' message text against a Vec<u8> buffer
- [ ] #3 Public entry  still writes to stdout in production
<!-- AC:END -->
