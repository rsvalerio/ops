---
id: TASK-1871
title: >-
  SEC-21: config.rs logs two attacker-controlled values with the Display
  formatter, contradicting the ERR-7 / TASK-1206 policy it applies three
  functions earlier
status: Triage
assignee: []
created_date: '2026-08-27 15:31'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/git/src/config.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:126-130` (`read_head_branch`), `extensions/git/src/config.rs:400-409` (`is_origin_header`)

**What**: `read_origin_url` deliberately logs its path with the Debug formatter and says why, three times:

```
// ERR-7 / TASK-1206: Debug-format the path so a hostile checkout
// path with newlines / ANSI cannot forge log records.
tracing::warn!(path = ?path.display(), …);
```

Two other call sites in the same file break that policy:

1. `read_head_branch` (`config.rs:126`) logs `path = %head_path.display()` — the Display formatter — on the non-`NotFound` IO error arm. This is the *same* `.git`-derived path, at **warn** level, which is on in the default log configuration. A checkout directory containing `\n` or an ANSI escape forges log records or repaints the operator terminal, which is exactly what TASK-1206 closed for the config reader.
2. `is_origin_header` (`config.rs:404-409`) logs `line` — the raw `.git/config` section-header line, verbatim, via the field shorthand (Display) — whenever a header that starts with `r`/`R` fails to parse. That string is fully attacker-controlled and, unlike a `url = …` value, never passes through `RedactedUrl::redact`, so ANSI escapes and interior `\r` reach the log sink unescaped. Debug level, so lower exposure, but the value is strictly more hostile than the path.

**Why it matters**: log-record forging and terminal-escape injection from a repository checkout. The crate already decided this is a real risk and wrote the mitigation into the file; leaving two call sites on `%` means the guarantee holds only for the paths someone happened to audit. The fix is mechanical (`%` → `?`) and makes the file internally consistent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_head_branch logs the HEAD path with the Debug formatter (path = ?head_path.display()), matching read_origin_url
- [ ] #2 is_origin_header logs the rejected header line with the Debug formatter so control bytes are escaped
- [ ] #3 a test pins the escaping contract for both values, in the style of read_origin_url_path_debug_escapes_control_characters
- [ ] #4 a grep for tracing macros in extensions/git shows no remaining Display-formatted path or raw-config-line field
<!-- AC:END -->
