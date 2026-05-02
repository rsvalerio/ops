---
id: TASK-0916
title: 'ERR-1: Maven extract_xml_value does not decode XML entity references'
status: Done
assignee: []
created_date: '2026-05-02 10:11'
updated_date: '2026-05-02 14:56'
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
- [x] #1 extract_xml_value (or a wrapper) decodes the standard XML predefined entities (&amp; &lt; &gt; &quot; &apos;) plus numeric &#NNN; / &#xHH; references
- [x] #2 Test covers a pom.xml with encoded ampersand in description
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
extract_xml_value now returns Option<Cow<str>> and routes through decode_xml_entities, which decodes the XML-1.0 predefined entities (&amp; &lt; &gt; &quot; &apos;) plus numeric &#NNN; / &#xHH; references. Borrow path stays zero-alloc for entity-free values. Unknown entities pass through verbatim so we don\\"t silently corrupt unfamiliar content. Updated existing tests for the Cow change and added extract_xml_value_decodes_predefined_entities + extract_xml_value_passes_through_unknown_entities.
<!-- SECTION:NOTES:END -->
