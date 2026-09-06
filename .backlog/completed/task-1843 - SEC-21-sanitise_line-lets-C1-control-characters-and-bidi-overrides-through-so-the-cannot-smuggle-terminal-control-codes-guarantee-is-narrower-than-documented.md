---
id: TASK-1843
title: >-
  SEC-21: sanitise_line lets C1 control characters and bidi overrides through,
  so the 'cannot smuggle terminal control codes' guarantee is narrower than
  documented
status: Done
assignee:
  - TASK-1984
created_date: '2026-08-27 15:23'
updated_date: '2026-08-29 00:36'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/src/ui.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/ui.rs:30-40` (`sanitise_line`)

**What**: The docstring states the guarantee as:

> Escapes ESC (`\x1b`) and any non-`\t` control character (`< 0x20`, `\x7f`) using `\xNN` so an attacker-controlled error message **cannot smuggle ANSI escapes or terminal control codes** into operator-facing output.

The implementation covers C0, DEL, and ESC — and nothing above `0x7f`:

```rust
pub fn sanitise_line(line: &str, out: &mut String) {
    for ch in line.chars() {
        match ch {
            '\t' => out.push('\t'),
            c if u32::from(c) < 0x20 || c == '\x7f' || c == '\u{1b}' => {
                let _ = write!(out, "\\x{:02x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
}
```

Two classes fall through the final `c => out.push(c)` arm verbatim:

1. **C1 controls, `U+0080..U+009F`.** `U+009B` is CSI — the single-character form of `ESC [` — and `U+009D` is OSC. Escaping `ESC` while passing CSI through leaves the shorter spelling of the same primitive open.
2. **Unicode bidi overrides**, `U+202A..U+202E` and `U+2066..U+2069` — the Trojan-Source display-spoofing vector, which reorders rendered text without changing the bytes. Under the exact threat model this function names (a hostile `.ops.toml` value shown in a `--dry-run` audit preview), a reordering override is arguably worse than a colour change, because the operator reads a command line that is not the one that will run.

**Reachability is the whole point of this helper**, per its own docs: it is the shared defence for `ops --dry-run` (SEC-21 / TASK-1184), which prints env-expanded `program` / `args` / `env` / `cwd` from a repo-supplied `.ops.toml`; for `AboutCard` values (`crates/core/src/project_identity/card.rs:288`, `sanitised()`), which carry `Cargo.toml` / `package.json` descriptions from a third-party repo; and for every `ui::warn` / `ui::error` line.

**Why it matters**: `ops` is designed to run inside repositories it does not control, and the dry-run preview exists so an operator can *audit* a config before executing it. A defence whose stated invariant is "cannot smuggle terminal control codes" but which passes CSI and bidi overrides gives that audit a false floor — reviewers reading the docstring have no reason to add their own escaping.

Note the C1-vs-terminal question deserves a deliberate answer rather than an assumed one: whether a given emulator acts on C1 delivered as UTF-8 varies (xterm gates it behind a resource; some VTE-family terminals historically did). The fix is the same either way and costs one range: extend the guard to `(0x80..=0x9f)`, and decide explicitly whether bidi controls are escaped or stripped. Whichever is chosen, the docstring should state the character classes it neutralises rather than the informal "terminal control codes".

<!-- scan confidence: verified by reading ui.rs:30-40; the match has no arm above 0x7f -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 sanitise_line escapes the C1 range U+0080..U+009F, so U+009B (CSI) and U+009D (OSC) cannot reach the terminal unescaped
- [x] #2 A deliberate decision is recorded for Unicode bidi controls (U+202A-U+202E, U+2066-U+2069) — either neutralised or documented as out of scope with the reason
- [x] #3 The docstring states the exact character classes neutralised instead of the informal 'terminal control codes' claim
- [x] #4 Tests cover U+009B and a bidi override reaching sanitise_line, asserting the rendered output for each
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in wave TASK-1984. `sanitise_line` now escapes the C1 range U+0080..=U+009F as \xNN (so CSI U+009B and OSC U+009D cannot reach the terminal unescaped) and neutralises the Unicode bidi reordering controls U+202A..=U+202E and U+2066..=U+2069 as \u{NNNN}. Bidi is escaped rather than stripped so an operator can see the input contained them; bidi *marks* (U+200E/U+200F) and general confusable filtering are documented as out of scope. The docstring now carries a table of the exact character classes neutralised instead of the informal "terminal control codes" claim, plus the recorded rationale for both decisions. Tests: c1_control_characters_are_escaped, bidi_controls_are_escaped, characters_adjacent_to_the_escaped_ranges_pass_through.
<!-- SECTION:NOTES:END -->
