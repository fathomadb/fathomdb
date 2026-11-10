---
title: 0.8.29 Slice 4 architecture and source alignment proposal
status: PROPOSED
target_release: 0.8.29
slice: 4
---

# Slice 4: architecture and source alignment

## Goal, maturity and boundary

Produce a code-grounded architecture review plan and, when prework is run,
proposals for the Slice 9 decision package. This draft does not execute that
review or approve any correction. Distinguish implemented, historically proven,
designed, experimental and net-new behavior; inclusion does not prove current qualification.

The [drafting brief](../../planning-brief.md),
[reconciliation](../../reconciliation-plan.md) and
[release proposal](../../../plan-0.8.29.md) set scope. Format 37, integrated
MCP/queue/Block 1 and partial B2 behavior are the starting substrate.
No source, canonical contract, environment, Git, ledger or release-state change
belongs to this slice. Architecture changes remain proposed successors/addenda.

## Sources inspected and bounded reading

- [Active architecture v2](../../../../design/fathomdb-data-plane-architecture-v2.md): current profile and data-plane/semantic-policy boundary.
- [ADR index](../../../../adr/ADR-0.6.0-decision-index.md), [runtime topology](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md), [format37 visibility](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md).
- [Canonical identity ADR](../../../../adr/ADR-0.8.0-canonical-identity-substrate.md), [provider transport ADR](../../../../adr/ADR-0.8.6-generalized-provider-protocol.md), [narrow provider record](../../../0.8.28/features/slice-50/built-in-provider-implementation-record.md).
- [MCP architecture proposal](../../../../design/mcp-adapter/architecture.md), [agent-access work list](../../../../design/agent-access/README.md), [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md).
- Concrete source excerpts: [fusion](../../../../../src/rust/crates/fathomdb-engine/src/fusion.rs), [open/provider admission](../../../../../src/rust/crates/fathomdb-engine/src/open.rs), [search](../../../../../src/rust/crates/fathomdb-engine/src/search.rs), [hints](../../../../../src/rust/crates/fathomdb-engine/src/hints.rs), [registry](../../../../../src/rust/crates/fathomdb-mcp/src/registry.rs).
- [Writer coalescing](../../../../../src/rust/crates/fathomdb-engine/src/write_commit.rs), [projection publication](../../../../../src/rust/crates/fathomdb-engine/src/projection_commit.rs), [schema version](../../../../../src/rust/crates/fathomdb-schema/src/lib.rs).
- Memory index and verified entries `verify-design-against-code-not-just-architecture.md`, `acceptance-md-locked-no-feature-acs.md`, `agent-access-block1-design-state.md`, `physical-erasure-done-defects-only.md` informed this draft; old memory status yields to current source/decisions.

Initial observations bound the future review: fusion explicitly merges by body;
production numerical-provider admission is Default/None; hints already include
empty-result/degradation signals; coalescing is transaction-owned. These are
inspection findings, not new correctness or performance claims.

## Slice prep: local needs, requirements and acceptance

All IDs below are release-local drafts; none amends canonical acceptance.

| Local ID | Draft need or requirement | Falsifiable acceptance |
| --- | --- | --- |
| PW29-4-N1 | Designers need an accurate exists-versus-new map. | Every reviewed proposal is tied to current owner modules and a maturity classification. |
| PW29-4-R1 | Compare Slice 0–3 proposals with accepted architecture and current code. | Every material claim has authority, source seam, delta and destination; unknowns are explicit. |
| PW29-4-R2 | Preserve identity, eligibility and visibility across optimizations. | Matrix addresses duplicate bodies, identity spaces, pre-cap eligibility, frozen reads, generation/nonce and external writers. |
| PW29-4-R3 | Preserve provider, runtime and adapter ownership. | Matrix distinguishes numeric embedding from caller semantic subprocess tasks and names every execution/commit boundary. |
| PW29-4-R4 | Propose bounded, independently reviewable corrections. | Each finding states evidence, affected contract, alternatives, proposed slice, prerequisites and a disconfirming check. |
| PW29-4-AC1 | Review scope is complete. | Every selected slice theme and every Slice 0–3 architecture-affecting finding has a row or explicit exclusion. |
| PW29-4-AC2 | Proposals are decision-ready. | No proposed behavior change lacks a test oracle, authority disposition or Slice 9 decision entry; no proposal claims execution. |

## Design approach and fit revision

Draft `requirements.md`, `design.md`, `alignment-matrix.md` and `test-mapping.md`
beside this plan. Map authority → actual owner/call path → proposal → observable
invariant → destination. Classify mismatch as stale documentation, code defect,
unsettled contract, intentional deferred destination or insufficient evidence.

Review five bounded paths rather than commissioning a general redesign:

1. Open/configuration/provider refusal → owned runtime → close; preserve failed-open cleanup and bounded provider retention.
2. Canonical/projection commit → format37 generation/nonce → ordinary/frozen read invalidation, including external SQLite writers.
3. Dense/native eligibility and source-dependency fallback → exact text candidates → fusion → snippets; keep provenance and UTF-8 boundaries explicit.
4. MCP admission/registry → synchronous SDK → engine dispatch → response shaping; preserve queue run-to-completion and data-free hints.
5. SDK/native/facade fields → compact output/degradation/span reporting; distinguish built storage/signals from unbuilt presentation.

Challenge cache proposals with write-interleaved workloads; a generation cache
is not selected by read-only prototype results. Counters/schema changes need a
separate decision. Fusion identity needs an explicit ADR/census before the
optimized text route; do not turn equal-body merging into a silent ranking change.
Keep Block 2 coverage/scored spans truthful; no guessed weak-result threshold.

Primary-source research starts with accepted ADR successor chains, maintained
interfaces and production callers/tests. For an unresolved mechanism, plan a
bounded check of SQLite or upstream runtime/library documentation; record the
exact question and applicability before external research. No external research
was performed for this plan. Reconcile historical proposal assumptions against source.
Apply the quality roadmap's optimized/reference and boundary/fault-oracle ideas
to proposed tests; defer broad fuzzing, exhaustive matrices and SQLite internals.
Fit review revises ownership/deltas before proposing any architecture addendum.

## Dependencies, tasks and deliverables

- Hard input for completed review: Slice 0 environment constraints, 1 candidate/pin impacts, 2 disposition inventory and 3 draft needs/contracts. Reading can start earlier; missing drafts remain explicit prerequisites.
- Iterate with Slice 3 on contract intent and Slice 5 on meaningful test oracles; send stale audience documentation to 6 and build/CI mechanics to 7.
- Slice 9 requires the reviewed package; Slice 10 may implement only recorded preparation decisions. Product corrections stay in their selected feature slices.
- Preserve 30 measurements as hard inputs to 40/50; 40 → 50 is preferred landing order only; 60 uses integrated flags/filter support, with final fusion parity later. Approval of 80 needs settled identity/fields/degradation; 90 also needs Block 1 follow-up exit.
- Conditional fork qualification in 140 precedes affected pin adoption in 110/120 and final qualification in 130; numerical labels do not imply execution order.
- Shared contracts/source owners are review conflicts to serialize later. Prework readers may fan out; only the parent assembles repository drafts. No compute/host access is assumed.

Tasks: inventory incoming proposals; build the five path maps; challenge contract
and ownership deltas; draft corrective alternatives; identify test obligations;
review fit with 3/5; issue `review.md` with unresolved items and Slice 9 inputs.
Each finding records current evidence, confidence, impact, proposed allocation,
decision owner and the evidence needed to reject or accept its premise.

New prerequisites for the next reviewer: verify the active profile's eleven-member
workspace statement against integrated MCP manifests; distinguish historical
generalized subprocess-provider text from current numerical-provider admission;
rebase cache assumptions onto format37; census equal-body identity collisions;
confirm B2-6/B2-7 residuals before assigning new implementation.

## Completion, independent review and stop points

Complete only when local AC1/AC2 pass against the assembled artifacts and an
independent reviewer can reconstruct every material claim from cited evidence.
Record review findings and unresolved disagreements without inventing verdicts.
Validate links and relevant Markdown/planning rules when drafts enter the repo;
proposal completeness is the oracle here, not a product-test pass.

Later authorized behavior changes require reviewed contracts, per-behavior RED
evidence before implementation, real databases, boundary/property tests and
independent code review. Source phases use required full/native qualification;
planning validation cannot support a whole-workspace green claim.
Stop a proposed promotion at an accepted-ADR conflict, undecided identity,
unsupported performance premise or unavailable evidence; route it to Slice 9.
Physical erasure stays complete except concrete defects and bounded Slice 100
qualification. Overflow chunking, ANN, caller-provider APIs and ranking/default
changes need their own authorization; this architecture review creates none.
