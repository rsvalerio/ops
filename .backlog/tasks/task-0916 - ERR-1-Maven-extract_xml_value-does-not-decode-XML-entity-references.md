---
id: TASK-0916
title: 'ERR-1: Maven extract_xml_value does not decode XML entity references'
status: Triage
assignee: []
created_date: '2026-05-02 10:11'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven/pom.rs:316`

**What**: extract_xml_value returns the raw byte slice between tag and end-tag; entity references such as &amp;, &lt;, &gt;, &quot;, and &#39; pass through verbatim. A pom.xml with `<description>Foo &amp; Bar</description>` renders literal `Foo &amp; Bar` in the About card.

**Why it matters**: POMs commonly carry encoded ampersands and angle brackets in description/name/url; surfacing the raw entity to operators is incorrect output that the line-based parser silently produces.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extract_xml_value (or a wrapper) decodes the standard XML predefined entities (&amp; &lt; &gt; &quot; &apos;) plus numeric &#NNN; / &#xHH; references
- [ ] #2 Test covers a pom.xml with encoded ampersand in description
<!-- AC:END -->
