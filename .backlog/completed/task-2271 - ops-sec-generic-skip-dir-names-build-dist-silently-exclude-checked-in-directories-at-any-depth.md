---
id: TASK-2271
title: 'ops sec: generic skip-dir names (build, dist) silently exclude checked-in directories at any depth'
status: Done
assignee: []
created_date: '2026-09-16 19:28'
updated_date: '2026-09-16 21:24'
labels:
  - code-review
dependencies: []
modified_files: []
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
CodeRabbit finding on PR #59 (wave 16 / TASK-2264 follow-up).

`ops_core::stack::scan_skip_dirs` unions every stack's build dirs and both the detection walk and every Trivy scan skip those names at any depth. Generic names on that list — `build` (Python/Gradle), `dist` (Node/Python), `vendor` — therefore also exclude *checked-in* source directories: `services/build/Dockerfile` is never inspected by detection (cannot auto-select the misconfig scan) and is omitted from Trivy traversal via `--skip-dirs **/build`. That is a silent scan gap for repos that legitimately carry such paths.

The any-depth behaviour is TASK-2264's deliberate, tested, README-documented contract (speed, no build races), so this is a design tradeoff, not a bug fix to brute-force:

- keep as-is and document the gap;
- restrict generic names to known generated-output paths (e.g. skip `build` only when a sibling stack marker says Gradle/Python generated it), keeping unambiguous cache names (`target`, `node_modules`, `.venv`, `__pycache__`, `.gradle`, `.terraform`) global;
- or drop only the ambiguous names from the shared list.

Decision needed, then implementation + tests (`markers_inside_gradle_build_dirs_ignored_by_detection` and the README skip-list table pin the current behaviour and must move with the decision).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Design decision recorded on this task (keep / restrict / drop, with rationale)
- [ ] #2 Implementation + tests match the recorded decision; README skip-list section and detection tests updated in the same change
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Decision (2026-09-16): restrict. Generic names (build, dist) are excluded only where ops_core::stack::is_generated_build_dir says the parent holds a manifest of a stack declaring that build dir; unambiguous cache/output names (target, node_modules, .venv, venv, __pycache__, .terraform, vendor, .gradle, .git) stay global by-name skips. Trivy receives generic dirs as exact discovered relative paths (verified against Trivy 0.74: nested paths scope to themselves, bare names match root-level only) instead of blanket **/build; the detection walk doubles as the inventory, so its complete() early-exit was removed (walk cost measured at ~76ms on this repo; file-level work stays gated so the k8s content probe still stops early). Implemented on PR #59; gradle marker test now carries a build.gradle fixture (generated case), new tests pin the checked-in services/build case and the per-path Trivy args.
<!-- SECTION:NOTES:END -->
