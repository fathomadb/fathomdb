---
title: FathomDB 0.8.26 — Memex contract completion
status: PROPOSED
target_release: 0.8.26
---

# FathomDB 0.8.26 — release plan

## Goals and scope

0.8.26 implements only the Priority 0, Priority 1, and Priority 2 work selected
from the
[`Memex needs prioritization scaffold`](memex-0.6.0-collaboration/memex-needs-prioritization-scaffold.md).
It repairs the released frozen-explanation contract, completes its public
guidance and installed-artifact witness, adds exact graph-target evidence,
qualifies and hardens the existing distributable read-only integrity route,
and—only after the write-path decision gate—adds atomic derived-edge actuation
with receipt evolution only if current receipt storage proves insufficient.

Multi-source provenance, source-set liveness, recursive closure, rich graph
paths/continuation, persisted evidence replay, snapshot leases, candidate
profiles, and general repair orchestration remain outside this release.

## Setup record

- **Baseline:** `40f807f5198cf826ef08ffd50658c8bb23f6f0f0`, the clean
  Memex-collaboration planning input on top of published 0.8.25 `main`.
- **Release branch:** `release/0.8.26`.
- **Release worktree:**
  `/home/coreyt/projects/fathomdb-worktrees/release-0.8.26`.
- **Primary checkout:** intentionally untouched; it is stale and contains
  unrelated untracked user files.
- **Publication:** not authorized by this draft.

## Scope

In scope:

- the explanation-enabled frozen-search `/correlationId` defect;
- complete public Python guidance for frozen and evidence-bearing retrieval;
- an installed-artifact, one-source Memex-profile conformance witness;
- immutable-revision, frozen, eligibility-bound graph-target and terminal-edge
  evidence resolution;
- a version-matched, read-only, operator-scoped integrity inspection route;
- versioned atomic `put_derived_edge` actuation over `ProvenancedEdgeV1`; and
- only the mutation-receipt additions required to represent and replay that
  new operation truthfully.

Out of scope:

- Memex admission, ontology, contradiction, consolidation, retrieval planning,
  answer, personalization, action, model, spend, or HITL policy;
- any repair/rebuild verb on the governed SDK;
- a second writer, shadow dependency store, body re-search, or logical-ID
  search workaround;
- arbitrary dependency DAGs or liveness languages;
- unrelated platform/dependency work unless Slice 8 explicitly selects it as
  required preparation; and
- tag, registry, release, or merge actions without a separate owner decision.

## Slice ladder

Prework is sequential. Slices 0–7 inspect evidence and draft only; Slice 8
obtains HITL decisions and replaces the provisional Slice 9 plan; Slice 9
implements only approved preparation. Findings from Slices 6–7 may instead be
allocated to a numbered hardening slice after Slice 10 or to Slice 50 when the
fix depends on built product artifacts or belongs at the release boundary.

| Slice | Outcome | Depends on | State |
| ---: | --- | --- | --- |
| 0 | Record environment/project-infrastructure needs and establish the isolated release workspace and draft plan. | 0.8.25 + Memex scaffold | Complete |
| 1 | Perform a read-only dependency, advisory, Dependabot, and pinning sweep; propose responses without upgrades. | 0 | Complete |
| 2 | Review repository cruft and propose keep, deprecate-in-place, archive-in-place, or delete. | 1 | Complete |
| 3 | Draft user-need, requirement, acceptance, interface, ADR, and architecture CRUD; allocate every draft to one slice. | 2 | Complete |
| 4 | Review the proposed architecture and high-level code alignment; propose corrections only. | 3 | Complete |
| 5 | Review verification adequacy from need through test and critical path. | 4 | Complete |
| 6 | Review local build, preflight, transcript, and `agent-verify` failure evidence; propose pragmatic corrections without implementing them. | 5 | Complete |
| 7 | Review post-build CI/CD, packaging, gitleaks, and registry failure evidence; propose pragmatic corrections and delivery placement. | 6 | Complete |
| 8 | Score all proposals, conduct interactive HITL decisions, replace/review Slice 9, and update this plan. | 7 | Draft |
| 9 | Implement only HITL-approved repository preparation under the reviewed Slice 9 plan. | 8 | Provisional draft |
| 10 | Repair frozen explanation, complete public guidance, and add the installed-artifact conformance witness. | 9 | Draft |
| 15 | Run the bounded graph-evidence performance and erasure-linearization implementation spike; accept or narrow the Slice 20 design. | 10 | Proposed by Slice 8 evidence |
| 20 | Add immutable-revision graph-target and terminal-edge evidence resolution under frozen authority. | 15 | Draft |
| 30 | Qualify and harden the existing versioned read-only operator integrity inspection route. | 20 | Draft |
| 35 | Run the bounded actuation V2 contract and performance implementation spike; accept or narrow the Slice 40 design. | 30 | Proposed by Slice 8 evidence |
| 40 | Add versioned atomic derived-edge actuation under the Slice 35 contract; evolve receipts only if explicitly approved. | 35 | Draft |
| 50 | Run integrated Memex-profile, cross-SDK, platform, package, and non-publishing release verification. | 40 | Draft |

## Requirements and acceptance criteria

Prework requirements are defined in
[`0.8.26-prework-slices-0-9.md`](0.8.26-prework-slices-0-9.md).
Feature requirements and draft acceptance criteria live in each Slice 10+
plan. Slice 3 may draft contract changes but cannot accept them; Slice 8 owns
the interactive scope decision.

No feature is accepted merely because it appears in this plan. New global
acceptance IDs are prohibited unless Slice 8 explicitly authorizes them.

## Cross-cutting definition of done

Every slice follows the
[`lean slice execution contract`](0.8.26/slice-execution-contract.md).
Every feature slice must additionally:

1. reconcile its draft design against the Slice 3–5 findings;
2. obtain independent design review and resolve findings before READY;
3. commit a real failing test, or stage it visibly for review, before product
   implementation;
4. implement through the existing single-writer/reader architecture;
5. obtain independent implementation review;
6. pass focused, property/fault, cross-SDK, restart, concurrency, and package
   routes proportionate to risk, with the broad release matrix reserved for a
   comprehensive change or Slice 50;
7. verify generated and published-surface documentation; and
8. write a status record with exact commit and evidence.

The fixed package/version claim requires a fresh built artifact. Publication
and post-publication registry tests require separate authorization.

## Reserved-gap policy

Every prework finding is recorded and receives an include, postpone, reject, or
needs-more-information ruling in Slice 8. Slices 6–7 findings must also receive
an explicit delivery placement: Slice 9, a reserved post-10 hardening slice,
Slice 50, postpone, or reject. The earlier
[`0.8.26 draft scope`](0.8.26-draft-scope.md) is input, not authority: its
platform, dependency, evidence-manifest, multi-source, liveness, and broad-batch
items must be re-evaluated. Priority 3+ Memex items remain outside 0.8.26 even
if their design is discussed.

## Immediate next action

Review the completed Slice 0–7 proposal package in Slice 8. Score and decide
each proposal interactively, then replace the provisional Slice 9 plan. Do not
activate release state, change dependencies, clean up evidence, or begin
feature work before that decision.

## Stop gates

Stop on an unresolved public-contract versioning decision; any schema/data
migration not explicitly approved; weakened frozen eligibility or evidence
non-disclosure; a repair verb leaking onto the governed SDK; partial graph
mutation; V1 digest/replay drift; binding incompatibility; or semantic policy
moving into FathomDB.
