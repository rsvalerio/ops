---
id: TASK-0976
title: >-
  PATTERN-1: looks_like_module_version accepts non-semver tokens, may suppress
  legal local replace targets
status: Done
assignee: []
created_date: '2026-05-04 21:58'
updated_date: '2026-05-04 23:04'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/go_mod.rs:101-110`

**What**: `looks_like_module_version(s)` returns true for any string that begins with `v<digit>` and contains a `.`. The intent (per the doc comment) is to distinguish a remote `replace` (target carries a version like `v1.2.3`) from a local filesystem path. The matcher is broader than that intent: `v1.foo.com/path`, `v9.local`, `v0.x` all match. cmd/go's `replace` syntax means the second whitespace-separated token of the target is the version, so a local path that *contains* whitespace and whose second whitespace-separated component happens to begin with `v<digit>.` will be misclassified as a remote replace and silently dropped from `local_replaces`.

Reproducer: `replace ex.com/m => ./has space/v1.foo` — the second token is `space/v1.foo`, which does not start with `v<digit>` so it does not trigger; safe. But `replace ex.com/m => ./root v1.snapshot` — second token `v1.snapshot` matches the heuristic and the local target gets dropped even though there is no path-then-version split (cmd/go would treat the whole thing as a single path target if it had no whitespace, but the heuristic short-circuits before path-shape verification).

**Why it matters**: Under-counting `local_replaces` cascades into `compute_module_count` which underrepresents workspace size in the About card. The bug is data-corrupting silently — no warn, no debug. Either tighten the matcher to require the `vMAJOR.MINOR.PATCH` shape (digits/dots only, with at least two dots) or invert the precedence: only treat the second token as a version if the first token is itself a valid module path, not a path containing spaces.

<!-- scan confidence: candidates to inspect — the false-positive surface depends on whether project-side `go.mod` files in the wild ever produce a path with whitespace + a v<digit>.X second token. Likelihood is low but the silent drop is the failure mode worth fixing. -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 looks_like_module_version requires the strict vMAJOR.MINOR(.PATCH)? shape with all-digit components
- [ ] #2 Test covers a local target whose second whitespace token starts v<digit>. but is not a semver
<!-- AC:END -->
