---
id: TASK-1911
title: >-
  CL-3: HOOK_SCRIPT silently assumes bash and ops are both on the hook's PATH;
  when either is missing every push is blocked with a bare 'command not found'
status: To Do
assignee:
  - TASK-2010
created_date: '2026-08-27 15:40'
updated_date: '2026-08-28 14:17'
labels:
  - code-review-rust
  - cognitive-load
dependencies: []
modified_files:
  - extensions/run-before-push/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-push/src/lib.rs:35` (`HOOK_SCRIPT`)

**What**:

    const HOOK_SCRIPT: &str = "#!/usr/bin/env bash\nexec ops run-before-push\n";

Two preconditions are implicit and unchecked, and both fail *closed on the push* with no actionable message.

1. **`bash` must resolve on the hook's PATH.** The script body is one `exec` — pure POSIX, using nothing bash provides. `#!/usr/bin/env bash` nonetheless makes bash a hard runtime dependency. On Alpine/busybox images, minimal CI containers, and NixOS shells without bash in scope, git reports `hook ... cannot be run` (exit 126/127) and refuses the push. `#!/bin/sh` costs nothing here and removes the dependency entirely.

2. **`ops` must resolve on the hook's PATH.** `ops <hook> install` runs from the developer's interactive shell, where `ops` is on PATH — that is the only environment in which the precondition is ever observed to hold. git hooks fired from GUI clients (SourceTree, Tower, GitKraken, the VS Code / JetBrains SCM panes, and macOS LaunchServices-launched apps generally) inherit a login-less, truncated PATH that routinely lacks `~/.cargo/bin` and `/usr/local/bin`. The user then sees `.git/hooks/pre-push: line 2: ops: command not found`, exit 127, push aborted — with nothing naming ops, the hook, or the fix. The failure looks like a git bug, and the usual user response is `git push --no-verify`, which disables the gate permanently.

Note the skip path does not help: `SKIP_OPS_RUN_BEFORE_PUSH` is read by `ops` itself (`ops_hook_common::should_skip`), so it is never consulted when the problem is that `ops` could not be launched.

**Why it matters**: CL-3 — the hook rests on undocumented environment assumptions rather than making them explicit. The blocking behaviour itself is correct (a hook that cannot verify must not report success), but the diagnostic is not: the script should say which binary it could not find, name the hook, and point at the fix, so the operator resolves PATH instead of reaching for `--no-verify`. This applies identically to `extensions/run-before-commit/src/lib.rs:44`, which carries the same two-line script; fix them together if the constant is ever lifted into `ops-hook-common`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 HOOK_SCRIPT no longer requires bash: the shebang is #!/bin/sh (or the script genuinely needs bash and the reason is written next to the constant)
- [ ] #2 The script checks that ops resolves before exec'ing it, and on failure prints a message to stderr naming the ops binary, the pre-push hook path, and how to fix PATH or bypass with SKIP_OPS_RUN_BEFORE_PUSH
- [ ] #3 The failure path still exits non-zero so git aborts the push — the hook must not fail open
- [ ] #4 A test asserts the shebang and the ops-missing guard are present in HOOK_SCRIPT, and that the script is valid under sh (e.g. sh -n)
<!-- AC:END -->
