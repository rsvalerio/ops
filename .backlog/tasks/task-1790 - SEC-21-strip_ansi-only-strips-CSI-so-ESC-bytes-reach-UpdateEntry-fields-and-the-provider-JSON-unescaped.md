---
id: TASK-1790
title: >-
  SEC-21: strip_ansi only strips CSI, so ESC bytes reach UpdateEntry fields and
  the provider JSON unescaped
status: Done
assignee:
  - TASK-1995
created_date: '2026-08-27 11:24'
updated_date: '2026-08-28 20:26'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
  - extensions-rust/cargo-update/src/tests.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:183-236` (`strip_ansi`), `:462-463` (success branch of `provide`)

**What**: `strip_ansi`'s doc comment promises to "Strip ANSI escape sequences
from a string", but the implementation only recognises one shape: `ESC [`
(CSI). Every other escape introducer falls through to `result.push(c)` at
`:232` with the raw `ESC` intact:

- **OSC** — `ESC ] … BEL` / `ESC ] … ESC \`. Cargo emits OSC-8 hyperlinks when
  `term.hyperlinks` is enabled (it is auto-detected from the terminal), so this
  is reachable in normal interactive use, not just under a hostile registry.
- **Two-character escapes** — `ESC c` (RIS, full terminal reset), `ESC (` charset selects.
- **A bare `ESC`** anywhere not followed by `[`.
- The `!terminated` branch at `:218-230` *deliberately* re-emits `ESC` `[` plus
  the buffered bytes (the TASK-1028 fix for truncated CSI) — correct for not
  losing visible text, but it means `ESC` provably survives `strip_ansi` even
  on the CSI path.

Whatever survives is then split on whitespace by `parse_action_line` and stored
verbatim into `UpdateEntry.name` / `.from` / `.to`, which `provide` serialises
straight into the provider JSON at `:463` with no escaping. That JSON is what
the about page renders to the operator's terminal.

The asymmetry is the tell: the **error** branch of the same function was
hardened for exactly this (SEC-21 / TASK-1537 at `:446-459` routes the stderr
tail through `{:?}` so ANSI/NUL/newline cannot repaint the terminal or forge a
log record), and the `tracing::warn!` breadcrumbs use `?clean` for the same
reason. The **success** branch — the path that actually runs on every
successful `ops about --refresh` — has no equivalent guard. Cargo's stderr here
is shaped by crate names, version strings and registry metadata, i.e. content a
poisoned or typosquatted crate controls.

Also note `strip_ansi` is what the parser sees, so a surviving `ESC` can change
tokenisation as well as rendering.

**Why it matters**: A01/A03-class output-encoding gap. It defeats the stated
purpose of the function (the parser is handed text the author believes is
escape-free), and it lets attacker-shaped bytes reach an operator's terminal
through the one path in this file that was never given the SEC-21 treatment
its sibling path received.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 strip_ansi handles the non-CSI escape families it claims to (at minimum OSC ESC ] terminated by BEL or ST, and two-character ESC sequences), or its doc comment is narrowed to state exactly what it strips
- [x] #2 No ESC (U+001B) or other C0 control byte can survive into UpdateEntry.name/from/to — either scrubbed in strip_ansi or rejected/escaped where the entry is built
- [x] #3 The success branch of provide gives the same control-byte guarantee the non-zero-exit branch got from SEC-21/TASK-1537
- [x] #4 Tests cover an OSC-8 hyperlink line, a bare ESC, and the existing truncated-CSI case, asserting no ESC reaches the serialized JSON
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-1995: strip_ansi now handles OSC (BEL and ST terminated) and nF/two-character escapes, each with a bounded scan. Truncated sequences and a bare ESC are still preserved (TASK-1028), so the control-byte guarantee is enforced where the entry is built: parse_action_line rejects any name/version carrying a control character and the caller warns. Tests assert no ESC/NUL reaches the serialized JSON for the OSC-8, bare-ESC and truncated-CSI shapes.
<!-- SECTION:NOTES:END -->
