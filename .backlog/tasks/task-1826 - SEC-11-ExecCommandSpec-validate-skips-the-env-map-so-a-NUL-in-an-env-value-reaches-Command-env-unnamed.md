---
id: TASK-1826
title: >-
  SEC-11: ExecCommandSpec::validate skips the env map, so a NUL in an env value
  reaches Command::env unnamed
status: Done
assignee:
  - TASK-1983
created_date: '2026-08-27 11:33'
updated_date: '2026-08-28 23:52'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/core/src/config/commands.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/commands.rs:202-227`

**What**: `ExecCommandSpec::validate` runs `check_control_chars` over `program`, every element of `args`, and `cwd`. It never touches `env: HashMap<String, String>` (commands.rs:126) — neither keys nor values.

The stated purpose of the check (commands.rs:188-192) is:

> rejects NUL and other control characters (`< 0x20` except `\t`) … so a bad config fails at load with a named error instead of a cryptic `EINVAL` at spawn time.

But `crates/runner/src/command/build.rs:621-624` passes `spec.env` straight into `Command::env`, so a NUL in an env value surfaces as std's generic `InvalidInput` — "nul byte found in provided data" — with no command name, no field name, and no offending key. That is precisely the cryptic-spawn-failure outcome the validator was written to prevent, on the one field it does not cover.

Keys have a second problem values do not: an env key containing `=` or a newline is also unvalidated, and on Unix an `=` in a key produces an entry the child's `getenv` can never retrieve.

**Why it matters**: `.ops.toml` is repo-supplied content, so this is the same SEC-11 trust boundary the rest of `validate` guards. The gap is asymmetric in a way that misleads: a reviewer reading `validate` reasonably concludes every string that reaches `Command` is screened, and the `env` field is the only one that is not.

<!-- scan confidence: verified — validate's body at commands.rs:202-227 contains no reference to self.env -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 validate iterates self.env and applies check_control_chars to every value, with the field name rendered as env[<key>] so the error names the offending variable
- [x] #2 validate rejects an env key containing '=' or any control character, with an error naming the command and the key
- [x] #3 Tests cover a NUL in an env value and an '=' in an env key, asserting the error message contains both the command name and the key
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added `ExecCommandSpec::validate_env`, called from `validate`. Applies `check_control_chars` to every env value (field rendered as `env[<key>]`) and to every key (`env key "<key>"`), and rejects `=` in a key. Keys are visited in sorted order so the diagnostic is deterministic despite `HashMap`. Tests: `exec_spec_validate_rejects_nul_in_env_value`, `..._rejects_control_char_in_env_key`, `..._rejects_equals_in_env_key`, `..._accepts_plain_env`.
<!-- SECTION:NOTES:END -->
