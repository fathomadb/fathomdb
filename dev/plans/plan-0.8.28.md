---
title: FathomDB 0.8.28 — Tegra CUDA pool, forked cudarc, and the vector read-path fix
status: ACTIVE
target_release: 0.8.28
execution_model: direct; no Steward or Orchestrator; Slices 42 to 90 run serially before 0.8.29 starts; 0.8.29 starts when 0.8.28 enters final verification (owner 2026-10-31)
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

- **Slice 81 (owner 2026-11-03):** the bounded erasure-availability POC follows
  Slice 80 and precedes Slice 90. It compares eager and incremental cleanup,
  conditional staged replacement, foreground availability and durable deletion
  SLA feasibility; production remediation is not included. See the
  [Slice 81 package](0.8.28/features/slice-81/README.md).
- **Slices 82 and 83 (owner approved 2026-11-03):** CH1 Phase 1 repairs physical
  correctness/native cost; Phase 2 qualifies resumable cleanup/availability.
  Both precede Slice 90. The guarantee/API decision belongs to HITL at the end
  of 83; see the [approved plan](ch1-production-erasure-plan.md).

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

0.8.28 is a planned release. Its slices run serially and finish before
0.8.29 work starts (owner 2026-10-31, revising the earlier parallel plan).

- **Branches.** Slices 42 to 90 are cut from `dev/0.8.28` and land back
  on it, one at a time.
- **Hand-over to 0.8.29.** When Slice 90 lands and 0.8.28 enters final
  verification (the platform matrix on Windows, Linux, Tegra and macOS,
  CI, the registry dry runs and publication), 0.8.29 work starts. It does
  not wait for verification or publication to finish.
- **Forward merges.** `dev/0.8.28` is merged into `dev/0.8.29` at the
  hand-over, and again for any fix that final verification lands.
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
- Slice 42: REQ42 and AC42-01 to AC42-16 in
  [slice-42/requirements.md](0.8.28/features/slice-42/requirements.md),
  rewritten for the Unit B re-scope.
- Slices 50 to 90: REQ and AC in each slice's `requirements.md`, written
  and reviewed before design.

- Slice 92: S92-1–8 in the [documentation audit/convergence plan](0.8.28/features/slice-92/plan.md).
- Slice 95: S95-1–11 in the [CI/release-readiness plan](0.8.28/features/slice-95/plan.md),
  including the comparative 0.8.26/0.8.27 robustness review and S95-CI1–CI7 repairs.

`dev/acceptance.md` remains locked; these identifiers are release-local.

## Cross-cutting DoD

Every slice: owner rulings recorded in the ledger; requirements and AC,
design and a design review before code (Slices 42 to 90, owner 2026-10-31);
TDD with RED before GREEN for code, one RED commit per behaviour; a code
review before landing; independent reviews as specified by each slice plan
(Slices 92 and 95 use Astra); the
full `./scripts/agent-verify.sh` gate for code changes; GPU qualification
receipts on the named hosts; `windchill3` driver and `/usr/local/cuda`
unchanged; no upstream PRs, issues or pushes.

## Reserved-gap policy

Slice numbers between the placed slices are reserved, and new slices go in
steps of 10. A slot stays empty unless a placement decision adds a slice;
it is never an implicit patch lane.
Owner placement on 2026-11-03 explicitly assigns reserved slot 81 to the erasure
availability POC; the subsequent approved CH1 allocation assigns slots 82 and 83.
Owner placement on 2026-11-04 also assigns slot 92 to documentation convergence
before Slice 95. These explicit placements do not change this policy for other gaps.

## Immediate next slice

<!-- BEGIN GENERATED release-state:0.8.28:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 92** (`DOCUMENTATION-CONVERGENCE`) — Code-grounded user, contributor and project documentation audit and convergence

**Remaining ladder:** 92 → 95.<!-- END GENERATED release-state:0.8.28:plan-immediate-next -->

Product implementation completed through Slice 90 before the documentation
convergence allocated to [Slice 92](0.8.28/features/slice-92/plan.md).
[Slice 95](0.8.28/features/slice-95/plan.md) owns CI repairs and final release
qualification. Publication remains held; the existing 0.8.29 hand-over rule
applies when final verification begins. Documentation convergence alone does
not start that hand-over.

Slices 40 and 41 merged into `release/0.8.28` at `869d6a9e1` and
`0ec01bc22`; Slice 42 landed on `dev/0.8.28` (code head `fdbb74e10`).
Slices 60, 70 and 80 are complete on `dev/0.8.28` and `release/0.8.28`.
Slices 81–84, including 84b, are complete. Both cleanup optimization candidates
were rejected after failed service qualification; baseline production is retained.
Slice 85 completed the approved explicit-maintenance API and format 36 at
`a2fb4a647`, with Astra review, independent qualification and accepted observed
timings. [Status](0.8.28/features/slice-85/status.md) records assembled checks and
hosted CI infrastructure limits. Slice 90 completed filtered direct body FTS;
its [status](0.8.28/features/slice-90/status.md) records reviewed semantics and
verification. [Slice 95](0.8.28/features/slice-95/plan.md) now owns the CI
setup/disk/consumer-smoke repairs, evidence audit and final release qualification
(the applicable `AGENT_LONG` perf gates, eu7, feature-complete gate and
installed-artifact smoke tests), followed by the separate owner publication
decision (Slice 31 plan W8 and R-CODE-4).

## Erasure sequence disposition and approved contract

Owner 2026-11-04 set aside further cleanup optimization after the completed
84/84b negative investigations. The [full synthesis](../notes/erasure-optimization-sequence-findings.md)
records Slices 81–84b. A separately reviewed [explicit maintenance contract/API
contract](../design/explicit-maintenance-erasure/README.md) was approved by HITL
on 2026-11-04 for a separate clean [Slice 85](0.8.28/features/slice-85/plan.md).
That resolves the original end-of-83 decision. No deadline, immediate revocation
subsystem or renewed optimization campaign is approved. The concrete design and
independent bounded qualification completed in Slice 85; current schema is 36.
Physical erasure is done except for concrete defects (owner 2026-11-04); it must
not dominate further release planning. Hosted CI setup/disk/legacy-smoke repairs
are assigned to Slice 95 as CI work, with no platform-green or publication claim.

## Slice 42 — slice135 descriptor check and the published-Rust page cache

Placed by owner ruling on 2026-10-16 (todos ledger (`dev/todos-and-considerations-ledger.jsonl`) seq 343, `TC-f9fa3b87-6f23-427b-b027-6889e52b3bef`; todos ledger seq 344, `TC-4786ddae-2a6a-4da9-b618-3f659cfacb6e`). Branch
`slice/0.8.28-42-test-hygiene-and-published-cache`, fast-forwarded to
`dev/0.8.28` at `59079d5c5`. Plan:
[slice-42/plan.md](0.8.28/features/slice-42/plan.md).

- **Unit A.** `slice135_persistent_sqlite_full_preserves_state_until_cap_lifts`
  counts every open descriptor in the process, so it fails in 9 of 10
  parallel runs while its sibling tests open engines. The design counts
  only descriptors under the test's own directory: one helper and two
  changed lines, as owner ruling Q-A1 approved. Done when 30 parallel runs
  pass and a deliberately leaked descriptor still fails the check.
- **Unit B, re-scoped on 2026-10-31.** `libsqlite3-sys` 0.39.1
  (`rusqlite` 0.41.1) bundles SQLite 3.54.1 and no longer sets
  `SQLITE_ENABLE_MEMORY_MANAGEMENT`. SQLite 3.54.1 also includes the
  3.53.3 Windows WAL path fix, so the exit conditions in the retired patch
  note (git history at `59079d5c5`) are met. Unit B
  makes these changes:
  - adopt `rusqlite` 0.41.1;
  - delete the vendored crate and its `[patch.crates-io]` entry;
  - rework the `FATHOMDB_WIN_VERBATIM_SHM_SAFE` guard in
    `connection_runtime.rs` to check the SQLite version;
  - update the quality roadmap's patch line.

  Published crates and `cargo install` then get per-connection caches
  without a forked SQLite.
  - **Risks checked in phase 1 (2026-10-31):** `rusqlite` 0.41 makes
    `trace_v2` and `busy_handler` take `&mut self` and generic
    callbacks, which breaks five test-hook calls and three test calls;
    sqlite-vec 0.1.9 passed the engine and schema suites on SQLite
    3.54.1 in a probe; the minimum supported Rust version is unchanged
    (the new crates need 1.89 and 1.88); the Windows WAL path is
    checked on the local Windows VM.
  - **Verification:** the full gate, the Unit I tests (AC40-95 to AC40-97)
    built from a scratch consumer of the registry crates, the Windows
    verbatim-path tests, and the `AGENT_LONG` perf gates as a new
    baseline.

Slice 42 lands first, so Slices 50 to 90 measure on the SQLite that ships.
The slice documents were rewritten for the re-scope on 2026-10-31; a
design review comes before any code.

## Slices 50 to 90 — the vector read-path fix

Each slice runs requirements and AC, design, a design review, TDD and a
code review. Every slice must keep public results identical unless its
requirements say otherwise.

- **Slice 50 — bound the pending-work scans (`DRAIN-SCAN-BOUND`).**
  - Each `UNION ALL` arm of `next_pending_projection_jobs` and
    `pending_projection_candidate_page`
    in `projection_worker.rs` gets its own `ORDER BY write_cursor LIMIT`.
    That removes repeated full-backlog sorting: in the prototype at
    n=100000 the held drain falls from 33 s to 5.7 s (`exp/0.8.29-ac013-scale-drain` `aa30085fc`).
    That figure is for 64-row chunks (Slice 60). On format 34 the slice
    measures linearity under a rule it states before the run.
  - The provider subunit is complete at `630af0bfc`: actual verification
    refusal fails open without returning a dense-disabled Engine. Historical
    D50-1/D50-2 drain proposals are unselected. Remaining work is scan Unit S;
    see the [reconciled slice plan](0.8.28/features/slice-50/plan.md).
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
  - **Qualified and frozen:** all 30 vector-only 64/128/256 cells through
    2M rows passed the decision rule; CS64 wins all ten size/dimension pairs.
    AC-013 p50/p99 is 59/67 ms. Candidate-bound TC-5 GPU recall is 0.958,
    CI [0.938, 0.974], meeting the unchanged criterion. Requested legacy
    EU7 failures remain diagnostic evidence; `ac_013b` is report-only.
    The historical comparison with 1024-row chunks was not remeasured.
  - **Chunking stays parked.** The storage identity supports later windows;
    allocation, grouping and lifecycle behavior remain follow-on work.
  - **Accepted ADR and qualification:**
    [Slice 60 status](0.8.28/features/slice-60/status.md), including scale,
    cache/concurrency caveats, cutoff ties and independent verification.
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
  - **Fixed:** previously any superseded, inactive or out-of-window
    vector-bearing node turned the vector arm off. In a 10k store with 10
    supersessions, 500 of 500 searches fell back to text.
  - **Cost:** the vec0 attribute budget is ten.
  - **Verification:** a RED test per cause; the Slice 35 eligibility
    suites; native filtering compared with the reference scan; identical
    results on clean stores. Astra reviews, 64 independent focused tests and
    the scoped gate campaign pass; AC-013 p50/p99 is 55/63 ms.
    [Slice 70 status](0.8.28/features/slice-70/status.md).
- **Slice 80 — search view flags (`SEARCH-VIEW-FLAGS`) — COMPLETE.**
  - Independent existence flags work across Rust, Python and TypeScript
    search paths. Historical edge membership covers queued/in-flight work,
    both supersession keys, consolidation, reopen and rebuild. Partial tuples
    refuse; incomplete dense history reports explicit fallback.
  - Relaxed property FTS uses a private memory-only corpus per call and one
    held snapshot, with existing statistical baseline plus authorized history.
    No-history property ranking parity, erasure/reuse/close behavior, source
    authorization and frozen repair visibility are tested. Resource costs and
    retained-edge BM25 effects are explicit; default eligibility is unchanged.
  - Astra review and independent qualification are recorded in
    [Slice 80 status](0.8.28/features/slice-80/status.md), with
    [requirements](0.8.28/features/slice-80/plan.md) and
    [lifecycle design](0.8.28/features/slice-80/design.md).
  - MCP exposure remains 0.8.29 Slice 60.
- **Slice 81 — erasure availability POC (`ERASURE-AVAILABILITY-POC`).**
  - Owner placed this bounded mechanism experiment on 2026-11-03. It compares
    eager versus incremental physical cleanup; staged replacement is conditional.
  - Complete: reconciled requirements/AC, reviewed design, TDD/fault tests and
    bounded measurements are in the [slice package](0.8.28/features/slice-81/README.md).
    See [results](../experiments/erasure-availability/RESULTS.md) for qualified
    comparisons, incomplete attempts and the separate native timing benchmark.
  - The POC changed no production erasure semantics. Its findings feed approved
    CH1 Slices 82/83; the physical erasure finding remains open pending repair.
- **Slice 82 — CH1 Phase 1 (`CH1-PHYSICAL-ERASURE`).** Physical correctness and
  native cost, durable sanitation and retained ghosts; conservative admission.
  [Phase plan](0.8.28/features/slice-82/plan.md), acceptance CH1-1A–1F.
- **Slice 83 — CH1 Phase 2 (`CH1-ERASURE-AVAILABILITY`).** Resumable native work,
  safe denial/publication, restart and unrelated-operation progress.
  [Phase plan](0.8.28/features/slice-83/plan.md), acceptance CH1-2A–2F.
  End-of-slice HITL decision: physical-erasure guarantee and possible cleanup API;
  no new API or numerical SLA is selected by allocation.
- **Slice 84 — cleanup contention.** Preserve the Slice 83 physical-success
  contract while measuring and reducing cleanup contention.
  [Plan](0.8.28/features/slice-84/plan.md), local acceptance S84-1–7.
- **Slice 85 — explicit erasure maintenance (`EXPLICIT-ERASURE-MAINTENANCE`).**
  Approved opt-in manual policy, bounded queued intent, exclusive maintenance
  handle, typed complete/incomplete outcomes and fresh-store format revision.
  [Plan](0.8.28/features/slice-85/plan.md), local acceptance S85-1–8.
  Complete with Astra review and independent qualification; see
  [status](0.8.28/features/slice-85/status.md). Physical erasure is done except
  for defects; no new optimization campaign is planned.
- **Slice 90 — filtered direct body FTS (D28-14).** It follows the
  [draft scope](0.8.28-draft-scope.md#d28-14-filtered-direct-body-fts).
  Completed with [reviewed plan/design](0.8.28/features/slice-90/plan.md) and
  [status](0.8.28/features/slice-90/status.md), building on Slice 80 view admission
  across Rust, Python and TypeScript.

- **Slice 92 — documentation audit and convergence.** Owner placement on
  2026-11-04 (ledger seq-365), after 90 and before 95. Audit and update user
  guidance, contributor architecture/ADRs/designs/code documentation and project
  hygiene against accepted authority and implementation. Reuse current catalogs,
  generated navigation and docs checks; correct the pending format-36 truth
  checker through RED/GREEN. [Plan](0.8.28/features/slice-92/plan.md), S92-1–8;
  [status](0.8.28/features/slice-92/status.md). Planning only.
- **Slice 95 — CI completeness and final release readiness.** Owner placement
  on 2026-11-04 (steward ledger seq-364), initially after 90; now depends on 92.
  Slice 92 owns documentation convergence; 95 checks its baseline and later
  repair deltas during integrated qualification.
  Explicitly repairs cold verify pinned-tool setup and parallel Rust disk
  exhaustion, plus the observed Cargo/ShellCheck/preflight-evidence failures and manual empty-diff routing gap.
  Preserve routing/tier/diagnostic controls, supported-platform receipts and live
  feature coverage; distinguish ordinary CI from non-publishing release rehearsal.
  Audit documentation and applicable final candidate evidence. Physical erasure stays
  closed except for defects. [Plan](0.8.28/features/slice-95/plan.md) and
  [planned status](0.8.28/features/slice-95/status.md); no execution/publication
  result is claimed.

## Slice ladder

| Slice | Scope | Plan |
| ---: | --- | --- |
| 30 | Tegra private CUDA memory pool (D28-08) | [slice-30/plan.md](0.8.28/features/slice-30/plan.md) |
| 31 | cudarc and Candle forks from crates.io (D28-13) | [slice-31/plan.md](0.8.28/features/slice-31/plan.md) |
| 40 | FTS ranking, latency and page-cache defects (COMP benchmarks) | [slice-40/plan.md](0.8.28/features/slice-40/plan.md) |
| 41 | SDK context pack, full-render override, configurable query stop words (reserved; docs on `slice/0.8.28-41-sdk-evidence-helper`) | not on this branch |
| 42 | slice135 descriptor check; `rusqlite` 0.41.1 and `libsqlite3-sys` 0.39.1 (Unit B re-scoped) | [slice-42/plan.md](0.8.28/features/slice-42/plan.md) |
| 50 | Bounded pending-work scans; provider contract complete | [Plan](0.8.28/features/slice-50/plan.md) |
| 60 | Store format 35 (64-row chunks, `search_index` rowid, refusal at open) | [Plan](0.8.28/features/slice-60/plan.md) |
| 70 | Vector eligibility inside vec0 | [Plan](0.8.28/features/slice-70/plan.md) |
| 80 | `include_superseded` and `include_inactive` on search | [Plan](0.8.28/features/slice-80/plan.md) |
| 81 | Erasure availability POC and deletion SLA feasibility | [Package](0.8.28/features/slice-81/README.md) |
| 82 | CH1 Phase 1: physical correctness and native cost | [Plan](0.8.28/features/slice-82/plan.md) |
| 83 | CH1 Phase 2: native resumability/availability; guarantee/API decision at end | [Plan](0.8.28/features/slice-83/plan.md) |
| 84 | Cleanup contention and certification cost | [Plan](0.8.28/features/slice-84/plan.md) |
| 85 | Explicit physical-erasure maintenance and format boundary | [Plan](0.8.28/features/slice-85/plan.md) |
| 90 | D28-14 filtered direct body FTS | [Plan and design](0.8.28/features/slice-90/plan.md) |
| 92 | User, contributor and project documentation audit/convergence | [Plan](0.8.28/features/slice-92/plan.md) |
| 95 | CI completeness, runner repairs and final release readiness | [Plan](0.8.28/features/slice-95/plan.md) |
