---
title: 0.8.25 Slice 25 independent implementation review — cycle 3
status: COMPLETE
review_cycle: 3
reviewed_on: 2026-09-04
reviewed_commit: 6e9b2ba2
verdict: FAIL
---

# Slice 25 independent implementation review — cycle 3

## Verdict

**FAIL.** The telemetry, cursor-refusal, operation fault, and shared-fixture
corrections pass. One P1 replay defect and one P2 verification gap remain.

## Findings

- A legitimate nested provenance refusal is stored as `write_refused` at its
  exact nested field, but persisted-receipt validation admits only the record
  root. The first request returns a terminal refusal and exact replay fails
  with `Storage`.
- The `reference_unavailable` receipt validator admits arbitrary descendants
  by suffix rather than the three governed paths.
- Verification does not yet cover every refusal-path rule, persisted replay as
  a property, full rollback state, raw database/WAL erasure, reverse-index
  query planning, embedded-NUL source identity, or exact receipt collection
  maxima and one-over failures.

Focused Rust/schema and TypeScript checks passed at the reviewed commit.
Packaged Python was unavailable to the reviewer because the worktree symlink
resolved to a stale main-checkout native build; release verification must use a
fresh isolated wheel.
