---
id: TASK-2596
title: 'READ-13: Trim the Concurrency Design pros/cons essay on Sqlite'
status: To Do
assignee: []
created_date: '2026-10-10 15:47'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - read
dependencies: []
parent_task_id: 'TASK-2625'
modified_files:
  - extensions/sqlite/src/connection.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/sqlite/src/connection.rs:Sqlite'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/connection.rs:39-52`

**What**: The `Sqlite` struct doc carries a "# Concurrency Design (EFF-001)" section written as a decision essay: a Pros/Cons pair ("Simple, safe, no risk of data races" vs "potential bottleneck under load") followed by a numbered list of three alternatives to switch to if read-heavy access ever becomes a problem (multiple read-only connections, connection pooling, RwLock over a connection-per-reader pool), closing with "this is acceptable".

**Why it matters**: READ-13 flags "why we picked X over Y" essays: they document a decision process rather than the end state. The enduring facts a reader needs are the contract — all operations serialize on one `Mutex<Connection>`; `rusqlite::Connection` is `Send` but not `Sync`, which rules out shared handles; ops runs one command at a time so serialization is fine. The three-way alternatives list is migration-planning material. Fix: compress to a short contract statement (serialization guarantee + the Send-not-Sync reason + expected single-command workload); drop the Pros/Cons headings and the numbered alternatives.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The Sqlite struct doc states the serialization contract and its rationale in prose without Pros/Cons headings or a numbered list of future alternatives
<!-- AC:END -->
