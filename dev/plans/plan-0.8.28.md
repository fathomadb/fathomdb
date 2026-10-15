---
title: FathomDB 0.8.28 — Tegra private CUDA memory pool on forked cudarc
status: ACTIVE
target_release: 0.8.28
execution_model: direct; no Steward or Orchestrator
---

# FathomDB 0.8.28 — Tegra private CUDA memory pool on forked cudarc

## Goal and scope

0.8.28 ships the owner-placed Tegra private CUDA memory pool and moves it
onto forked, published crates:

- **Slice 30 (D28-08):** the private pool as the default on aarch64 Linux
  integrated GPUs, with context-loss detection and `doctor cuda-allocator`.
  Complete on `release/0.8.28`.
- **Slice 31 (D28-13):** consume `cudarc-fathomdb` and the Candle fork
  crates from crates.io, delete `third_party/cudarc-0.19.7`, and qualify
  CUDA 12.6 and 13.0. Fork work is planned, reviewed and qualified inside
  each fork, first (owner ruling Q14).

The other entries in the [draft scope](0.8.28-draft-scope.md) are not on the
ladder. D28-09 to D28-12 are owner-directed but not yet placed; D28-01 to
D28-07 are candidates. D28-14 (filtered direct node and edge body FTS) is
placed in 0.8.28 (seq-333) but has no slice yet. All three are open
decisions in
`dev/plans/release-state-0.8.28.json`. This plan adds no scope; it indexes
the slice plans, which are the working authority for their slices.

## Quality-roadmap consultation

Read [the quality testing roadmap](../notes/fathomdb-quality-testing-roadmap.md)
on 2026-10-13, as AGENTS.md requires.

- **Applies:** "verify dependency upgrades and packaging on supported
  platforms" (remainder of 0.8.x). Slice 31 replaces a vendored dependency
  with forked registry crates and qualifies them on the RTX 3090, the Orin
  and a GCP T4 with CUDA 13.0. Resource-leak checks apply to the pool:
  Slice 30's CB1/CB2 and the forks' release oracles measure pool memory
  after drop and sync.
- **Applies, bounded:** one regression per confirmed defect. The fork
  qualification defects (FC8 G6 findings) each gained a RED-first test in
  the cudarc fork.
- **Deferred:** fault-injection campaigns, mutation checks beyond the forks'
  targeted mutations, and fuzz smoke cases. 0.8.28 changes the CUDA
  allocation substrate, not FathomDB's persistence composition; the
  roadmap's anomaly-testing targets (commit, queue, provider, erasure
  boundaries) are untouched. D28-09, if placed, would bring them in.
- **New evidence:** the measured Tegra pool facts (a pool cap gives about a
  third of the cap as usable memory; about four uncapped pools per process)
  are platform limits that FathomDB sizing must respect; Slice 31's design
  records them.

## Requirements

Requirements and acceptance criteria are per slice:

- Slice 30: [plan](0.8.28/features/slice-30/plan.md) § 3 (G1–G10) and
  AC30-01 to AC30-11.
- Slice 31: REQ31 and AC31 in the
  [Slice 31 plan](0.8.28/features/slice-31/plan.md) § 5, finalized in W1 as
  `requirements.md` and reviewed by R-REQ.

`dev/acceptance.md` remains locked; these identifiers are release-local.

## Cross-cutting DoD

Every slice: owner rulings recorded in the ledger; TDD with RED before
GREEN for code; independent Opus reviews at the slice plan's gates; the
full `./scripts/agent-verify.sh` gate for code changes; GPU qualification
receipts on the named hosts; `windchill3` driver and `/usr/local/cuda`
unchanged; no upstream PRs, issues or pushes.

## Reserved-gap policy

Slice numbers between and after 30 and 31 are reserved. A slot stays empty
unless a placement decision adds a slice; it is never an implicit patch lane.

## Immediate next slice

The slice ladder is complete: Slice 31 merged into `release/0.8.28` at
`1ceefddd8`. Next are the release-level steps: the open decisions below,
then the owner's publication decision (Slice 31 plan W8 and R-CODE-4).

## Slice ladder

| Slice | Scope | Plan |
| ---: | --- | --- |
| 30 | Tegra private CUDA memory pool (D28-08) | [slice-30/plan.md](0.8.28/features/slice-30/plan.md) |
| 31 | cudarc and Candle forks from crates.io (D28-13) | [slice-31/plan.md](0.8.28/features/slice-31/plan.md) |
