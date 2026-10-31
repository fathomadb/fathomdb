---
title: FathomDB 0.8.28 — Tegra CUDA pool, forked cudarc, and the vector read-path fix
status: ACTIVE
target_release: 0.8.28
execution_model: direct; no Steward or Orchestrator; run as a separate effort in parallel with 0.8.29 (owner 2026-10-31)
---

# FathomDB 0.8.28 — Tegra CUDA pool, forked cudarc, and the vector read-path fix

## Goal and scope

0.8.28 ships the owner-placed Tegra private CUDA memory pool on forked,
published crates, the COMP benchmark fixes, and the fix for the AC-013
regression that Slice 40 Unit I exposed:

- **Slice 30 (D28-08):** the private pool as the default on aarch64 Linux
  integrated GPUs, with context-loss detection and `doctor cuda-allocator`.
  Complete on `release/0.8.28`.
- **Slice 31 (D28-13):** consume `cudarc-fathomdb` and the Candle fork
  crates from crates.io, delete `third_party/cudarc-0.19.7`, and qualify
  CUDA 12.6 and 13.0. Fork work is planned, reviewed and qualified inside
  each fork, first (owner ruling Q14).
- **Slice 40:** FTS ranking, latency and page-cache defects found by the
  COMP benchmarks. Complete on `release/0.8.28`.
- **Slice 41 (`SDK-CONTEXT-PACK`, owner ruling 2026-10-16):** SDK
  context pack, full-render override, configurable query stop words.
  Complete on `release/0.8.28`.
- **Slice 42 (owner ruling 2026-10-16):** make the slice135 descriptor
  check independent of parallel tests (todos ledger (`dev/todos-and-considerations-ledger.jsonl`) seq 343, `TC-f9fa3b87-6f23-427b-b027-6889e52b3bef`), and give published
  Rust artifacts per-connection page caches (todos ledger seq 344, `TC-4786ddae-2a6a-4da9-b618-3f659cfacb6e`). Unit B was re-scoped on
  2026-10-31 from an investigation to adopting `rusqlite` 0.41.1 and
  `libsqlite3-sys` 0.39.1.
- **Slices 50 to 90 (owner rulings 2026-10-30 and 2026-10-31):** the
  AC-013 fix and the vector eligibility defect, then D28-14. The evidence
  and the rulings are in `dev/experiments/ac013-page-cache/RECOMMENDATION.md`
  and `dev/plans/0.8.28-0.8.29-slice-proposal.md` on `dev/0.8.29`. Todo
  TC-1b1f69e6 tracks it.

The other entries in the [draft scope](0.8.28-draft-scope.md) were ruled on
2026-10-31; the rulings are in `dev/plans/release-state-0.8.28.json`.

- **D28-14** is Slice 90.
- **D28-09** moves to a 0.8.29 slice.
- **D28-10 and D28-01 to D28-07** go to the 0.8.29 candidate-selection
  review checkpoint.
- **D28-11** is not run for 0.8.28. The owner accepts the risk; a check is
  placed in 0.8.30.
- **D28-12** is met for vector and hybrid latency by the AC-013 work and
  Slices 50 to 70. Populated-open latency moves to 0.8.29.

This plan indexes the slice plans, which are the working authority for
their slices.

## Execution

0.8.28 is a planned release, run as a separate effort in parallel with
0.8.29 (owner 2026-10-31).

- **Branches.** Slices 42 to 90 are cut from `dev/0.8.28` and land back
  on it.
- **Forward merges.** Each landed slice is merged forward into
  `dev/0.8.29`.
- **Ledgers.** Todos are written on `dev/0.8.29` only, so the seq numbers
  do not fork again; cite todos by TC id.
- **Release branch.** `release/0.8.28` is fast-forwarded to `dev/0.8.28`
  for qualification and publication.
- **Publication.** The `v*` tag push needs a separate owner authorization.

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

Re-read on 2026-10-31 for Slices 42 to 90, which change the persistence
format and dependencies:

- **Applies: verify dependency upgrades on supported platforms.** Slice
  42's SQLite bump runs on Linux, macOS and Windows, including the Windows
  verbatim-path case.
- **Applies: disabled-optimization tests.**
  - Slice 50 compares the bounded scan against the shipped scan on the
    same state.
  - Slice 70 compares native vector eligibility against the reference
    eligibility scan.
- **Applies: boundary-value tests.** Validity-window edges and the min/max
  sentinels for unbounded windows in the vec0 columns (Slice 70).
- **Applies, bounded:** one regression per confirmed defect, including
  the vector-arm defect.
- **Deferred:** fault injection at the erasure boundary, now that Slice 70
  touches the vector rows erasure deletes. D28-09 carries it into 0.8.29.
- **New evidence and a stale line.** A per-connection page cache exposed
  the vec0 overflow-chain cost. The roadmap's Windows VFS patch line
  becomes stale when Slice 42 retires the vendored crate; Slice 42 updates
  it.

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
  Unit B's requirements are rewritten for the re-scope.
- Slices 50 to 90: REQ and AC in each slice's `requirements.md`, written
  and reviewed before design.

`dev/acceptance.md` remains locked; these identifiers are release-local.

## Cross-cutting DoD

Every slice: owner rulings recorded in the ledger; requirements and AC,
design and a design review before code (Slices 42 to 90, owner 2026-10-31);
TDD with RED before GREEN for code, one RED commit per behaviour; a code
review before landing; independent Opus reviews at the slice plan's gates; the
full `./scripts/agent-verify.sh` gate for code changes; GPU qualification
receipts on the named hosts; `windchill3` driver and `/usr/local/cuda`
unchanged; no upstream PRs, issues or pushes.

## Reserved-gap policy

Slice numbers between the placed slices are reserved, and new slices go in
steps of 10. A slot stays empty unless a placement decision adds a slice;
it is never an implicit patch lane.

## Immediate next slice

<!-- BEGIN GENERATED release-state:0.8.28:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 42** (`TEST-HYGIENE-SQLITE-BUMP`) — slice135 descriptor check independent of parallel tests (Unit A); adopt rusqlite 0.41.1 and libsqlite3-sys 0.39.1, retire the vendored libsqlite3-sys (Unit B, re-scoped 2026-10-31)

**Remaining ladder:** 42 → 50 → 60 → 70 → 80 → 90.<!-- END GENERATED release-state:0.8.28:plan-immediate-next -->

Slices 40 and 41 merged into `release/0.8.28` at `869d6a9e1` and
`0ec01bc22`. Slice 42 is next, then Slices 50 to 90. After them come
release qualification (the `AGENT_LONG` perf gates, eu7, the
feature-complete gate and installed-wheel smoke tests), then the owner's
publication decision (Slice 31 plan W8 and R-CODE-4).

## Slice 42 — slice135 descriptor check and the published-Rust page cache

Placed by owner ruling on 2026-10-16 (todos ledger (`dev/todos-and-considerations-ledger.jsonl`) seq 343, `TC-f9fa3b87-6f23-427b-b027-6889e52b3bef`; todos ledger seq 344, `TC-4786ddae-2a6a-4da9-b618-3f659cfacb6e`). Branch
`slice/0.8.28-42-test-hygiene-and-published-cache`, cut from the Slice 40
branch at `d10ad65d5`. Plan:
[slice-42/plan.md](0.8.28/features/slice-42/plan.md).

- **Unit A.** `slice135_persistent_sqlite_full_preserves_state_until_cap_lifts`
  counts every open descriptor in the process, so it fails in 9 of 10
  parallel runs while its sibling tests open engines. The design counts
  only descriptors under the test's own directory: one helper and two
  changed lines, subject to owner ruling Q-A1. Done when 30 parallel runs
  pass and a deliberately leaked descriptor still fails the check.
- **Unit B, re-scoped on 2026-10-31.** `libsqlite3-sys` 0.39.1
  (`rusqlite` 0.41.1) bundles SQLite 3.54.1 and no longer sets
  `SQLITE_ENABLE_MEMORY_MANAGEMENT`. SQLite 3.54.1 also includes the
  3.53.3 Windows WAL path fix, so both exit conditions in
  `third_party/libsqlite3-sys-0.38.1/FATHOMDB-PATCH.md` are met. Unit B
  makes these changes:
  - adopt `rusqlite` 0.41.1;
  - delete the vendored crate and its `[patch.crates-io]` entry;
  - rework the `FATHOMDB_WIN_VERBATIM_SHM_SAFE` guard in
    `connection_runtime.rs` to check the SQLite version;
  - update the quality roadmap's patch line.

  Published crates and `cargo install` then get per-connection caches
  without a forked SQLite.
  - **Risks to check:** `rusqlite` 0.41 API changes; sqlite-vec 0.1.9 on
    SQLite 3.54.1; the minimum supported Rust version; and the Windows
    WAL path on the local Windows VM.
  - **Verification:** the full gate, the Unit I tests (AC40-95 to AC40-97)
    built from a scratch consumer of the registry crates, the Windows
    verbatim-path tests, and the `AGENT_LONG` perf gates as a new
    baseline.

Slice 42 lands first, so Slices 50 to 90 measure on the SQLite that ships.
The slice plan, requirements and test plan are revised for Unit B before
work starts.

## Slices 50 to 90 — the vector read-path fix

Each slice runs requirements and AC, design, a design review, TDD and a
code review. Every slice must keep public results identical unless its
requirements say otherwise.

- **Slice 50 — bound the pending-work scans (`DRAIN-SCAN-BOUND`).**
  - Each `UNION ALL` arm of `next_pending_projection_jobs` and
    `pending_projection_candidate_page`
    in `projection_worker.rs` gets its own `ORDER BY write_cursor LIMIT`.
    That makes drain linear in the backlog: at n=100000 the held drain
    falls from 33 s to 5.7 s (`exp/0.8.29-ac013-scale-drain` `aa30085fc`).
  - It also fixes the drain that never finishes on a dense-disabled store
    with pending vector rows (todo 289 on `dev/0.8.29`), which is in the
    same queries.
  - **Verification:** a query-plan test; drain cost per vector flat from
    10k to 100k; projected stores identical to the shipped scan; a node
    and edge ordering case; the drain and readiness suites.
- **Slice 60 — store format 35 (`STORE-FORMAT-35`).**
  - **Changes:** 64-row vec0 chunks (`chunk_size=64`); `search_index`
    rowid = `write_cursor`; a version-34 store is refused at open
    (`IncompatibleSchemaVersion`, with no upgrade path); the rebuild verbs
    and vector re-centring on the new layout.
  - **Long-record guard, storage half (B2-8, owner 2026-10-31).** A
    vector-span table, one row per vector: `write_cursor`, window number,
    byte start, byte end, and the record's body length. Format 35 fills
    only the head window. The `Embedder` trait gains a method that reports
    where it truncated, defaulting to unknown. The table is shaped for
    overflow chunking, so enabling chunking later re-projects long records
    without another format break. The `orient` and snippet reporting is
    0.8.29 Block 2.
  - **Before the format is frozen:**
    - a benchmark of the vector table alone to 2M rows (R8 option A), to
      confirm 64 rows against 128 and 256. Include row counts multiplied
      by overflow chunking (about 5.1 vectors per record on LongMemEval
      sessions), because chunking would reach CS64's crossing near 1.2M
      rows at about 235k records;
    - eu7 recall with bge on the RTX 3090.
  - **Chunking stays parked.** The re-evaluation record is
    `dev/design/overflow-chunking-reevaluation.md` on `dev/0.8.29`.
  - **ADR:** a successor to ADR-0.7.0-vector-binary-quant.
  - **Verification:** AC-013 (p50 about 63 ms, limit 80), `ac_013b`, the
    refusal test, the shape-sentinel and rebuild tests, and identical
    results.
  - Closes todo 306 (TC-016ff172).
- **Slice 70 — vector eligibility inside vec0 (`VECTOR-ELIGIBILITY-NATIVE`).**
  - **Columns:** `lifecycle`, `valid_from` and `valid_until` metadata
    columns, written at insert. They are kept current wherever lifecycle
    changes: `write_commit.rs`, `record_lifecycle.rs`,
    `dependency_closure.rs` and `consolidation.rs`.
  - **Keyed by record.** Every lifecycle update and every erasure applies
    to all vector rows of the record (`write_cursor`), not to one vector
    row, so overflow chunks can be added later without new upkeep paths.
  - **Filter:** the KNN filters on them. sqlite-vec has no `IS NULL` and
    no `OR`, so an unbounded window uses min/max sentinels.
  - **The scan goes:** the store-wide vector-row eligibility scan is
    removed.
  - **What it fixes:** today any superseded, inactive or out-of-window
    vector-bearing node turns the vector arm off. In a 10k store with 10
    supersessions, 500 of 500 searches fell back to text.
  - **Cost:** the vec0 attribute budget drops from 13 to 10.
  - **Verification:** a RED test per cause; the Slice 35 eligibility
    suites; native filtering compared with the reference scan; identical
    results on clean stores.
- **Slice 80 — search view flags (`SEARCH-VIEW-FLAGS`).**
  - `include_superseded` and `include_inactive` are honoured on search in
    Rust, Python and TypeScript. Edges keep their vector rows on
    supersession.
  - The interface docs change, replacing the "refused" rule in
    `dev/interfaces/rust.md`.
  - MCP exposure is 0.8.29 Slice 60, because `fathomdb-mcp` is not on this
    branch.
  - **Verification:** defaults unchanged; old versions are returned only
    when asked for; binding parity; the Slice 15 stale-body tests still
    pass.
- **Slice 90 — filtered direct body FTS (D28-14).** It follows the
  [draft scope](0.8.28-draft-scope.md#d28-14-filtered-direct-body-fts).
  It shares Slice 80's public search-signature changes in all three
  bindings.

## Slice ladder

| Slice | Scope | Plan |
| ---: | --- | --- |
| 30 | Tegra private CUDA memory pool (D28-08) | [slice-30/plan.md](0.8.28/features/slice-30/plan.md) |
| 31 | cudarc and Candle forks from crates.io (D28-13) | [slice-31/plan.md](0.8.28/features/slice-31/plan.md) |
| 40 | FTS ranking, latency and page-cache defects (COMP benchmarks) | [slice-40/plan.md](0.8.28/features/slice-40/plan.md) |
| 41 | SDK context pack, full-render override, configurable query stop words (reserved; docs on `slice/0.8.28-41-sdk-evidence-helper`) | not on this branch |
| 42 | slice135 descriptor check; `rusqlite` 0.41.1 and `libsqlite3-sys` 0.39.1 (Unit B re-scoped) | [slice-42/plan.md](0.8.28/features/slice-42/plan.md) |
| 50 | Bounded pending-work scans; dense-disabled drain fix | to be written |
| 60 | Store format 35 (64-row chunks, `search_index` rowid, refusal at open) | to be written |
| 70 | Vector eligibility inside vec0 | to be written |
| 80 | `include_superseded` and `include_inactive` on search | to be written |
| 90 | D28-14 filtered direct body FTS | to be written |
