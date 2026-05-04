---
id: TASK-1005
title: >-
  ARCH-2: HookConfig::SidecarIngestorConfig::cleanup_artifacts NotFound branch
  silences sidecar-removal failure but still removes sidecar
status: Triage
assignee: []
created_date: '2026-05-04 22:03'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:250-266`

**What**: `cleanup_artifacts` has three arms:

1. JSON `Ok(())` → remove sidecar.
2. JSON `Err(NotFound)` → remove sidecar (silent).
3. JSON `Err(other)` → log warn + leave sidecar in place (so a recovery run can recompute checksum from leftover JSON).

Arm #2 is the policy gap: the comment (lines 245-249) explicitly justifies arm #3 as "leave sidecar to drive recovery on next run". But arm #2 deletes the sidecar even though the file was already missing (so leftover JSON cannot be the recovery driver — it's gone). If the JSON went missing for a reason other than a successful previous load (e.g., the user manually `rm`'d the staging file mid-pipeline; an external scrubber on `target/`), the data_sources row in DuckDB references content that no longer exists *and* the sidecar that would have driven recovery is also gone. The next `--refresh` cycle starts from a half-state with no breadcrumb.

**Why it matters**:
- The ERR-1 / TASK-0466 contract (referenced at line 245) is "sidecar removed only after JSON removal succeeded". NotFound is *not* a successful removal; it's an unexpected absence. The current code conflates "I removed it" with "it wasn't there to remove" and applies the success policy to both.
- Operationally low impact (the next refresh re-collects), but the comment-vs-code drift is exactly the kind of trust-erosion finding ARCH-2 calls out: a future maintainer reading the comment will believe the sidecar always survives a JSON-removal anomaly, and rely on that invariant for some other recovery flow.

**Recommended fix**: in arm #2, log `tracing::debug!("cleanup_artifacts: JSON staging file already absent before removal; removing sidecar anyway")` so the operationally-rare case is at least visible. Or, more conservatively, treat NotFound the same as the `Err(other)` arm and leave the sidecar in place — the cost is at most one stale sidecar that the next ingest's `read_workspace_sidecar` will immediately overwrite.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cleanup_artifacts NotFound branch logs at tracing::debug! before removing the sidecar.
- [ ] #2 Decision documented (in code, near the match) on whether NotFound counts as 'successful removal' for the ERR-1 / TASK-0466 contract.
<!-- AC:END -->
