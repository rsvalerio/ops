---
id: TASK-1813
title: >-
  ERR-2: FailedFile collapses I/O errors into parse failures — a
  deleted-but-tracked file fails the hook
status: Triage
assignee: []
created_date: '2026-08-27 11:32'
labels:
  - code-review-rust
  - error-handling
dependencies: []
modified_files:
  - extensions/config-checkers/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/config-checkers/src/lib.rs:142-158` (`FailedFile`, `CheckerReport`) and `lib.rs:257-293` (the three sites that populate `files_failed`)

**What**: `FailedFile { path, message: String }` has no discriminator, so three semantically distinct outcomes are recorded identically and become indistinguishable to every caller:

- `metadata:` failed (the file is not there / not readable) — `lib.rs:257-267`
- `read:` failed (I/O error) — `lib.rs:271-283`
- the parser rejected the content — `lib.rs:285-293`

Only the third is a *check* failure. The CLI (`crates/cli/src/subcommands.rs:355-361`) maps `report.failed()` straight onto `ExitCode::FAILURE` under a doc comment promising it "mirrors the `pre-commit-hooks` contract", where a non-zero exit means *a file did not parse*.

Concrete false positive, verified in a scratch repo: `git ls-files` reports index entries, not worktree contents.

```
$ rm ok.json          # deleted, not staged
$ git ls-files
evil.json
ok.json               # still listed
```

`ops check-json --tracked` then stats a path that does not exist, and the run reports `check-json: ok.json: metadata: No such file or directory`, counts it in `files_failed`, and exits non-zero — a "your JSON is broken" verdict on a file that is not there. Same for sparse-checkout and `skip-worktree` entries, which `git ls-files` also lists by design. `files_scanned` is incremented for these too (`lib.rs:258`, `lib.rs:273`), which contradicts the field's own doc comment on `files_skipped` distinguishing "validated and OK" from "not validated at all" — a file whose metadata call failed was not scanned in any sense.

The crate already defines a proper domain error (`CheckError`, `lib.rs:74-102`) for the parse case; the report type then throws that typing away by rendering everything to `String`.

**Why it matters**: ERR-2 — the report is the crate's public contract and it cannot express which failures are the ones its exit code is supposed to signal. Downstream it produces a wrong answer (a hook failure attributed to a parse error that never happened) and there is no way for a caller to filter or a user to tell the difference except by reading the free-text prefix.

**Fix shape**: give `FailedFile` a `kind` — e.g. `enum FailureKind { Metadata(io::ErrorKind), Read(io::ErrorKind), Parse }` — and keep `message` for display. Then decide the tracked-mode policy deliberately rather than by accident: a `NotFound` on a `git ls-files` entry is the expected consequence of an unstaged deletion or a sparse checkout, so it should be skipped (or counted separately), not reported as a check failure. Also stop counting metadata/read failures in `files_scanned`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 FailedFile distinguishes parse failures from metadata/read I/O failures via a typed kind, not a message prefix
- [ ] #2 A file listed by git ls-files but absent from the worktree (unstaged deletion, sparse checkout, skip-worktree) does not make ops check-json --tracked exit non-zero as a parse failure
- [ ] #3 files_scanned counts only files that were actually read and parsed, consistent with the files_skipped doc comment
- [ ] #4 Tests cover the tracked_only = true path, including a tracked-but-missing file
<!-- AC:END -->
