---
status: ACTIVE
scope: post-0.8.26
updated: 2026-10-09
---

# FathomDB roadmap after 0.8.26

Start here for future work. This document consolidates the active release
schedule, draft scopes, experimental review checkpoints, and explicitly
preserved backlog into one readable map.

The [product hygiene roadmap](hygeine-roadmap.md) tracks cross-release
correctness, quality, performance and qualification risks separately from
this release schedule.

This is a roadmap, not implementation authority. Release placement is owned by
[`plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md`](plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md).
A release becomes executable only when it has an approved plan, release-state
file, and board. Publishing always requires a separate explicit HITL decision.

## Current position

- **0.8.26 is published.** It is the latest published release.
- **0.8.27 is the active implementation release** on `release/0.8.27`.
  Prework approved correction-safe erasure and a behavior-preserving semantic
  refactor. As of 2026-10-09, release state records Slice 135's bounded
  system qualification complete alongside the erasure fix (Slice 20) and the
  NAPI, TypeScript, Python and Rust SDK slices (110–132). Planned Slice 117
  (Jetson CUDA Node addon via the Tegra Pages route, gated on an unruled
  delivery-shape decision) is next, then 140 and 150. Its plan, state, and
  board are the execution authority;
  publication remains separately gated.
- An open todo does not automatically become roadmap scope. This file includes
  work only when a current schedule, draft scope, or explicit backlog/proposal
  record preserves it.

## Status vocabulary

| Status | Meaning |
| --- | --- |
| **Reported release blocker** | A reported defect assigned to a release for reproduction and disposition; it must be resolved or disproved before publication. |
| **Proposed release scope** | Candidate work assigned to a release for revalidation; not implementation authority. |
| **Review checkpoint** | A date to decide whether to run or promote an experiment; not a product commitment. |
| **Backlog** | Preserved work with no release slot. Scheduling requires an HITL decision. |
| **Parked** | Deliberately outside the release path until named evidence or an architecture decision reopens it. |

## Release map

| Release | Status | Theme |
| --- | --- | --- |
| **0.8.27** | **Active implementation; Slice 135 complete, 117 next** | Correction-safe source erasure, semantic decomposition, and bounded system qualification are recorded; planned Slice 117 adds a Jetson CUDA Node addon through the Tegra Pages route. |
| **0.8.28** | **Proposed release scope** | Advanced retrieval and continuation, Tegra CUDA memory-pool adoption, plus four owner-placed Slice 135 fault, quality, artifact and latency follow-ups. |
| **0.8.29** | **Review checkpoint** | Candidate-selection experiments. |
| **0.8.30** | Backlog | Explicit release of the process-lifetime module-level CUDA models. |
| **0.8.31** | **Review checkpoint** | Associative retrieval and automatic profile-routing experiments. |
| **0.8.33** | **Review checkpoint** | Expanded integrity, repair-planning, and release-matrix experiments. |
| **0.9.0** | Planning only | Define post-0.8.x product identity and the next major release. No implementation is scheduled by the program schedule. |
| **Post-1.0, pre-2.1** | Backlog | ANN indexing for the 100k/1M vector-latency tiers. |

## 0.8.27 scope

The source of record is
[`plans/plan-0.8.27.md`](plans/plan-0.8.27.md), with live progress on
[`plans/runs/STATUS-0.8.27.md`](plans/runs/STATUS-0.8.27.md). The original
draft intake is retained as a superseded record with every item dispositioned.

### Release-blocking finding

**F27-01 — source erasure after correction/supersession.** Memex reports that
registry FathomDB 0.8.26 cannot reliably erase a source bucket after a
revision-pinned correction supersedes the original and writes a live
replacement. `Engine.erase_source(bucket)` raises `StorageError` even when the
associated dependency closures report `complete`, leaving the original body
and other bucket records stored and retrievable.

The same-bucket case remains locked. In the cross-bucket case, erasing the
replacement bucket first can unlock the original. Memex truthfully reports
`storage_failed`; it does not claim erasure or bypass the public API.

This is a privacy-deletion and authority-boundary risk. Treat it as blocking
0.8.27 publication until FathomDB resolves it or disproves it with durable
evidence. Memex later received a narrow exemption for this exact characterized
refusal, so the finding no longer blocks its 0.6.0 cutover by itself.
Slice 20 implemented and reviewed correction-safe erasure on
`release/0.8.27`; the finding stays publication-relevant until the release's
final qualification (Slice 150) and publication decision. The
required outcome is a supported,
truthfully reported erase of buckets containing superseded revisions and closed
dependents, tested for same-bucket and cross-bucket replacements without private
database APIs. The exact external characterization is recorded in Memex
`release/0.6.0` at
`dev/fathomdb/0.8.26-erase-after-supersede-gap.md` (local checkout:
`/home/coreyt/projects/memex-worktrees/release-0.6.0/dev/fathomdb/0.8.26-erase-after-supersede-gap.md`).

### Deferred candidate work

| ID | Candidate work | Proof required before promotion |
| --- | --- | --- |
| **D27-01** | Full opt-in cross-operation frozen-snapshot leases with typed mismatch, expiry, unavailable, and drift outcomes. | Moved to 0.8.28 D28-05; proof remains unmet. |
| **D27-02** | Opaque cursors bound to request, snapshot, projection generation, ordering, exclusive key, and expiry. | Moved to 0.8.28 D28-06; proof remains unmet. |
| **D27-03** | Generalized graph pagination and richer `operational_state` continuation. | Merged into 0.8.28 D28-03; proof remains unmet. |
| **D27-04** | Persisted source-complete evidence receipts with eligibility-bound replay and retention/expiry behavior. | Moved to 0.8.28 D28-07; proof remains unmet. |

Constraints:

- All features remain opt-in; ordinary search does not acquire lease overhead.
- A cursor or evidence handle never reveals bytes after visibility,
  eligibility, lifecycle, or erasure changes.
- Public state remains compact and versioned.

## 0.8.28 proposed scope

The source of record is
[`plans/0.8.28-draft-scope.md`](plans/0.8.28-draft-scope.md). D28-01 to D28-07
are caller-selected advanced reads. D28-08 is owner-placed Tegra pool work;
D28-09 to D28-12 are the four owner-placed items from the
[product hygiene roadmap](hygeine-roadmap.md#open-risk-register).
Automatic routing is outside this scope; default changes require their own
reviewed contracts and gates.

| ID | Candidate work | Proof required before promotion |
| --- | --- | --- |
| **D28-01** | Manual named-profile configuration and qualification. | At least one non-A0 treatment has a reviewed, reproducible use case. |
| **D28-02** | Specialized time-scoped and changed-fact retrieval. | External-validity evidence shows improvement beyond filtering without truth inference or answer regression. |
| **D28-03** | Rich constrained-graph continuation, replayable path evidence, and richer `operational_state` continuation. | A consumer needs more than the deterministic one-page graph/state result and can state bounded continuation, authorization, and replay semantics for both components. |
| **D28-04** | Expanded deterministic exclusion and not-selected tracing. | A concrete debugging or compliance decision cannot be answered by compact inclusion/degradation output. |
| **D28-05** | Full opt-in cross-operation frozen-snapshot leases. | Compact frozen reads cannot meet a concrete multi-operation consistency need. |
| **D28-06** | Fully request/snapshot/projection/ordering-bound cursors. | A duplicate, omission, or authorization failure is demonstrated under minimal continuation. |
| **D28-07** | Persisted source-complete evidence replay. | A caller needs resolution beyond compact-reference lifetime without weaker authorization. |
| **D28-08** | **Owner-placed (2026-10-07):** adopt the Tegra private CUDA memory pool as the aarch64-Linux integrated-GPU default, with C7 context-reset instrumentation and an upstream cudarc pool primitive. | Requirements, acceptance criteria, ADR, TDD and an AGX Orin qualification against the study's gates, allocation correctness and release first. |
| **D28-09** | **Owner-placed (2026-10-09, Slice 135 rank 1):** qualify in-transaction erasure and WAL interruption; repair any confirmed contract violation. | Controlled fault positions, independent reopened-state and erasure oracles, truthful status and recovery. |
| **D28-10** | **Owner-placed (2026-10-09, Slice 135 rank 2):** improve qualified multi-hop evidence reachability. | Held-out support-set gain by hop count without correctness, provenance or lifecycle regression. |
| **D28-11** | **Owner-placed (2026-10-09, Slice 135 rank 3):** qualify installed CPU–CUDA and Jetson retrieval parity on 0.8.28 release artifacts. | Frozen same-query audit on real hardware; preserve 0.8.27 Slice 117/150 gates. |
| **D28-12** | **Owner-placed (2026-10-09, Slice 135 rank 7):** attribute and conditionally remediate engine vector, hybrid and populated-open latency leads. | Matched stage/CPU/queue and whole-call boundaries; rerun affected cells after a product change. |

### Prework: Tegra CUDA memory-pool study

The owner ruled on 2026-10-05 that 0.8.27 ships the aarch64-Linux synchronous
CUDA allocation fallback with early `cuInit` and no memory pool, and that
0.8.28 evaluates a pool against it. The
[study plan](plans/0.8.28/prework/tegra-cuda-memory-pool-study.md) defines the
evidence. As of 2026-10-07, Phases 0–4 are complete on branch
`llm/0.8.28-tegra-pool-study`. On the AGX Orin 64 GB the private pool created
at first use passed the allocation, release, cap and performance gates (steady
embed 0.986 of the default pool; 1.93× faster than the synchronous path).
A spot check on the corrected embedder-close fix (now on `release/0.8.27`)
confirmed the result. On 2026-10-07 the owner placed adoption in 0.8.28 as
D28-08 and ruled:

- a co-resident CUDA context reset stays unsupported and documented in 0.8.28,
  which adds instrumentation (context-id detection, a typed
  `cuda_context_lost` error, a diagnostic snapshot, and a characterization
  test) so 0.8.29 can design the survival fix from data;
- early `cuInit` at module load is the Tegra contract: FathomDB creates the
  pool only when its early `cuInit` ran, otherwise uses the synchronous path,
  and never refuses;
- an upstream cudarc PR for the pool primitive is a goal, with every vendored
  patch item tracked to an upstream status and removal path.

The [draft scope](plans/0.8.28-draft-scope.md#d28-08-tegra-private-cuda-memory-pool)
holds the full rulings.

## 0.8.30 backlog

Placed by HITL decision on 2026-10-07. This is backlog with a release slot,
not implementation authority; no plan, release-state file, or board exists.

**B30-01 — release the module-level CUDA models.** Todo
`TC-8d3c1cde-97d7-46d5-af50-9cc479da6de7`. The embedder-close fix releases the
engine-owned embedder at `close()`. The module-level batch-CLS embedder and
reranker live in process-lifetime statics instead. They hold about 17 MiB
(reranker) to 143 MiB (with batch CLS) until exit. The amount is bounded and
does not grow, but on integrated GPUs it is host memory. It also keeps a
private CUDA pool from returning to zero reserved.

- Shape: an explicit, opt-in release call in Rust, Python, and TypeScript, not
  automatic idle release. It needs clearable cells, typed errors, and an
  interface-doc or ADR update. Tests cover reload after release, concurrent
  use, and use after release.
- Prerequisite: fix the cudarc `CudaSlice` drop crash after a co-resident CUDA
  context reset first. A runtime release drops CUDA objects on the path where
  that crash occurs. Statics are never dropped at exit today.
- Revisit triggers: that fix, deployments with many processes, 8 or 16 GB
  integrated devices, or reports of memory held while idle.

## Experimental review checkpoints

The governing inventory is
[`plans/0.8.29-0.8.33-experimental-review-schedule.md`](plans/0.8.29-0.8.33-experimental-review-schedule.md).
A checkpoint may authorize a preregistered experiment, move it, or Park it.
Product promotion still requires requirements, acceptance criteria, design,
TDD, and verification.

### 0.8.29 — candidate selection

- Entity and alias expansion.
- Complementary-evidence and coverage-aware selection.
- MMR and diversity treatments beyond accepted A0.

Promotion requires a bounded family that improves held-out answer use without
correctness, groundedness, attribution, latency, lifecycle, or accepted-default
regression.

**Placed work, not a checkpoint (owner ruling 2026-10-07):** make FathomDB
survive a co-resident library resetting the CUDA primary context (todo
`TC-281155a1-4767-4379-9506-ef97b2ed51a9`). Today cudarc's `CudaSlice` drop
crashes inside `Engine::close` after such a reset. Design the fix from the
data that the 0.8.28 D28-08 instrumentation collects.

### 0.8.31 — associative retrieval and routing

- Associative PPR and graph diffusion.
- Automatic profile routing.

Routing is relevant only after associative retrieval produces a valid gain and
at least two manual profiles have been accepted with inspectable fallback.

### 0.8.33 — integrity and release expansion

- Full integrity-job orchestration.
- Sophisticated repair planning.
- Exhaustive scale-by-feature-by-CUDA matrices.

Promotion requires proof that representative checks miss a material failure or
that the additional machinery changes an operator or release decision.

## Unscheduled product backlog

### Multi-source dependency semantics

The shipped profile admits one canonical source. Multi-source and derived
dependency sets, bounded membership, prospective cycle validation, and the
Engine-known `all_required` / `any_surviving` liveness grammar remain deferred
beyond 0.8.26. Future allocation requires approval. See
[`design/fathomdb-data-plane-architecture-v2.md`](design/fathomdb-data-plane-architecture-v2.md)
and
[`design/0.8.x-after-0.8.25-design-notes.md`](design/0.8.x-after-0.8.25-design-notes.md).

### Governed-surface candidates

These four consumer-driven additions were rescued from the retired 0.8.15 plan
and remain wanted but unscheduled:

- op-store `read.state(collection, record_key?)` plus current-state list/scan
  over `operational_state`;
- touch or last-accessed support; and
- `graph.neighbors` label filtering; and
- a 50-result cap for `graph.neighbors`.

Their historical design context is in
[`plans/plan-0.8.15.md`](plans/plan-0.8.15.md). Each public addition requires a
governed-surface allowlist change and explicit sign-off.

### Independent evaluation backlog

- **EXP-C:** global-query productization remains wanted but underpowered at the
  existing corpus cap; reopening it requires a larger corpus or a different
  decision rule, not more runs of the same design.
- **EXP-D:** the larger entity-rich evaluation set remains blocked on HITL
  spend approval.
- **EXP-E:** the fork gate remains blocked on EXP-D and on D1/RAPTOR work that
  has no release allocation.
- **Fixture-scoped scale characterization:** preserve and, when commissioned,
  resume the 0.8.23 V2 runner/validator design in
  [`design/0.8.23-scale-characterization-v2.md`](design/0.8.23-scale-characterization-v2.md).

The EXP-C/D/E source context is
[`plans/plan-0.8.17.md`](plans/plan-0.8.17.md); the retired release itself is
not being revived.

### Runtime, connection, and read-performance backlog

The focused backlog in the repository-root [`ROADMAP.md`](../ROADMAP.md)
contains no approved API names or promised features.

Runtime-wide candidates:

- SQLite diagnostic-log routing;
- diagnostics-mode SQLite heap limits; and
- measured, bounded runtime page-cache provisioning.

Engine or connection candidates:

- connection busy timeouts; and
- prepared-statement cache sizing.

Bounded performance experiments:

- lookaside sizing;
- page-cache allocation;
- JSON versus f32 BLOB query-vector transport;
- Rust-side binary quantization after BLOB transport, if incrementally useful;
  and
- residual dispatch or allocation work backed by direct measurements.

### Consumer documentation gap

Document that opening a stdlib `sqlite3` connection to a database while a
FathomDB Engine is open against the same file in the same process is unsafe
because the process contains distinct SQLite library copies. Separate-process
inspection remains the safe route. This is a documentation obligation, not an
engine defect or a new release commitment.

## Longer-horizon proposals

These records preserve future work but do not override the release map.

- **Readability and navigability:** the bounded engine/binding/SDK semantic
  decomposition moved to the 0.8.27 plan. The original 0.9.0 proposal remains
  historical rationale; unrelated CLI/test splits and duplication cleanup stay
  unscheduled. See
  [`design/0.9.0-readability-refactor-proposal.md`](design/0.9.0-readability-refactor-proposal.md).
- **Multi-field and recursive-payload FTS:** multi-field FTS, per-kind
  tokenizers, and related precision controls remain deferred to at least 0.9.x;
  they are not a hidden 0.8.x commitment. See
  [`design/0.5.1x0.8.11.2/20-multifield-fts-design.md`](design/0.5.1x0.8.11.2/20-multifield-fts-design.md)
  and
  [`design/0.5.1x0.8.11.2/30-perkind-tokenizer-fts-design.md`](design/0.5.1x0.8.11.2/30-perkind-tokenizer-fts-design.md).
- **1–2M-chunk vector scaling:** the open decision is whether that scale is a
  near-term consumer need. If it is, run the E1 overhead-attribution experiment
  before considering a packed/SIMD exact-scan kernel. See
  [`design/1-2M-chunk-scaling-DECISION-BRIEF.md`](design/1-2M-chunk-scaling-DECISION-BRIEF.md).
- **ANN indexing:** tracked, not started, and explicitly targeted post-1.0 and
  before 2.1. HNSW, IVF, and DiskANN are candidates to evaluate when the slice
  opens. See [`design/ann-index-vec0.md`](design/ann-index-vec0.md).
- **Cross-release operational proposals:** CI/CD redesign, FathomDB/Memex
  roadmap alignment, memory-tier and expiry policy, and preservation-first
  worktree/branch consolidation remain proposal records. The reviewed catalog
  is [`design/document-lifecycle.json`](design/document-lifecycle.json).

## Parked work

The following work is deliberately not on a release path:

- database-owned query decomposition, synthesis, consolidation judgment,
  answer verification, or abstention;
- arbitrary dependency or liveness languages;
- mandatory frozen snapshots and unbounded browse or trace APIs;
- a public projection-work scheduler or generalized repair planner without a
  concrete operational requirement; and
- exhaustive release matrices where representative regression cells suffice.

Reconsideration requires an explicit architecture or program-direction
decision; these items must not enter a slice opportunistically.

## Maintenance rule

When placement changes, update the owning schedule, draft, or decision record
first, then update this roadmap in the same closing documentation change. Do
not use this summary to bypass release-state, plan, board, design-review, or
HITL gates.
