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
- **Slice 40:** FTS ranking, latency and page-cache defects found by the
  COMP benchmarks. In progress on `slice/0.8.28-40-fts-ranking-defects`.
- **Slice 41 (`SDK-CONTEXT-PACK`, owner ruling 2026-10-16):** SDK
  context pack, full-render override, configurable query stop words.
  Reserved here; planned on `slice/0.8.28-41-sdk-evidence-helper`, whose
  slice docs are not on this branch. Depends on Slice 40; lands before
  Slice 42.
- **Slice 42 (owner ruling 2026-10-16):** make the slice135 descriptor
  check independent of parallel tests (todos ledger (`dev/todos-and-considerations-ledger.jsonl`) seq 312, `TC-f9fa3b87-6f23-427b-b027-6889e52b3bef`), and investigate how published
  Rust artifacts get per-connection page caches (todos ledger seq 313, `TC-4786ddae-2a6a-4da9-b618-3f659cfacb6e`). Depends on Slice 40;
  lands after Slice 41.

The other entries in the [draft scope](0.8.28-draft-scope.md) are not on the
ladder. D28-09 to D28-12 are owner-directed but not yet placed; D28-01 to
D28-07 are candidates. D28-14 (filtered direct node and edge body FTS) is
placed in 0.8.28 (seq-333) but has no slice yet. All three are open
decisions in
`dev/plans/release-state-0.8.28.json`. This plan indexes the slice plans,
which are the working authority for their slices.

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
- Slice 40: REQ40 and AC40 in
  [slice-40/requirements.md](0.8.28/features/slice-40/requirements.md).
- Slice 42: REQ42 and AC42-01 to AC42-12 in
  [slice-42/requirements.md](0.8.28/features/slice-42/requirements.md).

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

<!-- BEGIN GENERATED release-state:0.8.28:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 42** (`TEST-HYGIENE-PUBLISHED-CACHE`) — slice135 descriptor check independent of parallel tests; published-Rust page-cache investigation (todos ledger seq 312 and 313)

**Remaining ladder:** 42.<!-- END GENERATED release-state:0.8.28:plan-immediate-next -->

Slices 40 and 41 merged into `release/0.8.28` at `869d6a9e1` and
`0ec01bc22`. Slice 42 is next. After it come the release-level steps: the
open decisions, then the owner's publication decision (Slice 31 plan W8 and
R-CODE-4).

## Slice 42 — slice135 descriptor check and the published-Rust page cache

Placed by owner ruling on 2026-10-16 (todos ledger (`dev/todos-and-considerations-ledger.jsonl`) seq 312, `TC-f9fa3b87-6f23-427b-b027-6889e52b3bef`; todos ledger seq 313, `TC-4786ddae-2a6a-4da9-b618-3f659cfacb6e`). Branch
`slice/0.8.28-42-test-hygiene-and-published-cache`, cut from the Slice 40
branch at `d10ad65d5`. Plan:
[slice-42/plan.md](0.8.28/features/slice-42/plan.md).

- **Unit A.** `slice135_persistent_sqlite_full_preserves_state_until_cap_lifts`
  counts every open descriptor in the process, so it fails in 9 of 10
  parallel runs while its sibling tests open engines. The design counts
  only descriptors under the test's own directory: one helper and two
  changed lines, subject to owner ruling Q-A1. Done when 30 parallel runs
  pass and a deliberately leaked descriptor still fails the check.
- **Unit B.** Investigation only. `cargo install fathomdb-cli` and
  consumers of the published crates build the registry `libsqlite3-sys`,
  which keeps the shared page cache. The investigation measures what they
  get today and weighs routes other than a forked SQLite stack or waiting
  for upstream: prebuilt CLI binaries, a run-time report, documented
  persistent overrides, a system SQLite, `mmap_size`, cache size, reader
  count and a custom page cache. It ends with a recommendation for the
  owner. Nothing is implemented unless the owner rules for it.

No product code, schema, public surface or workflow changes in this
slice.

## Slice ladder

| Slice | Scope | Plan |
| ---: | --- | --- |
| 30 | Tegra private CUDA memory pool (D28-08) | [slice-30/plan.md](0.8.28/features/slice-30/plan.md) |
| 31 | cudarc and Candle forks from crates.io (D28-13) | [slice-31/plan.md](0.8.28/features/slice-31/plan.md) |
| 40 | FTS ranking, latency and page-cache defects (COMP benchmarks) | [slice-40/plan.md](0.8.28/features/slice-40/plan.md) |
| 41 | SDK context pack, full-render override, configurable query stop words (reserved; docs on `slice/0.8.28-41-sdk-evidence-helper`) | not on this branch |
| 42 | slice135 descriptor check; published-Rust page-cache investigation | [slice-42/plan.md](0.8.28/features/slice-42/plan.md) |
