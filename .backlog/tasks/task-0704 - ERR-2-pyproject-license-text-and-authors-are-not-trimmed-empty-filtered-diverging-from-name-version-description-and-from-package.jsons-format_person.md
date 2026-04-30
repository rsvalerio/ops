---
id: TASK-0704
title: >-
  ERR-2: pyproject license text and authors are not trimmed/empty-filtered,
  diverging from name/version/description and from package.json's format_person
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:28'
updated_date: '2026-04-30 11:50'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:200-216`

**What**: `parse_pyproject` runs `trim_nonempty` on `name`, `version`, `description`, and (since TASK-0690 was filed) `requires_python` is open — but `license` (LicenseField::Text), `authors[].name`, and `authors[].email` are never trimmed. A `pyproject.toml` with `license = "  "` or `authors = [{ name = "  ", email = "  " }]` renders the About card with whitespace-only license and empty `<...>`-style author bullets. The sibling Node provider already canonicalised this in TASK-0563 / TASK-0566 (`format_person` trims+drops empties); pyproject diverged.

**Why it matters**: Cards rendered with whitespace-only fields look like rendering bugs to users; the asymmetry between the same author shape on Node vs Python is also a maintenance hazard — a future refactor that consolidates the two will have to rediscover the missing trim. Low severity (cosmetic), but documented divergence in the same provider style.

<!-- scan confidence: candidates to inspect -->
- lib.rs:200-205 (`LicenseField::Text(s) => Some(s)` — no trim)
- lib.rs:206-216 (authors filter_map — no trim on name/email)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 LicenseField::Text(s) is trimmed and dropped if empty, mirroring Python LicenseField::Table { text } and Node LicenseField::Object { type }
- [x] #2 pyproject author name/email run through trim_nonempty (or an equivalent helper shared with package.json format_person)
- [x] #3 test pins whitespace-only license and whitespace-only author components rendering as None
<!-- AC:END -->
