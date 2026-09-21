---
status: ACTIVE
scope: post-0.8.26
updated: 2026-09-21
---

# FathomDB roadmap after 0.8.26

Start here for future work. This document consolidates the active release
schedule, draft scopes, experimental review checkpoints, and explicitly
preserved backlog into one readable map.

This is a roadmap, not implementation authority. Release placement is owned by
[`plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md`](plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md).
A release becomes executable only when it has an approved plan, release-state
file, and board. Publishing always requires a separate explicit HITL decision.

## Current position

- **0.8.26 is published.** All tracked releases are published and no release is
  currently active.
- **0.8.27 is the next named candidate release.** Its scope is proposed intake,
  not approved implementation. There is no `plan-0.8.27.md`,
  `release-state-0.8.27.json`, or `STATUS-0.8.27.md` yet.
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
| **0.8.27** | **Reported release blocker + proposed scope** | Correction-safe source erasure after supersession, followed by full opt-in snapshot and cursor continuity, generalized graph/state pagination, and persisted evidence replay. |
| **0.8.28** | **Proposed release scope** | Manually selected advanced retrieval, rich graph continuation, and expanded exclusion tracing. |
| **0.8.29** | **Review checkpoint** | Candidate-selection experiments. |
| **0.8.31** | **Review checkpoint** | Associative retrieval and automatic profile-routing experiments. |
| **0.8.33** | **Review checkpoint** | Expanded integrity, repair-planning, and release-matrix experiments. |
| **0.9.0** | Planning only | Define post-0.8.x product identity and the next major release. No implementation is scheduled by the program schedule. |
| **Post-1.0, pre-2.1** | Backlog | ANN indexing for the 100k/1M vector-latency tiers. |

## 0.8.27 proposed scope

The source of record is
[`plans/0.8.27-draft-scope.md`](plans/0.8.27-draft-scope.md). Every item must be
revalidated against the shipped 0.8.25 and 0.8.26 contracts before it enters an
executable plan.

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
0.8.27 publication and Memex 0.6.0 production cutover until FathomDB reproduces
and resolves it, or disproves it with durable evidence. Continued isolated
Memex Slice 80 development is not blocked. The required outcome is a supported,
truthfully reported erase of buckets containing superseded revisions and closed
dependents, tested for same-bucket and cross-bucket replacements without private
database APIs. The exact external characterization is recorded in Memex
`release/0.6.0` at
`dev/fathomdb/0.8.26-erase-after-supersede-gap.md` (local checkout:
`/home/coreyt/projects/memex-worktrees/release-0.6.0/dev/fathomdb/0.8.26-erase-after-supersede-gap.md`).

### Candidate work

| ID | Candidate work | Proof required before promotion |
| --- | --- | --- |
| **D27-01** | Full opt-in cross-operation frozen-snapshot leases with typed mismatch, expiry, unavailable, and drift outcomes. | Show that the compact optional frozen read cannot satisfy a concrete multi-operation consistency need. |
| **D27-02** | Opaque cursors bound to request, snapshot, projection generation, ordering, exclusive key, and expiry. | Demonstrate duplicate, omission, or authorization risk under the minimal continuation contract. |
| **D27-03** | Generalized graph pagination and richer `operational_state` continuation. | Identify a bounded consumer workload with deterministic complete ordering; ranked top-K remains separate. |
| **D27-04** | Persisted source-complete evidence receipts with eligibility-bound replay and retention/expiry behavior. | Demonstrate a need to resolve evidence beyond the compact reference lifetime without weakening current authorization. |

Constraints:

- All features remain opt-in; ordinary search does not acquire lease overhead.
- A cursor or evidence handle never reveals bytes after visibility,
  eligibility, lifecycle, or erasure changes.
- Public state remains compact and versioned.

## 0.8.28 proposed scope

The source of record is
[`plans/0.8.28-draft-scope.md`](plans/0.8.28-draft-scope.md). These are
caller-selected advanced reads; automatic routing and default changes are not
part of this scope.

| ID | Candidate work | Proof required before promotion |
| --- | --- | --- |
| **D28-01** | Manual named-profile configuration and qualification. | At least one non-A0 treatment has a reviewed, reproducible use case. |
| **D28-02** | Specialized time-scoped and changed-fact retrieval. | External-validity evidence shows improvement beyond filtering without truth inference or answer regression. |
| **D28-03** | Rich constrained-graph continuation and replayable path evidence. | A consumer needs more than the deterministic one-page result and can state bounded continuation semantics. |
| **D28-04** | Expanded deterministic exclusion and not-selected tracing. | A concrete debugging or compliance decision cannot be answered by compact inclusion/degradation output. |

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

- **0.9.0 readability and navigability:** capability-module decomposition of
  the engine, focused binding/CLI/test splits, reusable structure maps, and
  duplication cleanup. The proposal is placed for 0.9.0, but implementation is
  not scheduled. See
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
