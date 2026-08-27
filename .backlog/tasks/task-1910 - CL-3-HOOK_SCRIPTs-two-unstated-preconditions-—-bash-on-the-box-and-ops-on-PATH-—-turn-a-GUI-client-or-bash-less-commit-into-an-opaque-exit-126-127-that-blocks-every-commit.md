---
id: TASK-1910
title: >-
  CL-3: HOOK_SCRIPT's two unstated preconditions — bash on the box and ops on
  PATH — turn a GUI-client or bash-less commit into an opaque exit 126/127 that
  blocks every commit
status: Triage
assignee: []
created_date: '2026-08-27 15:40'
labels:
  - code-review-rust
  - cognitive-load
dependencies: []
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:44`

**What**:

```rust
const HOOK_SCRIPT: &str = "#!/usr/bin/env bash\nexec ops run-before-commit\n";
```

Two environmental preconditions are baked in, neither documented nor checked at install time:

1. **`bash` must exist and be on PATH.** The script body contains nothing bash-specific — it is a single `exec`. `#!/bin/sh` would run everywhere; `#!/usr/bin/env bash` fails on Alpine/busybox images, minimal container builds, and BSD/Nix setups where bash is not installed. `env` then exits **127**, git reports `hook failed`, and every commit is blocked with a message that names bash, not ops. The legacy hooks this crate is written to recognise and replace all use `#!/bin/sh` (see the fixtures at lines 135, 157).

2. **`ops` must be on the PATH *of the process that invokes git*.** Git hooks inherit the environment of the caller, and GUI clients (IDE VCS panes, GitHub Desktop, SourceTree, Fork) are launched from the desktop session, not the login shell — so `~/.cargo/bin` and `~/.local/bin` are routinely absent. The hook then dies with `ops: command not found`, exit **127**, and git aborts the commit. The user's only clue is a 127 in a dialog box. `install_hook` already knows where the running binary is (`std::env::current_exe()`), so an absolute path — or a `command -v ops || exit 0`-style guard with an explicit diagnostic — is available at install time.

Both failures are indistinguishable, from git's point of view, from "your checks failed": non-zero exit, commit refused. Because this is the pre-commit path, the blast radius is every commit on that machine until the user finds and deletes `.git/hooks/pre-commit` by hand.

**Why it matters**: CL-3 — preconditions must be explicit (types, asserts, guard clauses), not relied on as undocumented invariants. Here the invariants are ambient properties of a *future* process environment that the installing process cannot observe, and the failure mode is a hard block on the developer's primary workflow with an error that does not name the tool responsible. The bash requirement in particular buys nothing: it is a gratuitous portability constraint on a one-line script.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 HOOK_SCRIPT uses #!/bin/sh, or a comment states the specific bash feature that requires it
- [ ] #2 The installed hook resolves ops robustly (absolute path from current_exe at install time, or a PATH fallback) rather than depending on the invoking process's PATH
- [ ] #3 If ops cannot be found at hook run time, the hook emits a diagnostic naming ops and the reinstall command on stderr before exiting, instead of surfacing a bare 127
- [ ] #4 A test asserts the shebang and the resolution strategy so a future edit to the one-line script cannot silently reintroduce either assumption
<!-- AC:END -->
