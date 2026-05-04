---
id: TASK-0998
title: 'PATTERN-1: merge_alias_for double map lookup on stale-alias prune'
status: Triage
assignee: []
created_date: '2026-05-04 22:01'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:198-222`

**What**: `merge_alias_for` prunes stale aliases owned by a previous
registration of the same id with this nested-conditional shape:

\`\`\`rust
for old_alias in old_spec.aliases() {
    if let Some(owner) = self.non_config_alias_map.get(old_alias.as_str()) {
        if owner == id.as_str() {
            self.non_config_alias_map.remove(old_alias.as_str());
        }
    }
}
\`\`\`

Each iteration probes the HashMap twice: once for `get`, again for `remove`.
The newly-inserted-aliases loop below has the same `get` + `insert` shape.

**Why it matters**: Pure micro-PATTERN issue, not a perf hot path. But the
`Entry` API expresses the "lookup once, conditionally mutate" intent
without the duplicate probe and without the temptation to drift between
the two calls (e.g. a future caller that mistakenly compares against a
clone of the lookup string would re-introduce the alias). Same fix
applies to the new-alias loop:

\`\`\`rust
match self.non_config_alias_map.entry(old_alias.to_string()) {
    Entry::Occupied(o) if o.get() == id.as_str() => { o.remove(); }
    _ => {}
}
\`\`\`

Comparable rewrites already landed for `CommandRegistry::insert`
(PATTERN-3 / TASK-0753) — the same ergonomic + correctness contract is
worth applying here.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 merge_alias_for routes stale-alias prune through Entry::Occupied (one map probe per branch)
- [ ] #2 new-alias loop routes through Entry as well so the warn + insert paths share the same lookup
<!-- AC:END -->
