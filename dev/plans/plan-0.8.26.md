---
title: FathomDB 0.8.26 — breaking Memex contract completion
status: ACTIVE
target_release: 0.8.26
---

# FathomDB 0.8.26 — breaking release plan

## Goals and scope

0.8.26 implements only the Priority 0, Priority 1, and Priority 2 work selected
from the
[`Memex needs prioritization scaffold`](memex-0.6.0-collaboration/memex-needs-prioritization-scaffold.md).
It repairs the released frozen-explanation contract, completes its public
guidance and installed-artifact witness, adds exact graph-target evidence,
qualifies and hardens the existing distributable read-only integrity route,
and changes the single V1 actuation grammar in place to add atomic derived-edge
support. A post-verification adversarial review then reopens the ladder for an
executable cross-SDK parity oracle, current retrieval/recovery/engine owner
reconciliation, lifecycle authority-topology guards, and exact candidate
requalification. The release accepts fresh databases only and carries no
historical actuation, receipt, replay, integrity, operation-ID, or
database-migration compatibility. It introduces no parallel functional V1/V2
public API pairs.

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
- current-V1 atomic `put_derived_edge` actuation over `ProvenancedEdgeV1`; and
- one changed-in-place V1 request, receipt, replay, and integrity contract for fresh 0.8.26
  databases;
- an executable canonical-operation equality oracle across the Python and
  TypeScript governed surfaces;
- current code-grounded retrieval, recovery, and engine design owners;
- machine-checked maintained-owner authority and witness topology; and
- a replacement exact non-publishing candidate after the reopened ladder.

Out of scope:

- Memex admission, ontology, contradiction, consolidation, retrieval planning,
  answer, personalization, action, model, spend, or HITL policy;
- any repair/rebuild verb on the governed SDK;
- a second writer, shadow dependency store, body re-search, or logical-ID
  search workaround;
- parallel functional V1/V2 surfaces, historical request translation,
  historical receipt/replay or integrity support, cross-release operation-ID
  behavior, and database migration from any earlier release;
- arbitrary dependency DAGs or liveness languages;
- unrelated platform/dependency work unless Slice 8 explicitly selects it as
  required preparation;
- tag, registry, release, or merge actions without a separate owner decision;
  and
- product, schema, migration, or public-surface changes arising only from the
  design reconciliation; an implementation contradiction is a stop condition,
  not implicit authorization to repair runtime code.

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
| 8 | Score all proposals, conduct interactive HITL decisions, replace/review Slice 9, and update this plan. | 7 | Complete; D26-01 later resolved at `seq-290` |
| 9 | Implement only HITL-approved repository preparation under the reviewed Slice 9 plan. | 8 | Complete on release branch at `ded6c72f` |
| 10 | Repair frozen explanation, complete public guidance, and add the installed-artifact conformance witness. | 9 | Complete on release branch at `c2795caf` |
| 15 | Run the bounded graph-evidence performance and erasure-linearization implementation spike; accept or narrow the Slice 20 design. | 10 | Complete on release branch; opt-in V1 sidecar selected at `seq-290` |
| 20 | Add immutable-revision graph-target and terminal-edge evidence resolution under frozen authority. | 15 | Complete on release branch at `c0a567d5` |
| 30 | Qualify and harden the existing versioned read-only operator integrity inspection route. | 20 | Complete on release branch at `6d80e7a8` |
| 35 | Prove the breaking changed-in-place V1 actuation, fresh-database, receipt, and performance contract; accept or narrow the Slice 40 design. | 30 | Complete on release branch at `9a81a75c` |
| 40 | Extend V1 actuation in place with the accepted atomic derived-edge contract and fresh-database boundary. | 35 | Complete on release branch at `bfb2132b` |
| 45 | Reconcile maintained architecture documentation with accepted decisions, public contracts, and the as-built 0.8.26 system. | 40 | Complete on release branch at `6f68e2fd` |
| 46 | Reconcile and organize maintained technical design documentation under the Slice 45 architecture hierarchy. | 45 | Complete on release branch at `6d952c9c` after release-wide review and full-gate remediation |
| 50 | Run integrated Memex-profile, cross-SDK, platform, package, documentation, and non-publishing release verification. | 46 | Complete on release branch at `68514f70`; superseded as the final candidate by the reopened ladder |
| 55 | Replace subset surface checks with an executable canonical-operation Python/TypeScript parity oracle. | 50 | Next |
| 60 | Reconcile the maintained retrieval, recovery, and engine owners against one code-grounded authority matrix, then back-check earlier owner and SDK artifacts. | 55 | Draft |
| 65 | Enforce maintained-owner authority topology, reconcile error ownership, rerun affected earlier proofs, and bind the replacement exact candidate. | 60 | Draft |

## Requirements and acceptance criteria

Prework requirements are defined in
[`0.8.26-prework-slices-0-9.md`](0.8.26-prework-slices-0-9.md).
Feature requirements and draft acceptance criteria live in each Slice 10+
plan. Slice 3 may draft contract changes but cannot accept them; Slice 8 owns
the interactive scope decision.

No feature is accepted merely because it appears in this plan. New global
acceptance IDs are prohibited unless Slice 8 explicitly authorizes them.

## Cross-cutting DoD

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

0.8.26 is not an in-place upgrade. Every installed-artifact witness creates a
fresh database. One representative prior-version database must be refused
before mutation; no historical migration matrix is required.

## Reserved-gap policy

Every prework finding is recorded and receives an include, postpone, reject, or
needs-more-information ruling in Slice 8. Slices 6–7 findings must also receive
an explicit delivery placement: Slice 9, a reserved post-10 hardening slice,
Slice 50, postpone, or reject. The earlier
[`0.8.26 draft scope`](0.8.26-draft-scope.md) is input, not authority: its
platform, dependency, evidence-manifest, multi-source, liveness, and broad-batch
items must be re-evaluated. Priority 3+ Memex items remain outside 0.8.26 even
if their design is discussed.

## Immediate next slice

<!-- BEGIN GENERATED release-state:0.8.26:plan-immediate-next -->
**IMMEDIATE NEXT: Slice 65** (`LIFECYCLE-GATE`) — semantic ownership recurrence guards and candidate requalification

**Remaining ladder:** 65.<!-- END GENERATED release-state:0.8.26:plan-immediate-next -->

D26-01 was ruled at `seq-290` and implemented by Slice 20. Slice 30 completed
the selected existing-CLI integrity route without a prebuilt fallback. Slice
50 remains historical evidence for candidate `68514f70`, but it is no longer
the integration candidate after the ladder reopened. Do not integrate, tag, or
publish before Slice 65 binds the replacement exact candidate.

## Stop gates

Stop on an unresolved public-contract shape at its owning slice; any path that
migrates a pre-0.8.26 database or introduces a parallel V2 actuation surface; weakened
frozen eligibility or evidence
non-disclosure; a repair verb leaking onto the governed SDK; partial graph
mutation; binding incompatibility; or semantic policy moving into FathomDB.
