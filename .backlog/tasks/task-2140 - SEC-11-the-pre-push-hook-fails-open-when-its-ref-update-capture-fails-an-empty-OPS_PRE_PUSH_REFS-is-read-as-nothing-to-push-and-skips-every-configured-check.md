---
id: TASK-2140
title: >-
  SEC-11: the pre-push hook fails open when its ref-update capture fails - an
  empty OPS_PRE_PUSH_REFS is read as "nothing to push" and skips every
  configured check
status: To Do
assignee:
  - TASK-2234
created_date: '2026-09-08 07:01'
updated_date: '2026-09-08 10:53'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: high
ordinal: 56000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:55` (`HOOK_SCRIPT`), `extensions/run-before-push/src/lib.rs:127` (`classify_ref_updates`)

**What**: The installed hook captures git's ref-update stream with an unchecked command substitution and then exports the result unconditionally:

```sh
OPS_PRE_PUSH_REFS=$(cat)
export OPS_PRE_PUSH_REFS
exec ops run-before-push </dev/null
```

`$(cat)` yields the empty string on *any* capture failure, and the script neither runs under `set -e` nor inspects `$?`. `classify_ref_updates(Some(""))` maps the empty stream to `PushRefs::NothingToPush`, whose `skip_reason()` is `Some("nothing to push")` — so the dispatch path short-circuits with success and **runs none of the configured commands**, while git proceeds with the push.

The two states are conflated at the boundary: "git told us there are no ref updates" and "we failed to read what git told us" arrive as the same value. `REF_UPDATES_ENV_VAR`'s own doc draws the distinction it needs (unset = manual invocation), but only for *absent* vs *present*, never for *present but unread*.

Reproduced (a `sh` with `ops` resolvable but `cat` not, i.e. the truncated-PATH environment the `command -v ops` probe two lines above was written for):

```
$ echo "refs/heads/main 1111 refs/heads/main 2222" | PATH="$PWD/bin" /bin/sh hook.sh
hook.sh: 6: cat: not found
refs=[]
exit=0
```

The hook exits 0 having forwarded nothing, and the push is unchecked. `cat` is also the script's only external-command dependency — every other construct in it is an `sh` builtin — so it is the single point where the hook can lose the stream without saying so.

This directly contradicts the invariant the classifier documents at :124: "the classifier only ever *skips* work on input it fully understood, so a parser gap can never silently disable the gate." A capture gap does exactly that.

**Why it matters**: This is the failure direction that matters for a verification gate. A hook that wrongly *runs* costs a developer a minute; a hook that wrongly *skips* ships unverified commits to the remote while reporting success, and nothing in the output distinguishes it from a legitimate no-op push. The fail-open path is reachable in precisely the degraded-PATH environments (GUI git clients, IDE VCS panes, minimal containers) that the crate already treats as a first-class concern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The hook distinguishes a successful capture of an empty stream from a failed capture: the capture's exit status is checked (or the stream is read by ops itself rather than through a shell substitution)
- [ ] #2 A failed capture makes the hook run the configured commands (fail closed) or abort with a diagnostic naming the hook and the cause - never exit 0 having skipped the checks
- [ ] #3 classify_ref_updates cannot be reached with a value that means 'unread'; the encoding forwarded through OPS_PRE_PUSH_REFS makes 'no ref updates' and 'capture failed' distinct states
- [ ] #4 A test drives HOOK_SCRIPT with the capture made to fail (e.g. cat unavailable on PATH) and asserts the hook does not exit 0 with the checks skipped
- [ ] #5 The classifier doc at :124 is true again, or is corrected to state the boundary condition it actually guarantees
<!-- AC:END -->
