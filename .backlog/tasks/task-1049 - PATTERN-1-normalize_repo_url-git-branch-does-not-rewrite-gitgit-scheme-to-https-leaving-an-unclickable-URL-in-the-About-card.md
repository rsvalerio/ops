---
id: TASK-1049
title: >-
  PATTERN-1: normalize_repo_url 'git+' branch does not rewrite git+git:// scheme
  to https, leaving an unclickable URL in the About card
status: Done
assignee: []
created_date: '2026-05-07 20:54'
updated_date: '2026-05-07 23:28'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `/Users/rsvaleri/projects/ops/extensions-node/about/src/repo_url.rs:36-38`

**What**:
The catch-all `git+` branch executes after `git+ssh://` / `ssh://` / npm-shorthand have been ruled out, and only strips the `git+` prefix plus a trailing `.git`. For `git+git://github.com/o/r.git` it returns `git://github.com/o/r`, leaving the deprecated `git://` scheme on a URL the About card renders as a clickable link.

The sibling `git://` branch (lines 39-41) does rewrite to `https://`, so the asymmetry is solely an artefact of branch ordering: `git+` matches first and steals the prefix that `git://` would have rewritten. Other plausible inputs that hit the same path:

- `git+ftp://host/path.git` → `ftp://host/path` (also unbrowsable)
- `git+file:///srv/git/repo` → `file:///srv/git/repo` (leaks a local-only path into a public-looking About card)

**Why it matters**:
The About card surfaces this string as the canonical project URL — both for human display and for downstream metadata pipelines that scrape the JSON (audit logs, mirrors, sitemaps). A `git://` URL is non-routable through corporate proxies, deprecated by GitHub, and will fail every "click the repo link" eyeball check. The user perceives ops as producing a broken link with no diagnostic.

The fix is one extra line: after stripping `git+`, dispatch the *remaining* string back through `normalize_repo_url` so the `git://` / `ssh://` / scp branches get a chance — or run an explicit `if rest.starts_with("git://") { return format!("https://{}", rest_without_git_scheme.trim_end_matches(".git")); }` before the bare `.git`-strip.

Cross-references:
- TASK-0479 (Done) fixed the `git+ssh://` family but did not collapse `git+git://`.
- TASK-0318 (Done) DUP'd the prefix table but kept the per-branch ordering.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 normalize_repo_url("git+git://github.com/o/r.git") returns "https://github.com/o/r", matching the bare "git://..." branch
- [ ] #2 A regression test pins both the git+git:// case and a non-rewritable scheme (e.g. git+ftp://) so a future refactor of the prefix table can't silently re-introduce the bug
<!-- AC:END -->
