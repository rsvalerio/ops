---
id: TASK-1728
title: >-
  PATTERN-1: single-line POM container sections leak parent/organization/license
  values into top-level project fields
status: To Do
assignee:
  - TASK-1990
created_date: '2026-08-27 11:11'
updated_date: '2026-08-28 14:11'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-java/about/src/maven/pom.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:280` (`match_section_open`) and `extensions-java/about/src/maven/pom.rs:182` (`dispatch_started_line`)

**What**: `match_section_open` returns `Option<PomSection>` where `None` is overloaded to mean two different things:

1. "this line is not a section opener" — the caller must still run `parse_top_level` on it, and
2. "this line was a single-line container and I already consumed it" — the caller must NOT run `parse_top_level` on it.

`dispatch_started_line` cannot tell them apart, so it always falls through to `parse_top_level(line, data)` on `None`. Every single-line container branch in `match_section_open` (the `<scm>…</scm>` shortcut at line 300, the `<licenses>…</licenses>` shortcut at line 311, and the `SKIP_SECTIONS` single-line arm at lines 323-326 whose own comment says "Single-line container - ignore entirely") therefore has its line re-parsed at top level, which is exactly what `SKIP_SECTIONS` exists to prevent.

Reproduced against a verbatim copy of this module (only the `read_optional_text` call stubbed to `fs::read_to_string`):

```
<project>
    <parent><artifactId>parent-pom</artifactId><version>9.9.9</version></parent>
    <artifactId>child</artifactId>
    <version>1.0.0</version>
</project>
  => artifact_id = Some("parent-pom"), version = Some("9.9.9")     # WRONG: child's own coords lost

<project>
    <organization><name>Acme</name><url>https://acme.example</url></organization>
    <artifactId>real</artifactId>
</project>
  => name = Some("Acme"), scm_url = Some("https://acme.example")   # WRONG: org name/url captured

<project>
    <licenses><license><name>MIT</name></license></licenses>
</project>
  => license = Some("MIT"), name = Some("MIT")                     # WRONG: project name becomes "MIT"
```

Because `try_set_once` is first-writer-wins and `<parent>` conventionally precedes the project's own `<artifactId>`/`<version>`, the parent coordinates win outright. `MavenIdentityProvider::provide` folds these into `m.name = pom.name.or(pom.artifact_id)` and `m.version`, so the About card renders the parent POM's identity (or a license name) as the project's.

The existing tests miss this because every container test uses the multi-line shape: `parse_pom_organization_url_not_captured_as_scm` only exercises multi-line `<organization>`, there is no `<parent>` test at all, and `parse_pom_single_line_licenses` / `parse_pom_single_line_scm` assert only `pom.license` / `pom.scm_url` and never check that `pom.name` stayed `None`.

**Why it matters**: silently wrong project identity on a very common real-world POM shape - single-line `<parent>` blocks are standard in Maven multi-module children and in POMs emitted by formatters/generators. The user sees another artifact's name and version in `ops about` with no error and no log line.

**Suggested fix**: replace the `Option<PomSection>` sentinel with a three-state outcome that the dispatcher can act on, e.g.

```rust
enum SectionOutcome { Entered(PomSection), Consumed, NotASection }
```

`Consumed` returns from `dispatch_started_line` without calling `parse_top_level`; `NotASection` falls through as today. That encodes the invariant in the type (CL-3) rather than in a comment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 match_section_open no longer uses None to mean both 'not a section' and 'already consumed' - the two outcomes are distinct variants and dispatch_started_line only falls through to parse_top_level for the 'not a section' case
- [ ] #2 A single-line <parent><artifactId>p</artifactId><version>9.9.9</version></parent> followed by the project's own <artifactId>child</artifactId>/<version>1.0.0</version> yields artifact_id=child and version=1.0.0
- [ ] #3 A single-line <organization><name>Acme</name><url>https://acme.example</url></organization> leaves PomData::name and PomData::scm_url untouched
- [ ] #4 A single-line <licenses><license><name>MIT</name></license></licenses> sets license=MIT and leaves PomData::name as None
- [ ] #5 Regression tests cover all three shapes plus the single-line <scm> case (asserting name stays None), and the existing pom tests still pass
<!-- AC:END -->
