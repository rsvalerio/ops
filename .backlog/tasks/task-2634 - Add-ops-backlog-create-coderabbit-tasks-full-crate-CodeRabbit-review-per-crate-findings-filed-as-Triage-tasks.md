---
id: TASK-2634
title: 'Add ''ops backlog create-coderabbit-tasks'': full-crate CodeRabbit review per crate, findings filed as Triage tasks'
status: In Progress
assignee: []
created_date: '2026-10-17 13:05'
updated_date: '2026-10-17 14:02'
labels:
  - backlog
  - code-review
  - coderabbit
  - feature
dependencies: []
modified_files: []
priority: medium
ordinal: 164000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
New command, a sibling of `ops backlog create-review-tasks` (extensions/create-review-tasks), that runs CodeRabbit against the **whole content** of each crate, not just changed files.

CodeRabbit CLI (checked on v0.9.0) only reviews diffs. `--dir <path>` narrows the review to *changes* inside a folder, and there is no whole-folder mode. The workaround is to make every file in the crate look new: an orphan branch with an empty commit, in a throwaway worktree.

### Shape
Like `create-review-tasks`, iterate the workspace `review_targets` provider. For each crate, either:
- (a) file one `REVIEW: Run CodeRabbit full review against <crate>` task (status To Do), parented under a `coderabbit-review-request-<date>-<n>` main task, whose body has the exact worktree steps below so an agent or human can run it later; or
- (b) with `--run`, do the steps directly.

### Steps per crate (what the task body instructs and `--run` automates)
1. `git worktree add --detach <tmp>/ops-cr-<crate>` from HEAD.
2. In it: `git checkout --orphan cr-full-<crate>`, then `git rm -r --cached -q .` (files stay on disk, untracked), then `git commit --allow-empty -m "empty base"`.
3. `coderabbit review --agent --uncommitted --include-untracked --dir <abs crate path> -c AGENTS.md` (add `--fresh` and `--use-credits` as flags).
4. `coderabbit review --show-prompts` (prints the prompts of the review just run; it does **not** start one, so step 3 is required first). Parse the structured findings from `--agent` output and/or the prompts.
5. For each finding, create a backlog task **in the main checkout's** `.backlog` (not the worktree's orphan tree): status Triage, labels `coderabbit`, `code-review`, plus a category label; `modified_files` = the finding's file; body with File / What / Why / Suggested fix like the code-review-rust tasks. Use `unless_exists` with a dedup key (e.g. `coderabbit:<crate>:<file>:<hash of finding>`) so re-runs don't duplicate.
6. Commit the new task files on the current branch (`chore(backlog): file CodeRabbit findings for <crate>`).
7. `git worktree remove --force` the worktree and `git branch -D cr-full-<crate>`, in a drop guard so this also runs on error or Ctrl-C.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `ops backlog create-coderabbit-tasks [--crate <name>...] [--dry-run]` files one REVIEW task per review target, whose body has the worktree/orphan-branch steps
- [ ] #2 `--run` executes the steps per crate: worktree, orphan empty base, coderabbit review --agent, --show-prompts, one Triage task per finding, commit, worktree and branch removed
- [x] #3 Worktree and temp branch are always cleaned up, including on failure (drop guard / explicit cleanup path), and the main checkout is never switched or dirtied apart from the new task files
- [x] #4 Findings are deduplicated with unless_exists so re-running on the same crate files no duplicates
- [x] #5 Fails clearly when coderabbit is missing or unauthenticated (`coderabbit auth status`) or over quota (`coderabbit usage`), before creating a worktree
- [x] #6 `--dry-run` and global `ops --dry-run` preview the crates/tasks without creating a worktree or calling coderabbit; registered in unpreviewable_mutation like create-review-tasks
- [x] #7 Unit tests for finding parsing and task rendering, plus an integration test with a stub coderabbit on PATH

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Reuse extensions/create-review-tasks (review_targets iteration, request/subtask creation) and crates/backlog wave_claim.rs (git worktree add/remove, rollback). Wire args in crates/cli/src/args.rs and dispatch in backlog_cmd.rs/subcommands.rs. Before parsing, check the `--agent` output format on the installed coderabbit version and pin it in a fixture.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented in extensions/create-coderabbit-tasks (new crate) plus a RequestScheme parameter on the create-review-tasks engine, which files the coderabbit-review-request-<date>-<n> set (default mode) through the same allocation, claim re-check and rollback code. --run: preflight (no staged index, coderabbit auth status, coderabbit usage) runs before any worktree exists; the ReviewWorktree drop guard removes the worktree and the cr-full-<crate> branch; a signal-hook flag catches the first SIGINT/SIGTERM so Ctrl-C unwinds through the guard (a second signal kills). A branch left by a SIGKILLed run is refused, with the commands that remove it. Findings are filed with create_task + dedup_key coderabbit:<crate>:<file>:<sha256-16 of the whitespace-collapsed comment+instructions>, and committed via backlog commit.

Open for AC #2: (a) the --agent NDJSON parser and tests/fixtures/agent-output.ndjson follow CodeRabbit's documented schema (type/severity/fileName/codegenInstructions/suggestions/comment, then complete/error). They were NOT captured from a real v0.9.0 run, because cli.coderabbit.ai is blocked from the build container. Re-pin the fixture from a real run. (b) --run does not call 'coderabbit review --show-prompts': the documented codegenInstructions field already carries the agent prompt, and its plain-text output has no documented format to parse. The REVIEW subtask body still lists it as a manual step. (c) 'coderabbit usage' is judged by its exit status only; there is no parsing of quota numbers.
<!-- SECTION:NOTES:END -->
