---
id: TASK-1892
title: >-
  SEC-25: the hook file itself is the one entity in the write path with no
  symlink check, so ops reports 'already installed' for a hook it does not own
status: To Do
assignee:
  - TASK-2008
created_date: '2026-08-27 15:35'
updated_date: '2026-08-28 14:17'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - extensions/hook-common/src/install.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:77-98` (`handle_existing_hook`), `extensions/hook-common/src/paths.rs:23-39`

**What**: The installer's stated defence is that nothing on the write path may be a symlink. `canonical_git_dir` refuses a `.git` that is not a real `.git` directory, `canonical_subdir` refuses a symlinked `hooks/` (`paths.rs:28-30`), `looks_like_git_dir` refuses a symlinked `HEAD` (`paths.rs:72-75`), and `find_git_dir` refuses a symlinked `.git` entry (`git.rs:56`). Each has a test. The final component — `.git/hooks/<hook_filename>` — has none.

`OpenOptions::create_new(true)` returns `AlreadyExists` for a symlink (including a dangling one), so control reaches `handle_existing_hook`, which does:

```rust
let existing = std::fs::read_to_string(hook_path).context("failed to read existing hook")?;
if existing == config.hook_script {
    writeln!(w, "Hook already installed at {}", hook_path.display())?;
```

`read_to_string` **follows** the link, so the decision is made on the content of a file somewhere else entirely, while every subsequent write targets the link path. Two divergent outcomes follow:

1. Target content equals the ops script -> ops prints *"Hook already installed"* and exits 0 for a hook whose real body lives outside the repository and can be changed by whoever owns that file. Reinstalling never corrects this, because the idempotency branch returns early every time.
2. Target content carries a legacy ops marker -> `upgrade_legacy_hook` stages and `persist`s, and `rename(2)` does **not** follow symlinks, so the link is silently replaced by a regular file. An operator who deliberately symlinked `pre-commit` into a shared hooks directory loses that wiring with no diagnostic — the output says only "Updating outdated ops hook".

**Why it matters**: this is not a privilege boundary (writing the symlink requires write access to `.git/hooks` already), which is why it is filed low — but it is a hole in a defence the crate documents as complete, and case 1 is a correctness bug on its own: the installer reports a hook is in place when the file git will actually execute is one ops neither wrote nor can vouch for. A `symlink_metadata` probe on `hook_path` before the existing-hook branch, refusing with the same "is a symlink" wording `canonical_subdir` already uses, closes both.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 install_hook rejects a symlinked <git_dir>/hooks/<hook_filename> with a clear error, consistent with the existing symlink refusals for .git, hooks/ and HEAD
- [ ] #2 A #[cfg(unix)] test symlinks pre-commit at a file containing the exact ops hook script and asserts install_hook errors instead of reporting 'Hook already installed'
- [ ] #3 A #[cfg(unix)] test symlinks pre-commit at a file containing a legacy ops marker and asserts the symlink is not silently replaced by a regular file
<!-- AC:END -->
