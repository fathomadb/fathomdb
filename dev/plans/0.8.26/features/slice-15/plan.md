---
title: FathomDB 0.8.26 Slice 15 — graph-evidence impact spike
status: DRAFT — AUTHORIZED DECISION SUPPORT
---

# Slice 15 plan — graph-evidence impact spike

## Purpose

Produce the performance, concurrency, response-shape, and erasure evidence
needed for HITL to rule D26-01. This is a bounded implementation spike between
Slices 10 and 20, not product delivery. The provisional comparison is an
opt-in sidecar on the existing V1 graph request/result versus required fields
on `GraphTargetV1`; neither option creates a V2 method, result, or router.

## Slice-complete workflow

Before work, enumerate changes since this plan, related Slice 10 work, current
graph/evidence/erasure code, assigned requirements and acceptance criteria, and
draft allocations. Adjust the plan if evidence changed, keeping it complete
but not overbuilt. Complete spike requirements and acceptance criteria, obtain
independent design review, use human-intent RED tests and the smallest GREEN
prototype, obtain independent code review and verification, and write status.
If a temporary branch or worktree is used, merge accepted work and clean it up.

## Requirements and acceptance

- Compare two changed-in-place V1 shapes: an opt-in result sidecar that leaves
  `GraphTargetV1` unchanged, and required evidence fields on every target.
- The sidecar candidate is requested by an added V1 request flag, returned as
  an optional field on `GraphExpandResultV1`, and absent byte-for-byte from the
  ordinary response when not requested.
- Hydrate at most 50 target and 50 terminal-edge revision identities after
  final graph selection using bounded indexed statements independent of the
  10,000-work-unit traversal limit.
- Prototype first-generation V1 point resolution on the primary connection;
  add no persistent sidecar, evidence table, cache, reader-pool route, or batch
  resolver.
- Preserve one indistinguishable unavailable refusal for missing, erased,
  expired, revoked, superseded, ineligible, and context-mismatched revisions.
- Prove erasure cannot report success before an in-flight resolver releases
  materialized bytes; preserve typed `ErasureIncomplete` for held WAL readers.
- Observe ordinary graph, search, sequential/concurrent write, RSS, WAL, and
  query-plan sentinels. A repeatable writer-throughput loss over 10 percent or
  writer-p99 increase over 25 percent triggers design review, not an invented
  product acceptance criterion.

## TDD RED/GREEN and measurements

1. Obtain independent review of [`design.md`](design.md) and resolve findings.
2. Stage or commit RED tests for opt-in omission, bounded hydration, exact
   node/edge resolution, nondisclosure, restart, erasure ordering, and
   ordinary-path sentinels.
3. Build the smallest disposable prototypes for both V1 response choices.
4. Measure opt-in graph expansion at 1 and 50 results and at 10,000 work units,
   including 1,000 sequential calls and eight concurrent callers. Retain the
   absent-option ordinary graph/search sentinels.
5. Measure node/edge resolution over 1 KiB and 100 KiB sources, 1,000
   sequential calls, eight concurrent callers, and one 50-resolution
   Memex-shaped run.
6. Measure 1 KiB writes alone, concurrent with opt-in graph expansion, and
   concurrent with repeated point resolution. Record p50/p95/p99, throughput,
   SQL count, response bytes, copied/hashed bytes, mutex wait, RSS, WAL, erasure
   latency, and outcomes.
7. Obtain independent code review and focused verification. Preserve or
   discard each prototype explicitly; do not let spike code drift into Slice
   20 by accident.

## Exit and HITL handoff

Write `status.md` with RED/GREEN and review commits, environment, raw result
locations, both-option comparison, ordinary-path evidence, erasure proof, and
a recommendation. Return D26-01 to HITL. Slice 20 remains blocked until the
public V1 shape is explicitly ruled.

Stop on a V2 surface/router, unindexed work, ordinary-response byte drift when
the opt-in flag is absent, erasure overtaking, false nondisclosure, schema
migration, persistent cache/table, or a need for product-scale refactoring.
