---
id: TASK-1062
title: >-
  PATTERN-1: extract_urls collapses pyproject [project.urls] documentation key
  into the homepage slot, masking the distinct PEP 621 link
status: Done
assignee: []
created_date: '2026-05-07 21:04'
updated_date: '2026-05-07 23:15'
labels:
  - code-review
  - triage
  - extensions-python
  - extract_urls
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-python/about/src/lib.rs:260-284 extract_urls picks the homepage from candidates ["homepage", "home", "home-page", "documentation"]. PEP 621 (and PyPI) distinguish `Homepage` from `Documentation` as separate, semantically distinct labels — projects routinely set both (e.g. Homepage = https://example.org, Documentation = https://docs.example.org). Folding `documentation` into the homepage slot means a pyproject.toml with ONLY a `Documentation` URL renders that link as the project homepage, while a pyproject.toml with BOTH renders only Homepage and silently discards Documentation. The About card schema lacks a documentation field today, so the surface fix is: drop `documentation` from the homepage candidate list so its absence falls through cleanly to None rather than being misrepresented as Homepage. If the About card should expose Documentation as its own bullet (PEP 621 alignment), file a follow-up to extend ParsedManifest + ProjectIdentity. Add a regression test: pyproject with only `Documentation = "https://docs.x"` must yield homepage=None, not the docs URL.
<!-- SECTION:DESCRIPTION:END -->
