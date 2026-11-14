---
id: TASK-2129
title: 'SEC-33: existing git hook is slurped unbounded as UTF-8, so a binary or non-UTF-8 hook fails install with an opaque error instead of being refused as foreign'
status: Done
assignee: []
created_date: '2026-09-08 06:55'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2234'
modified_files:
  - extensions/hook-common/src/install.rs
priority: medium
ordinal: 45000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:160` (and the re-read at `:253`)

**What**: `handle_existing_hook` classifies an already-present
`<git_dir>/hooks/<name>` with

```rust
let existing = std::fs::read_to_string(hook_path).context("failed to read existing hook")?;
```

Two problems come out of that single call:

1. **No byte cap.** The file is attacker-or-operator-controlled content on a
   path ops does not own. Every other untrusted read in this crate is capped —
   `git::read_capped_to_string` reads the `gitdir` back-reference through a
   64 KiB `take()` precisely because "a hostile (or device-backed) file must
   not be slurped into memory unbounded". The hook file gets no such
   treatment, so a multi-gigabyte or `/dev/zero`-backed `pre-commit` is read
   whole into a `String` before any classification happens.
2. **Non-UTF-8 is a hard error, not a `Foreign` classification.** A hook that
   is a compiled binary, or a shell script saved in latin-1, makes
   `read_to_string` fail with `ErrorKind::InvalidData`. The user then sees
   `failed to read existing hook: stream did not contain valid UTF-8` instead
   of the actionable message `handle_existing_hook` has for exactly this case
   ("a pre-commit hook already exists at ... and was not installed by ops ...
   Remove it manually or back it up"). A non-UTF-8 hook is by definition not
   one ops wrote (`hook_script` is a `&'static str`), so `ExistingHook::Foreign`
   is the correct and safe answer.

`classify_existing_hook` only ever needs a prefix of the file: `Current` is a
whole-script equality, `Partial` is `hook_script.starts_with(content)`, and
`Legacy` is a line scan for markers that live in the script's first lines. It
never needs more bytes than `hook_script.len()` plus a small margin to prove
the content is longer.

**Why it matters**: the fix is fail-closed in both directions — a hook that is
too large or not UTF-8 can only be foreign, and refusing it with the existing
"not installed by ops" message is both safer and more useful than an
`InvalidData` string with no path in it. Today the same input either burns
unbounded memory or wedges `ops <hook>-install` with an error the operator
cannot act on. Bounded impact (the caller already has write access to
`.git/hooks`), which is why this is medium and not high.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reading the existing hook for classification is bounded by an explicit byte cap derived from (or at least commented against) config.hook_script.len(), in the spirit of git::read_capped_to_string
- [ ] #2 A non-UTF-8 existing hook is classified ExistingHook::Foreign and refused with the 'not installed by ops' message, not surfaced as a read error
- [ ] #3 An over-cap existing hook is likewise classified Foreign rather than read whole
- [ ] #4 Tests cover: a hook containing invalid UTF-8 bytes is refused as foreign and left byte-for-byte intact; an over-cap hook is refused as foreign and left intact
<!-- AC:END -->
