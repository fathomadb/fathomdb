---
title: FathomDB 0.8.30 Memex adoption scope
status: PROPOSED
target_release: 0.8.30
date: 2026-11-21
---

# FathomDB 0.8.30 Memex adoption scope

## Context and authority

FathomDB **0.8.30** is already completed and published (`v0.8.30`, schema **37**;
see [0.8.30 release plan](plan.md) and
[0.8.30 release notes](../../../docs/release-notes/0.8.30.md)). This document
scopes the post-publication adoption of `0.8.30` by **Memex** (a personal agent)
and the operational evidence to collect before `0.8.31` begins. It does not
reopen the closed `0.8.30` release or authorize a `0.8.30.1` patch unless Memex
reproduces a blocking defect.

## In scope

1. **Repository and workspace hygiene before consumer work**
   - Synchronize stale local `main` checkouts with `origin/main` (`v0.8.30`) and
     consolidate or prune abandoned worktrees.
   - Capture and archive a **closed golden `0.8.30` (schema 37) fixture
     database** (plus a representative anonymized Memex workload database) to
     serve as real producer fixtures for `0.8.31` Inspector/upgrade-helper
     qualification and `0.8.34` full data migration.
2. **Memex adoption on published `fathomdb==0.8.30`**
   - Integrate Memex against the published `0.8.30` SDK artifact (dedicated
     local embedded store per personal agent instance).
   - Enforce the host-vs-database responsibility boundary: Memex owns caller
     scope, provenance trust roles (approved policy, user statement, agent
     inference), memory admission, and abstention/sufficiency policy; FathomDB
     owns canonical storage, provenance/revisions, hybrid retrieval, frozen
     evidence, and erasure.
3. **External-reviewer P0 operability verification on the real Memex corpus**
   - Exercise backup, restore, and full rebuild from authoritative Memex
     records on the pinned `0.8.30` artifact, comparing revisions, evidence
     spans, and graph results before and after restore/rebuild.
   - Exercise interrupted-write / WAL reopen and post-commit projection
     readiness under the real Memex write/read cadence, reusing `0.8.30`
     [Slice 55 integrity and recovery patterns](features/slice-55/status.md) and
     `fathomdb doctor` / `safe-export`.
4. **Input capture for `0.8.31`–`0.8.34`**
   - Record real Memex friction points: missing outside-`body` node/edge
     properties, historical graph needs, SDK vs. MCP workflow gaps, idle
     memory/model lifecycle (`release_models()`), and real record-length
     distribution (needed evidence for
     [overflow-chunking tripwires](../../design/overflow-chunking-reevaluation.md)).

## Out of scope

- Any `0.8.30` schema change (format remains **37**) or new FathomDB engine
  features.
- Node/edge property graph expansion, strict writes, historical graph replay,
  and the minimal schema-37→38 Memex upgrade helper (allocated to
  [0.8.31](../0.8.31/scope.md)).
- Engine-enforced caller scope, read+write MCP, opt-in bulk writes, and
  power-loss/fuzzing durability campaigns (allocated to
  [0.8.32](../0.8.32/scope.md)).
- Full multi-version migration and Hermes/OpenClaw personal-agent enablement
  (allocated to [0.8.34](../0.8.34/scope.md)).

## Migration bridge (settled 2026-11-21)

Per owner ruling (2026-11-21), `0.8.31` includes a **minimal schema-37→38
export/upgrade helper** so Memex can upgrade live `0.8.30` stores directly to
`0.8.31` ahead of the full general multi-version `fathomdb migrate` contract in
[0.8.34](../0.8.34/scope.md).
