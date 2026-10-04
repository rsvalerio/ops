---
id: TASK-2404
title: 'READ-13: hook-common docs narrate past implementations and task history'
status: Triage
assignee: []
created_date: '2026-10-04 14:16'
labels:
  - code-review-rust
  - READ
dependencies: []
modified_files:
  - extensions/hook-common/src/install.rs
  - extensions/hook-common/src/git.rs
  - extensions/hook-common/src/lib.rs
  - extensions/hook-common/src/fixtures.rs
  - extensions/hook-common/src/git_state.rs
  - extensions/hook-common/src/config.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/hook-common/src:docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/install.rs:88,121,311,320,323`, `git.rs:51,108,244,401`, `lib.rs:94,147`, `fixtures.rs:3,50`, `git_state.rs:122`, `config.rs:221`

**What**: `///` and `//!` blocks describe the journey, not the end state: "the previous shape opened hook_path itself...", "used to be hand-copied", "previously surprised users", "until now the only one with no symlink check", "the previous implementation read_to_string -> fs::write", "the TASK-1113 stale-recovery branch". The `reject_symlinked_hook`, `write_new_hook`, `upgrade_legacy_hook` and `hook_script!` docs are mostly migration narrative with TASK ids. A reader using the API gets no value from it, and it goes stale. Candidate lines to inspect; <!-- scan confidence: candidates to inspect -->.

**Why it matters**: READ-13: process artifacts in docs look authoritative but rot; the invariants (atomic stage + rename, symlink refusal, bounded read) should be stated as current behaviour, with history left to git/PR descriptions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Doc comments state current behaviour and invariants only; no 'previously', 'used to', 'previous shape' narrative
- [ ] #2 Retained security rationale is rewritten as present-tense invariants; history is left to git log
<!-- AC:END -->
