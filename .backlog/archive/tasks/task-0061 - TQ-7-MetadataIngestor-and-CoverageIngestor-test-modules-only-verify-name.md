---
id: TASK-0061
title: 'TQ-7: MetadataIngestor and CoverageIngestor test modules only verify name()'
status: Triage
assignee: []
created_date: '2026-04-14 20:54'
labels:
  - rust-test-quality
  - TestGap
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions-rust/metadata/src/ingestor.rs and extensions-rust/test-coverage/src/ingestor.rs each have a single test that asserts name() returns the expected string. The collect() and load() methods — which handle filesystem I/O, cargo metadata execution, DuckDB schema initialization, table creation, checksumming, and staged-file cleanup — have zero test coverage. These are the most failure-prone paths (external tool invocation, SQL execution, file lifecycle management). CoverageIngestor delegates to SidecarIngestorConfig, partially reducing risk, but MetadataIngestor has inline load logic with 7 sequential fallible operations.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 MetadataIngestor::load: test with in-memory DuckDB and sample metadata.json
- [ ] #2 CoverageIngestor::load: test with in-memory DuckDB and sample coverage data
- [ ] #3 Both: test collect() error handling when working directory is invalid
<!-- AC:END -->
