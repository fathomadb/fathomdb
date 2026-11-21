---
title: FathomDB 0.8.30 Memex adoption plan
status: PROPOSED
target_release: 0.8.30
date: 2026-11-21
---

# FathomDB 0.8.30 Memex adoption plan

## Goal

Start running **Memex** on published FathomDB **0.8.30** (`v0.8.30`, schema 37),
prove day-2 operability on a real personal-agent corpus, preserve golden
schema-37 fixtures for downstream releases, and feed concrete operational
findings into `0.8.31` and `0.8.33`.

## Quality roadmap and standing tripwire check

- **Quality testing roadmap**
  ([fathomdb-quality-testing-roadmap.md](../../notes/fathomdb-quality-testing-roadmap.md)):
  Applies the roadmap's real-database recovery, resource-leak, and
  installed-artifact checks to Memex's actual corpus and pinned `0.8.30`
  package. Coverage-guided fuzzing and power-loss proofs remain deferred to
  [0.8.31](../0.8.31/plan.md).
- **Standing tripwires**
  ([overflow-chunking-reevaluation.md](../../design/overflow-chunking-reevaluation.md)):
  No tripwire (`T1`–`T3`) has fired. Memex adoption will collect the explicitly
  requested **real record-length distribution** evidence (proportion of records
  exceeding 512 wordpieces) to inform future re-evaluation.

## Requirements and acceptance signals

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R30M-01 | Clean workspace baseline and golden `0.8.30` fixtures | Local `main` synced to `origin/main` (`v0.8.30`); stale worktrees pruned; closed real `0.8.30` (schema 37) producer database archived with SHA-256 manifest, active/superseded nodes and edges, provenance, dependencies, and operational state. |
| R30M-02 | Memex end-to-end integration on published `0.8.30` | Memex opens, writes, retrieves (FTS + hybrid), resolves frozen evidence, and closes/exits cleanly against the registry-installed `0.8.30` artifact; host explicitly tags source authority (user statement vs. agent inference vs. policy) and owns abstention. |
| R30M-03 | Operability proof on real Memex corpus (External Reviewer P0) | On the pinned `0.8.30` package: (1) interrupt a write during projection lag, reopen, and verify committed records and eventual index readiness; (2) execute backup/restore and full rebuild from authoritative Memex inputs, verifying matching revisions, evidence spans, and graph outputs. |
| R30M-04 | Friction and sizing feedback captured for `.31`/`.33` | Written adoption note recording Memex record-length distribution, idle/active RSS (including `release_models()` behavior), and concrete gaps prioritized for `0.8.31` (properties, history, scoping, MCP) and `0.8.33` (migration, personal-agent ergonomics). |

## Execution steps

| Step | Deliverable | Depends on | Exit criteria |
| --- | --- | --- | --- |
| M1 | Workspace sync and golden schema-37 fixture archive | Published `v0.8.30` | Clean checkout baseline; immutable schema-37 fixture bundle and checksums committed or stored with manifest. |
| M2 | Memex integration against pinned `fathomdb==0.8.30` | M1 | Memex read/write/frozen-evidence/erasure workflows pass on the installed `0.8.30` package without local editable workarounds. |
| M3 | Memex corpus operability drill (backup, restore, rebuild, crash/WAL) | M2 | Recorded receipts for backup/restore parity, full rebuild parity, interrupted-write WAL reopen, and `fathomdb doctor` verification. |
| M4 | Adoption findings handoff to `0.8.31` and `0.8.33` | M3 | Adoption report written, including record-length stats, memory profile, and confirmed defect list (if any require a `0.8.30.1` patch). |

## Open decisions

- **Memex upgrade bridge**: Confirm whether Memex stays pinned to `0.8.30` (or
  rebuilds from authoritative source) until `0.8.33` schema migration lands, or
  whether a schema-37→38 export/upgrade helper is needed earlier in `0.8.31`.
- **Patch trigger**: Any data-loss, lock, or crash defect discovered in Memex
  adoption opens a scoped `0.8.30.1` fix; feature gaps route to `0.8.31`.
