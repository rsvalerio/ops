---
id: TASK-2159
title: 'PERF-3: every tokei candidate is walked, stat''d and language-classified twice, and the second walk is given up to 50,000 roots'
status: To Do
assignee: []
created_date: '2026-09-08 07:04'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - performance
dependencies: []
parent_task_id: 'TASK-2244'
modified_files:
  - extensions/tokei/src/lib.rs
priority: low
ordinal: 72000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:233-311` (`scan_tokei`)

**What**: `scan_tokei` runs its own `ignore::WalkBuilder` over `working_dir`, and for each entry calls `LanguageType::from_path` and `entry.metadata()`. It then hands the resulting `Vec<PathBuf>` of up to `ScanLimits::files` (50,000) candidates to `languages.get_statistics(&candidates, &[], &config)`.

Tokei does not treat that slice as a file list. `utils::fs::get_all_files` (tokei 14.0.0, `src/utils/fs.rs:14-57`) builds `WalkBuilder::new(paths[0])` and then calls `walker.add(path)` for every remaining path, i.e. it constructs a walker with one root per candidate and re-runs the whole `ignore` pipeline on each: parent-`.gitignore` resolution (`parents(true)`), `.gitignore`/`.git/info/exclude`/global-gitignore matching, hidden-file rules, the `.tokeignore` custom ignore, plus a fresh `stat`. It then re-runs `LanguageType::from_path` on each entry — the exact classification `scan_tokei` already performed at line 263.

**Why it matters**: on a large monorepo the walk is the dominant cost of this provider and it is paid roughly twice, with the second pass doing strictly more per file than the first (ignore-matcher construction per root, not just a stat). The redundancy is invisible from the call site — `get_statistics(&candidates, ...)` reads like "count these files" — so it will not be noticed without reading tokei's source.

**Direction**: the per-file counting primitive is public — `LanguageType::parse(path, &config) -> Result<Report, (io::Error, PathBuf)>`. Driving that directly over the already-filtered candidate list (in parallel, if the current wall-clock matters) removes the second walk entirely, and as a bonus gives the per-file open errors that `scan_tokei:314-318` currently has to infer from `candidates.len() - records.len()`.

<!-- scan confidence: verified against tokei 14.0.0 source, not a grep heuristic -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Candidate files are counted without handing tokei a second walker rooted at every candidate path
- [ ] #2 A file tokei cannot open is reported directly rather than inferred from a records-vs-candidates shortfall
- [ ] #3 Existing behaviour is preserved: the oversize, unreadable, truncated and depth-cap tests still pass unchanged
<!-- AC:END -->
