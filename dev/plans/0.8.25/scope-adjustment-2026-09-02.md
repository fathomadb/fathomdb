---
title: FathomDB 0.8.25 post-design scope adjustment
status: APPROVED
target_release: 0.8.25
decided_on: 2026-09-02
---

# FathomDB 0.8.25 post-design scope adjustment

## Decision and authority

This owner-approved decision narrows the reviewed 0.8.25 design envelope after
the design-documentation campaign. It supersedes the 0.8.25 allocation in
P25-22 where the two conflict. It does not invalidate or delete the reviewed
designs: they remain durable architecture and experimental evidence, but only
the retained subset below may advance toward READY implementation authority in
0.8.25.

FathomDB remains a local-first, provenance-preserving retrieval data plane.
This release prioritizes identity, governed visibility, lifecycle closure,
source-complete evidence, and truthful operation. Semantic policy and
unproven retrieval algorithms remain outside the release implementation scope.

## Retained 0.8.25 scope

| Slice | Retained outcome | Explicit boundary |
| ---: | --- | --- |
| 10 | Executable measurement classification and trustworthy release claims. | Keep the native `Engine.search` witness and historical classification without changing measured payloads. |
| 15 | Immutable revision/source identity, source version, exact locator, canonical hash, additive wire evolution, and SDK parity. | Preserve legacy reads and additive migration behavior. |
| 20 | Core dependency registration. | Canonical-source-to-derived dependency identity, bounded forward/reverse lookup, structural validation, and cycle rejection only. Multi-source sets, general derived-to-derived graphs, and configurable liveness grammar are deferred. |
| 25 | Core model-free atomic actuation. | One bounded idempotent batch for canonical/derived writes, core dependency registration, and caller-decided lifecycle actions; return a compact committed/refused receipt. Broader omnibus operations and exhaustive consequence journals are deferred. |
| 30 | Dependency-aware lifecycle and erasure closure. | Keep visibility fencing, idempotent restart/resume, structured proof, and no searchable orphan for the Slice 20 core dependency model. |
| 35 | Eligibility before ranking plus optional frozen reads. | Owner/scope/kind/status/time predicates apply before lexical, vector, and graph truncation. Ordinary reads do not require a frozen snapshot. Advanced snapshot leases are deferred. |
| 40 | Core projection generation/readiness. | Keep durable generation identity, false-readiness prevention, restart-safe advancement, and compact mutation-to-ready correlation. Defer a richer public work-manifest surface. |
| 45 | Minimal pagination and governed state reads. | Add bounded stable continuation for canonical and `operational_state` reads plus point reads. Ranked top-K remains separate. Full lease semantics and generalized graph pagination are deferred. |
| 50 | Compact source-complete evidence contract. | Add opt-in evidence identity and eligibility-bound resolution without growing default `SearchHit`. Persisted evidence leases and replayable receipt retention are deferred. |
| 55 | Basic tracing and integrity. | Add bounded source-to-derived and derived-to-source tracing, orphan/projection checks, and compact inclusion/degradation explanation. Persisted trace pages, exhaustive exclusion explanation, frozen jobs, and repair orchestration are deferred or experimental. |
| 60 | Minimal constrained graph parity. | Combined expansion must honor seed source, direction, `edge_kind`, target kind, bounds, eligibility, and one read context with deterministic one-page results. Rich continuation and full path-evidence replay are deferred. |
| 75 | Trimmed trustworthy release verification. | Audit installed SDK/wire parity, representative lifecycle/evidence/concurrency paths, selected regression performance, and the native retrieval-only witness. Do not require exhaustive scale-by-feature-by-CUDA matrices. |

The active feature ladder is therefore:

`10 -> 15 -> 20 -> 25 -> 30 -> 35 -> 40 -> 45 -> 50 -> 55 -> 60 -> 75`

Slice 7 remains the immediate prerequisite. Slices 65 and 70 are removed from
the active 0.8.25 ladder.

## On-the-bubble allocation

| Target | Deferred work | Re-entry condition |
| --- | --- | --- |
| 0.8.26 | Multi-source provenance; bounded derived-to-derived dependencies; the small `all_required`/`any_surviving` liveness grammar; broader semantic-batch operation coverage; complete mutation-consequence receipts. | Revalidate against the shipped Slice 20/25/30 core and demonstrate an immediate consumer path that otherwise needs shadow state or multiple non-atomic calls. |
| 0.8.27 | Full cross-operation frozen-snapshot leases; request/snapshot/generation-bound cursor protocol; generalized graph pagination; persisted evidence receipts and eligibility-bound replay; richer `operational_state` continuation. | Prove the compact 0.8.25 contracts cannot provide the required consistency or continuation behavior and retain opt-in compatibility. |
| 0.8.28 | Manual named-profile/configuration/qualification contract; specialized time-scoped/changed-fact retrieval profile; rich constrained-graph continuation and path-evidence replay; expanded deterministic exclusion tracing. | Require a reviewed use case, bounded implementation, and quality/lifecycle/performance evidence. No automatic routing or default change is included. |
| Parked | Arbitrary dependency DAG/liveness languages; 1,000-operation omnibus semantic batches; public crash-journal administration; mandatory frozen snapshots; richer public projection-work manifests; unbounded browse/trace APIs. | Reconsider only from a concrete consumer requirement that cannot be served by the narrower contracts. |

## Experimental allocation

Experimental work is evidence-generating, not committed product scope. Review
means decide whether to run or continue an experiment; it does not imply
promotion, a default change, or inclusion in the named release.

| Planning review | Experimental family | Promotion prerequisite |
| --- | --- | --- |
| 0.8.29 | Entity/alias expansion; complementary-evidence and coverage-aware selection; MMR/diversity treatments beyond accepted A0. | Preregistered held-out gain with no correctness, groundedness, attribution, lifecycle, or accepted-default regression. |
| 0.8.31 | Associative PPR/graph diffusion; automatic profile routing. | A deterministic treatment must first beat the accepted retrieval/answer boundary. Routing additionally requires at least two independently accepted manual profiles and inspectable deterministic fallback. |
| 0.8.33 | Full integrity-job orchestration and sophisticated repair planning; exhaustive scale-by-feature-by-CUDA matrices. | Demonstrate that representative checks miss a material operational risk and that the expanded machinery or matrix changes release decisions. |
| Parked | Database-owned query decomposition, synthesis, consolidation judgment, answer verification, or abstention. | These responsibilities conflict with the data-plane boundary. Reconsider only through an explicit architecture change; experiments belong in the external semantic component meanwhile. |

Every experiment retains its input, configuration, receipt, result, and
negative evidence. An odd-micro planning review may defer an item to the next
listed review or Parked, but may not silently promote it.

## Documentation and implementation effect

- The reviewed Slice 65 and 70 designs are preserved and marked reallocated.
- Retained slice designs remain useful maximum-envelope designs, but their
  0.8.25 implementation authority is limited by this document and their
  revised slice plans.
- Future-release scope files preserve deferred work; no implementation is
  authorized by allocation alone.
- Slice 75 depends directly on Slice 60 and verifies only the retained surface.
- Any later expansion requires its target release's normal requirements,
  acceptance criteria, design review, TDD RED/GREEN, implementation review,
  and verification gates.
