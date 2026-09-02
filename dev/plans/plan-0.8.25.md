---
title: FathomDB 0.8.25 — governed data-plane foldback
status: ACTIVE
target_release: 0.8.25
---

# FathomDB 0.8.25 — release plan

## Goals and scope

0.8.25 uses the completed performance program and complete Memex 0.6.0
consumer-needs inventory to deliver the essential governed FathomDB data-plane
core and allocate nonessential work durably. FathomDB owns durable mechanisms
and invariants. An external semantic component owns semantic policy, reasoning,
and answers.

The requirements crosswalk is
[`memex-0.6.0-needs-in-fathomdb-0.8.25.md`](memex-0.6.0-needs-in-fathomdb-0.8.25.md).
The approved post-design implementation boundary is
[`0.8.25/scope-adjustment-2026-09-02.md`](0.8.25/scope-adjustment-2026-09-02.md);
it supersedes earlier allocation language where they conflict.
The prework contract is
[`0.8.25-prework-slices-0-7.md`](0.8.25-prework-slices-0-7.md).
The successor architecture and delivery plan are
[`fathomdb-data-plane-architecture-v2.md`](../design/fathomdb-data-plane-architecture-v2.md)
and [`fathomdb-data-plane-foldback-v2.md`](fathomdb-data-plane-foldback-v2.md).
Architecture v2 is approved by the completed Slice 4–6 review/HITL sequence
and becomes active implementation authority when Slice 7 completes.

## Setup record

- **Merged baseline:** `main` at
  `fca10bd2dfbb59ba7446fe66e0b1a9555c4df993`, which contains the merged
  `release/0.8.24` branch.
- **Release branch:** `release/0.8.25`.
- **Release worktree:**
  `/home/coreyt/projects/fathomdb-worktrees/release-0.8.25`.
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
  planning-only checks, not native rebuild verification. Slice 0 recorded the
  need for an isolated release-bound build/test arrangement; Slice 7 implements
  the approved arrangement without rebuilding against the wrong source or
  polluting the primary `main` checkout.
- **Allocation verification:** `./scripts/agent-verify.sh` passed all 103
  executed suites with one intentional TypeScript skip. The strict ptrace and
  egress checks ran unchanged; 1,251 Python tests passed with 32 explicit
  test-hook/integration skips in the focused environment check.

## Scope

In scope:

- executable measurement-layer classification;
- durable record-revision and source-version provenance;
- core canonical-source-to-derived dependency registration and lookup;
- bounded atomic application of caller-decided records, dependencies, and
  lifecycle actions with compact committed/refused receipts;
- dependency-aware lifecycle, erasure, and integrity closure;
- optional frozen reads and uniform eligibility-before-ranking predicates;
- core projection generation identity and readiness correlation;
- minimal stable canonical pagination and governed `operational_state` reads, with
  `latest_state` remaining a consumer concept;
- compact opt-in source-complete evidence, basic tracing, and integrity checks;
- minimal constrained combined graph parity; and
- proportionate cross-SDK, concurrency, lifecycle, and performance
  verification.

Out of scope:

- an Engine-owned semantic control plane;
- extraction, entity resolution, contradiction, truth, ontology, query intent,
  decomposition, synthesis, answer generation, semantic verification, model
  choice, spend, or HITL policy;
- adoption of a benchmark treatment that fails its registered boundary;
- candidate-selection, associative/diffusion, automatic-routing, or other
  experimental profile implementation allocated to the odd-micro review
  schedule;
- exhaustive scale-by-feature-by-CUDA matrices where representative release
  regression evidence is sufficient; and
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
| 6 | Consolidate and score proposals, conduct interactive HITL decisions, write/review the Slice 7 plan, and update the release plan. | 5 | Complete on release branch (`3a35c1e6`; approved `seq-274`) |
| 7 | Implement only HITL-approved repository-preparation work from Slices 0–6 with TDD/review/independent verification; write status and clean isolated worktrees. | 6 | Ready |
| 10 | Make measurement-layer classification executable, including whether `Engine.search` ran and which compared components differed. | 7 | Not started |
| 15 | Add immutable record-revision identity, caller source-version identity, exact source locators, canonical hashes, and missing Rust-facade identity exports. | 10 | Not started |
| 20 | Add core queryable canonical-source-to-derived dependency registration, bounded lookup, validation, and cycle rejection. | 15 | Not started |
| 25 | Add a bounded model-free atomic batch for caller-decided records, core dependencies, and lifecycle actions with compact idempotent receipts. | 20 | Not started |
| 30 | Close lifecycle and erasure across registered dependencies with visibility fencing, idempotent propagation, and no-active-orphan proof. | 25 | Not started |
| 35 | Add uniform indexed eligibility before lexical, vector, or graph truncation plus an optional Engine-minted frozen read context. | 30 | Not started |
| 40 | Add core durable projection-generation identity, false-readiness prevention, restart-safe advancement, and compact mutation-to-ready correlation. | 35 | Not started |
| 45 | Add minimal stable canonical pagination plus governed `operational_state` point/page reads while keeping `latest_state` a consumer concept. | 40 | Not started |
| 50 | Add compact opt-in source-complete evidence resolution under the original or equivalent eligibility envelope. | 45 | Not started |
| 55 | Add basic reciprocal provenance tracing, orphan/projection integrity checks, and compact inclusion/degradation explanation. | 50 | Not started |
| 60 | Make combined graph expansion honor typed seed, direction, edge/target, bound, eligibility, and read-context constraints with deterministic one-page results. | 55 | Not started |
| 75 | Audit installed cross-SDK/wire parity, representative concurrency/lifecycle/evidence paths, selected regression performance, and retrieval-only measurement. | 60 | Not started |

## Requirements and acceptance criteria

Prework requirements are defined by
[`0.8.25-prework-slices-0-7.md`](0.8.25-prework-slices-0-7.md): Slices 0–5
produce evidence and proposals, Slice 6 records HITL decisions and a reviewed
Slice 7 plan, and Slice 7 implements only approved repository preparation.

Feature requirements and slice-local falsifiable acceptance criteria are
written by their owning Slice 10+ plan before implementation. The draft plan
index is [`0.8.25/features/README.md`](0.8.25/features/README.md), and the
complete consumer-needs allocation is
[`memex-0.6.0-needs-in-fathomdb-0.8.25.md`](memex-0.6.0-needs-in-fathomdb-0.8.25.md).
No feature is accepted merely because it appears in this release overview.

The [design-documentation matrix](0.8.25/design-documentation-matrix.md)
records the completed maximum-envelope campaign: 21 projected logical needs
mapped exactly once to fourteen reviewed design records. The later approved
[scope adjustment](0.8.25/scope-adjustment-2026-09-02.md) limits 0.8.25
implementation authority, preserves the broader designs as evidence, and
allocates every removed item durably.

The design-documentation campaign is complete: all fourteen maximum-envelope
records passed independent review with no unresolved P1/P2 finding. The later
[design coherence review](0.8.25/design-coherence-review-2026-09-02.md)
reconciled the twelve active designs to the approved narrower scope and moved
removed design work to
[`0.8.x-after-0.8.25-design-notes.md`](../design/0.8.x-after-0.8.25-design-notes.md).
Because the normative designs changed materially, each active slice still
requires its formal independent review before READY. Slice 65/70 records remain
`REALLOCATED_EXPERIMENTAL`. Review does not start feature implementation.

## Cross-cutting DoD

Each feature slice writes numbered requirements and slice-local falsifiable
acceptance criteria, then writes or reconciles a design grounded in the reviewed architecture. An
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
an explicit owner decision includes, postpones, parks, or rejects it. Slice 7
cannot absorb Slice 10+ work. A failed benchmark treatment remains durable
negative evidence and does not become a default. Deferred work names its target
release or backlog authority; it is never silently dropped.

## Workload checkpoint

Slice 6 records every item before any scope reduction. It scores understanding,
risk, effort, and release disposition, then collects explicit HITL decisions.
The release was evaluated after the feature-design campaign and narrowed by
the approved 2026-09-02 scope adjustment. No item moved silently: deferred and
experimental work is assigned to 0.8.26, 0.8.27, 0.8.28, odd-micro review
checkpoints, or Parked. Completed performance tracks are not reopened.

The complete scored decision package is
[`slice-6-proposal-register.md`](0.8.25/prework/slice-6-proposal-register.md).
The complete proposal ruling is
[`slice-6-hitl-decisions.md`](0.8.25/prework/slice-6-hitl-decisions.md).
All proposals are ruled at `seq-272`/`seq-273`; P25-17 explicitly keeps all run
data, P25-20 stays narrow, and P25-07 includes the test-only `httpmock`
correction with its stop conditions. The Slice 7 plan passed independent
review after two bounded FIX cycles and was approved at `seq-274`.

## Immediate next slice

<!-- BEGIN GENERATED release-state:0.8.25:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 7** (`PREWORK-IMPLEMENTATION`) — approved repository-preparation implementation

**Remaining ladder:** 7 → 10 → 15 → 20 → 25 → 30 → 35 → 40 → 45 → 50 → 55 → 60 → 75.<!-- END GENERATED release-state:0.8.25:plan-immediate-next -->

Execute the approved Slice 7 plan directly, preserving its TDD, stop, review,
verification, no-push, and no-publication boundaries.

## Stop gates

Stop for an unresolved P1/P2 review finding, a public-surface change without
ADR/interface grounding, stale or uneraseable evidence, an unbounded query
path, binding drift, a benchmark treatment that misses its registered boundary,
or a requirement that belongs to the external semantic component.

Standing CUDA and ptrace authorization at `seq-271` permits applicable
verification; it does not authorize publication or weaken the protected release
route.
