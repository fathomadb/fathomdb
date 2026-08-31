---
title: FathomDB 0.8.25 — performance-program data-plane foldback
status: ACTIVE
target_release: 0.8.25
---

# FathomDB 0.8.25 — plan of record

## Purpose

0.8.25 converts the completed performance program's confirmed data-plane gaps
into governed FathomDB contracts while preserving the product boundary:
FathomDB owns durable mechanisms and invariants; an external semantic component
owns semantic policy, reasoning, and answers.

## Setup record

- **Merged baseline:** `main` at
  `fca10bd2dfbb59ba7446fe66e0b1a9555c4df993`, which contains the merged
  `release/0.8.24` branch.
- **Release branch:** `release/0.8.25`.
- **Release worktree:** `/tmp/fathomdb-release-0.8.25`.
- **Performance source:**
  `experiments/performance-0.8.23-plan-20260821` at
  `1fdd8142fda0c660b241b5785a52a9499e0ad2bb`.
- **History import:** merge commit `9ce9fcbd`; 0.8.24 production/release
  contracts won conflicts, experiment assets and results were retained, and
  append-only performance decisions were reallocated to ledger sequences
  259–270.
- **Local dependencies:** `node_modules` and `data` are ignored symlinks to the
  primary checkout's local resources. An ignored, worktree-local `.venv/bin`
  shim exposes only the primary environment's Python and pinned-tool launchers;
  the environment itself is not symlinked or rebound. The test-hook ownership
  guard therefore refuses native rebuilds from this worktree while ordinary
  tests and experiment tooling can use the already-installed dependencies.

## Scope

The versioned architecture is
[`fathomdb-data-plane-architecture-v1.md`](../design/fathomdb-data-plane-architecture-v1.md).
The detailed delivery method is
[`fathomdb-data-plane-foldback-v1.md`](fathomdb-data-plane-foldback-v1.md).

In scope:

- experiment measurement-layer classification;
- opt-in source-complete evidence resolution;
- constrained combined graph expansion; and
- governed native filtering and stable pagination.

Out of scope:

- an Engine-owned semantic control plane;
- extraction/entity/conflict policy, query decomposition, synthesis, answer
  generation, or semantic entailment;
- retuning rejected GLOBAL-01, GRAPH-01, or REASON-01 treatments; and
- publication, which always requires separate HITL authorization.

## Slice sequence

| Slice | Outcome | Depends on | State |
| ---: | --- | --- | --- |
| 0 | Import the committed performance program onto the 0.8.24-based release branch; reconcile ledgers and retired duplicate tests. | merged 0.8.24 main | Complete (`9ce9fcbd`) |
| 5 | Restore and finalize the reviewed architecture, boundary, learning register, and delivery plan. | 0 | Complete |
| 7 | Write requirements/AC, review design, implement with RED/GREEN, review, and verify measurement-layer classification, including the existing GLOBAL-01 paths. | 5 | Not started |
| 10 | Write requirements/AC, review design, implement with RED/GREEN, review, and verify opt-in source-complete evidence. | 7 | Not started |
| 20 | Write requirements/AC, review design, implement with RED/GREEN, review, and verify constrained combined graph expansion. | 10 | Not started |
| 30 | Write requirements/AC, review design, implement with RED/GREEN, review, and verify governed filters and stable pagination. | 10 | Not started |
| 40 | Run compact cross-workstream lifecycle/performance validation, binding parity, repository verification, and owner-ready release evidence. | 20, 30 | Not started |

Slice 5 establishes the architectural classification and preserves the
existing GLOBAL-01 scope. Slice 7 makes that rule executable in experiment
metadata and validation before any later verification report may be accepted.

## Immediate next action

Commission Slice 7 from its requirements and acceptance criteria. Do not begin
implementation from the architecture prose alone, and do not start Slice 10
until the measurement-classification contract is accepted.

## Verification and stop gates

Every feature slice uses the review and TDD limits in the foldback plan. Stop
for an unresolved P1/P2 finding, public-surface change without ADR/interface
grounding, stale or uneraseable evidence, unbounded retrieval, binding drift,
or a requirement that belongs to the external semantic component.

The expired 0.8.23 unmerged CUDA candidate was retired to the contract's empty
canonical manifest during Slice 5. Standing CUDA and ptrace authorization at
`seq-271` does not extend a candidate expiry, authorize publication, or weaken
the protected release route.
