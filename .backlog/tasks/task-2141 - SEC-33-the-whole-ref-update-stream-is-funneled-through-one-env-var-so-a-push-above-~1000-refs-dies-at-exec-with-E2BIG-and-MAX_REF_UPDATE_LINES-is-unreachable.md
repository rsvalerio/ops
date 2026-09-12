---
id: TASK-2141
title: 'SEC-33: the whole ref-update stream is funneled through one env var, so a push above ~1000 refs dies at exec with E2BIG and MAX_REF_UPDATE_LINES is unreachable'
status: Done
assignee: []
created_date: '2026-09-08 07:01'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2234'
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: medium
ordinal: 57000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:55` (`HOOK_SCRIPT`), `extensions/run-before-push/src/lib.rs:80` (`MAX_REF_UPDATE_LINES`)

**What**: The hook reads git's entire pre-push stdin into a single environment variable and then `exec`s through it:

```sh
OPS_PRE_PUSH_REFS=$(cat)
export OPS_PRE_PUSH_REFS
exec ops run-before-push </dev/null
```

On Linux, `execve` caps any *single* argument or environment string at `MAX_ARG_STRLEN` = 131072 bytes, independent of the much larger `ARG_MAX` (2 MiB here). A pre-push ref line is `<local ref> <local oid> <remote ref> <remote oid>` — roughly 130 bytes with SHA-1 oids and ordinary branch names, less with short names, more with SHA-256 oids or long refs. The cap is therefore reached at roughly a thousand refs.

Measured on this machine, with the script's exact shape:

```
n=800  bytes=104000 -> ok
n=1000 bytes=130000 -> ok
n=1200 bytes=156000 -> E2BIG   ("exec: Argument list too long", exit 126)
```

Two consequences:

1. **`git push --mirror`, `--tags`, or a first push of a repo with more than ~1000 refs aborts** with a bare `Argument list too long` from `/bin/sh`, naming neither ops nor the hook. Nothing in the crate's careful missing-ops diagnostic covers this path.
2. **`MAX_REF_UPDATE_LINES = 10_000` is dead** through the installed hook. The constant documents itself as the bound on "external input" for this stream, but `classify_ref_updates` can never observe more than ~1000 lines when the stream arrives via the env var — the exec fails first. The only caller that can exercise the bound is the unit test that constructs the string directly.

The stream is also the one piece of untrusted, unbounded input the hook handles, and the transport chosen for it is the one with the smallest hard limit available.

**Why it matters**: A developer pushing a mirror or a tag-heavy repo is blocked by a shell error with no attribution, and the documented input bound that was supposed to make the classifier safe on large pushes never runs. Both go away if the stream is passed on a pipe or a temp file, or if the hook truncates deliberately (and says so) rather than letting `execve` decide.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A push with more than MAX_REF_UPDATE_LINES ref updates reaches ops without an execve failure - the stream is passed by a mechanism with no per-string exec limit (pipe, fd, or temp file), or the hook bounds it explicitly before exporting
- [ ] #2 If a bound is applied in the shell, exceeding it yields PushRefs::Run (checks run) and a diagnostic, never a silent skip or a bare 'Argument list too long'
- [ ] #3 MAX_REF_UPDATE_LINES is reachable from the installed-hook path, or is removed and its rationale corrected
- [ ] #4 A test drives HOOK_SCRIPT end to end with a stream well past 131072 bytes and asserts the hook still classifies and dispatches
<!-- AC:END -->
