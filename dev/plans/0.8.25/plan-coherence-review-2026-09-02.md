---
title: FathomDB 0.8.25 plan coherence review
status: COMPLETE
target_release: 0.8.25
reviewed_on: 2026-09-02
---

# FathomDB 0.8.25 plan coherence review

## Scope and method

This review compares the active release plan, scope adjustment, architecture
v2, foldback plan, release-state ladder, status board, Memex-needs crosswalk,
feature index, and every active Slice 10–75 plan. It checks:

- one dependency-linear active ladder;
- one unambiguous implementation scope per slice;
- correct placement of deferred and experimental work;
- architecture and consumer-need traceability;
- proportional verification and publication boundaries; and
- truthful design/readiness status after narrowing reviewed designs.

Preserved prework records, completed design reviews, and the reallocated Slice
65/70 designs are historical evidence rather than active implementation
authority.

## Findings and corrections

| ID | Finding | Correction | State |
| --- | --- | --- | --- |
| CR25-01 | The foldback slice summaries still described maximum-envelope multi-source dependencies, omnibus actuation, mandatory snapshots, and generalized cursor work. | Reconciled the active summaries and proof packages to the approved core; retained future allocation. | Closed |
| CR25-02 | Slice 30 still required A25-04 multi-source liveness although Slice 20 had been narrowed to one canonical-source-to-derived form. | Narrowed Slice 30 closure to the Slice 20 core, marked its design for scope reconciliation, and retained multi-source closure in 0.8.26. | Closed |
| CR25-03 | Slice 10 and Slice 75 both named a native `Engine.search` witness without distinguishing responsibility. | Slice 10 owns a small classification fixture; Slice 75 owns the final packaged-release-candidate witness. | Closed |
| CR25-04 | `registry-installed` could be read as a pre-publication READY requirement. | Added `packaged` as the pre-publication route; actual registry-installed smokes are a separately authorized post-publication close gate. | Closed |
| CR25-05 | Architecture v2 still allocated full multi-source liveness and expanded exclusion explanation to 0.8.25. | Preserved them as architecture goals while assigning delivery to 0.8.26 and 0.8.28. | Closed |
| CR25-06 | CUDA was mandatory in Slice 60 merely because combined search can use a dense arm. | Made CUDA conditional on a dense/rerank dispatch change; device-independent graph constraint parity does not require a cross-product run. | Closed |

## Active slice coherence

| Slice | Consumes | Produces | Review verdict |
| ---: | --- | --- | --- |
| 10 | Slice 7 environment and architecture activation | Executable metric ownership plus classification fixture | Aligned; final candidate witness remains in 75. |
| 15 | Measurement contract | Immutable revision/source identity, locator, hash, shared wire rules | Aligned and unchanged by narrowing. |
| 20 | Stable artifact/source identities | Core source-to-derived dependency registration and lookup | Aligned; broader dependency forms retained in 0.8.26. |
| 25 | Core dependency identity | Bounded idempotent caller-decided batch and compact receipt | Aligned; no semantic decision moves into FathomDB. |
| 30 | Core batch and dependency form | Fenced, resumable lifecycle/erasure closure and no-orphan proof | Aligned to core dependencies; design reconciliation required. |
| 35 | Lifecycle-correct visibility | Pre-ranking eligibility and optional compact frozen read | Aligned; no mandatory snapshot overhead. |
| 40 | Eligibility/read context and mutation boundary | Projection generation identity and truthful readiness | Aligned; public work-manifest expansion remains Parked. |
| 45 | Read context and generation state | Minimal canonical/state continuation and point reads | Aligned; full leases and graph pages remain in 0.8.27. |
| 50 | Revision identity, eligibility, and read context | Compact source-complete evidence and safe resolver | Aligned; persisted replay remains in 0.8.27. |
| 55 | Dependencies, evidence, lifecycle, readiness | Basic reciprocal trace, orphan/projection checks, compact explanations | Aligned; expanded tracing/repair remains deferred or experimental. |
| 60 | Eligibility, evidence, and compact explanation | Honored graph constraints and deterministic bounded results | Aligned; rich continuation/path replay remains in 0.8.28. |
| 75 | All retained Slice 10–60 receipts | Proportionate installed parity, lifecycle/concurrency/performance closure, final native witness | Aligned; does not backfill feature-local proof or require experimental matrices. |

## Final verdict

**PASS — coherent and aligned, with normal READY gates still open.**

The release plan, release state, status board, active slice set, dependencies,
architecture allocation, and future-work placement agree. No active slice
depends on reallocated Slice 65/70 work. No experimental algorithm, live-model
route, publication, or actual registry mutation is required for 0.8.25.

Before each narrowed slice becomes READY, its maximum-envelope design must be
reconciled to the approved subset, receive the normal independent design
review, and name exact commands, fixtures, thresholds, and receipt paths. This
is planned work, not an unresolved architecture or scope decision.

## Verification

- Active plan frontmatter dependencies equal the release-state ladder.
- Every active plan has outcome, verification-route, and draft-to-ready
  sections.
- Every narrowed plan cites the approved scope authority.
- Slice 65/70 plans remain `REALLOCATED_EXPERIMENTAL`.
- Markdown/plan-status lint, release-state generated-view checks,
  design-reference checks, and `git diff --check` pass.
