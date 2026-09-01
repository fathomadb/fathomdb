---
title: FathomDB 0.8.25 — governed data-plane foldback
status: ACTIVE
target_release: 0.8.25
---

# FathomDB 0.8.25 — proposed release plan

## Goals and scope

0.8.25 converts the completed performance program and the complete Memex 0.6.0
consumer-needs inventory into governed FathomDB data-plane contracts. FathomDB
owns durable mechanisms and invariants. An external semantic component owns
semantic policy, reasoning, and answers.

The requirements crosswalk is
[`memex-0.6.0-needs-in-fathomdb-0.8.25.md`](memex-0.6.0-needs-in-fathomdb-0.8.25.md).
The prework contract is
[`0.8.25-prework-slices-0-7.md`](0.8.25-prework-slices-0-7.md).
The successor architecture and delivery plan are
[`fathomdb-data-plane-architecture-v2.md`](../design/fathomdb-data-plane-architecture-v2.md)
and [`fathomdb-data-plane-foldback-v2.md`](fathomdb-data-plane-foldback-v2.md).
They remain proposed inputs until Slice 4 architecture review, Slice 5
verification review, and Slice 6 HITL decisions are complete.

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
- **Main reconciliation:** post-cut `main` commit `4fc1b890`, which retires the
  expired CUDA candidate, is included by merge commit `3e9f4673`; it produced
  no tree delta against the already-corrected release branch.
- **Local dependencies:** `node_modules` and `data` are ignored symlinks to the
  primary checkout's local resources. An ignored, worktree-local `.venv/bin`
  shim exposes the primary environment's Python and pinned-tool launchers but
  does not rebind the environment to this worktree. This is adequate for
  planning-only checks, not native rebuild verification. Slice 0 must choose an
  isolated release-bound build/test arrangement that neither rebuilds against
  the wrong source nor pollutes the primary `main` checkout.
- **Allocation verification:** `./scripts/agent-verify.sh` passed all 103
  executed suites with one intentional TypeScript skip. The strict ptrace and
  egress checks ran unchanged; 1,251 Python tests passed with 32 explicit
  test-hook/integration skips in the focused environment check.

## Scope

In scope:

- executable measurement-layer classification;
- durable record-revision and source-version provenance;
- explicit single- and multi-source dependencies;
- atomic application of caller-decided semantic mutations and complete
  receipts;
- dependency-aware lifecycle, erasure, and integrity closure;
- frozen read snapshots and eligibility-before-ranking predicates;
- projection generation identity and readiness correlation;
- stable ordered pagination and governed `latest_state` reads;
- opt-in source-complete evidence and structural explanation;
- constrained combined graph expansion;
- benchmark-gated deterministic candidate-selection primitives; and
- cross-SDK, concurrency, lifecycle, and performance verification.

Out of scope:

- an Engine-owned semantic control plane;
- extraction, entity resolution, contradiction, truth, ontology, query intent,
  decomposition, synthesis, answer generation, semantic verification, model
  choice, spend, or HITL policy;
- adoption of a benchmark treatment that fails its registered boundary; and
- publication, which always requires separate HITL authorization.

## Slice ladder

Prework runs sequentially through Slices 0–7. Feature work starts at Slice 10.
Every later slice depends only on earlier slices; no slice number hides a
backward dependency.

| Slice | Outcome | Depends on | State |
| ---: | --- | --- | --- |
| 0 | Identify environment and project-infrastructure needs; establish the isolated release branch/worktree, release-state/board authority, and overall plan. | merged 0.8.24 main | Complete (`321ca576`) |
| 1 | Inspect Dependabot and perform a read-only library/pinning sweep; enumerate and plan dependency responses without upgrading. | 0 | Complete (`51043e20`) |
| 2 | Perform a repo-wide cruft review and propose keep, deprecate-in-place, archive-in-place, or delete without taking action. | 1 | Complete (`51043e20`) |
| 3 | Draft product-needs, requirements, acceptance-criteria, and architecture CRUD changes; allocate each draft to an implementation slice. | 2 | Complete (`51043e20`) |
| 4 | Review proposed architecture against Slices 0–3 and high-level code alignment; write change proposals only. | 3 | Complete (`51043e20`) |
| 5 | Review verification adequacy from needs through requirements, acceptance criteria, tests, critical paths, and release goals. | 4 | Complete (`51043e20`) |
| 6 | Consolidate and score proposals, conduct interactive HITL decisions, write/review the Slice 7 plan, and update the release plan. | 5 | Awaiting initial HITL |
| 7 | Implement only HITL-approved repository-preparation work from Slices 0–6 with TDD/review/independent verification; write status and clean isolated worktrees. | 6 | Not started |
| 10 | Make measurement-layer classification executable, including whether `Engine.search` ran and which compared components differed. | 7 | Not started |
| 15 | Add immutable record-revision identity, caller source-version identity, exact source locators, canonical hashes, and missing Rust-facade identity exports. | 10 | Not started |
| 20 | Add queryable canonical-to-derived, derived-to-derived, and multi-source dependency registration with caller-declared liveness rules. | 15 | Not started |
| 25 | Add atomic caller-decided semantic actuation, model-free consolidation application, idempotent operation identity, and complete mutation receipts. | 20 | Not started |
| 30 | Close lifecycle and erasure across registered dependencies with visibility fencing, idempotent propagation, and no-active-orphan proof. | 25 | Not started |
| 35 | Add an Engine-minted frozen read snapshot and uniform indexed eligibility predicates before lexical, vector, or graph truncation. | 30 | Not started |
| 40 | Add durable projection-generation identity and correlate mutation-to-ready, degraded, blocked, and deferred states. | 35 | Not started |
| 45 | Add opaque ordered pagination for canonical list, graph, and current-state reads; expose governed `latest_state` point and page reads. | 40 | Not started |
| 50 | Add opt-in source-complete evidence resolution bound to the originating snapshot and eligibility envelope. | 45 | Not started |
| 55 | Add backward/forward provenance tracing, inclusion/exclusion explanation, receipt correlation, dependency-orphan checks, and governed operator maintenance. | 50 | Not started |
| 60 | Add constrained combined graph expansion with typed direction, edge/target constraints, predicates, bounded continuation, and exact path evidence. | 55 | Not started |
| 65 | Qualify deterministic entity/alias, duplicate, diversity, complementary-evidence, coverage, and candidate-fusion primitives. | 60 | Not started |
| 70 | Qualify temporal and associative/graph-diffusion retrieval primitives without changing defaults absent an accepted benchmark. | 65 | Not started |
| 75 | Run cross-SDK/wire parity, snapshot concurrency, lifecycle, cold/steady performance, resource, and retrieval-only evaluation closure. | 70 | Not started |

## Requirements and acceptance criteria

Prework requirements are defined by
[`0.8.25-prework-slices-0-7.md`](0.8.25-prework-slices-0-7.md): Slices 0–5
produce evidence and proposals, Slice 6 records HITL decisions and a reviewed
Slice 7 plan, and Slice 7 implements only approved repository preparation.

Feature requirements and slice-local falsifiable acceptance criteria are
written by their owning Slice 10+ plan before implementation. The complete
consumer-needs allocation is
[`memex-0.6.0-needs-in-fathomdb-0.8.25.md`](memex-0.6.0-needs-in-fathomdb-0.8.25.md).
No feature is accepted merely because it appears in this release overview.

## Cross-cutting DoD

Each feature slice writes numbered requirements and slice-local falsifiable
acceptance criteria, then a design grounded in the reviewed architecture. An
independent design review allows at most three documented FIX-n cycles.
Implementation follows TDD RED/GREEN, then an independent implementation
review allows at most four documented FIX-n cycles, subject to the standing
same-failure retry stop. Focused, lifecycle, parity, and repository verification
must pass before closure.

Do not mint global `dev/acceptance.md` identifiers for these features without
separate authorization. Historical benchmark receipts remain evidence, not
generated test oracles.

## Reserved-gap policy

Every finding remains in its owning prework register or feature-slice plan until
the Slice 6 HITL session explicitly includes, postpones, or rejects it. Slice 7
cannot absorb Slice 10+ work. A failed benchmark treatment remains durable
negative evidence and does not become a default. Deferred work names its target
release or backlog authority; it is never silently dropped.

## Workload checkpoint

Slice 6 records every item before any scope reduction. It scores understanding,
risk, effort, and release disposition, then collects explicit HITL decisions.
The release may be overweight; evaluate that only after the prework findings
and feature-slice plans are complete. No item silently moves out of 0.8.25, and
completed performance tracks are not reopened by this planning work.

The complete scored decision package is
[`slice-6-proposal-register.md`](0.8.25/prework/slice-6-proposal-register.md).
The partial owner ruling is
[`slice-6-hitl-decisions.md`](0.8.25/prework/slice-6-hitl-decisions.md).
All recommendations except P25-07 are ruled at `seq-272`; P25-17 explicitly
keeps all run data, P25-20 stays narrow, and P25-07 is the sole open decision.

## Immediate next slice

<!-- BEGIN GENERATED release-state:0.8.25:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 6** (`HITL`) — proposal scoring, HITL decisions, and Slice 7 plan

**Remaining ladder:** 6 → 7 → 10 → 15 → 20 → 25 → 30 → 35 → 40 → 45 → 50 → 55 → 60 → 65 → 70 → 75.<!-- END GENERATED release-state:0.8.25:plan-immediate-next -->

Resolve P25-07 using the focused local/upstream evidence in the partial ruling.
After that ruling, write the approved-only Slice 7 implementation plan and
obtain its explicitly required independent review. The direct agent must not
invoke a Steward or Orchestrator. Do not begin implementation before the owner
approves the reviewed Slice 7 plan.

## Stop gates

Stop for an unresolved P1/P2 review finding, a public-surface change without
ADR/interface grounding, stale or uneraseable evidence, an unbounded query
path, binding drift, a benchmark treatment that misses its registered boundary,
or a requirement that belongs to the external semantic component.

Standing CUDA and ptrace authorization at `seq-271` permits applicable
verification; it does not authorize publication or weaken the protected release
route.
