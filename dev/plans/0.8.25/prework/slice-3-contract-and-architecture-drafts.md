---
title: 0.8.25 Slice 3 — contract and architecture CRUD drafts
status: COMPLETE
target_release: 0.8.25
observed_on: 2026-08-31
---

# Slice 3 — contract and architecture CRUD drafts

## Outcome

The existing locked 0.6.0 needs and requirements remain valid but do not
describe the complete semantic-data-plane integration contract. This draft
adds four successor needs and fourteen slice-local requirement/acceptance
packages. It proposes no deletion or renumbering of accepted contracts and
mints no global `REQ-*` or `AC-*` identifier.

## User-need CRUD draft

| Draft ID | CRUD | Proposed need | Rationale / allocation |
| --- | --- | --- | --- |
| N25-01 | Create | An application-managed semantic layer needs FathomDB to preserve and atomically actuate caller-decided identity, provenance, dependency, and lifecycle state without making semantic judgments. | Slices 15–30; extends NEED-001, NEED-010/011, and NEED-025. |
| N25-02 | Create | A caller needs every retrieved evidence item to remain attributable, visibility-correct, resolvable to exact source bytes, and reversible across version, supersession, and erasure. | Slices 35–55; extends NEED-001, NEED-021, and NEED-024. |
| N25-03 | Create | A caller needs bounded, deterministic retrieval primitives that can honor explicit eligibility and graph constraints without hidden semantic routing. | Slices 35, 60–70; extends NEED-012/013 and preserves external query intent. |
| N25-04 | Create | An evaluator/operator needs FathomDB-only quality, latency, readiness, and lifecycle claims to be separable from semantic-controller and answer-system results. | Slices 10 and 75; extends NEED-006, NEED-022, and NEED-024. |
| Existing needs | Keep | NEED-001–025 and their accepted amendments | No accepted need is contradicted. The new needs narrow data-plane outcomes rather than moving semantic policy into FathomDB. |
| NEED-026 trace row | Update | `dev/traceability.md` says NEED-026 was added for security hardening, but `dev/needs.md` contains no NEED-026 statement | Restore the missing locked-document statement from its recorded trace intent or remove the false “was added” claim; Slice 6 chooses, Slice 7 implements documentation only. |
| REQ-067 / AC-077 | Update | The placeholder says IR-1/IR-2 will define evidence recall, while the completed performance program now has registered retrieval measures and receipts | Preserve historical text but add a successor pointer to Slices 10/65/70/75; do not invent a global threshold in prework. |

No rename or delete is proposed. If adopted, the four drafts should enter a
versioned successor section rather than rewriting the historical 0.6.0 need
statements in place.

## Requirement and acceptance-criterion drafts

| Slice | Draft requirement | Falsifiable acceptance boundary |
| ---: | --- | --- |
| 10 | R25-10: Every evaluation receipt classifies each metric by layer and records whether `Engine.search` executed plus shared/differing components. | AC25-10: malformed or mixed-layer fixtures fail schema validation; existing GLOBAL-01 receipts are classified without changing measured values; one native retrieval-only witness proves the search flag. |
| 15 | R25-15: Every governed canonical/derived record has stable logical identity and immutable revision identity; source-backed records carry caller source version, exact locator, and canonical hash across restart/reindex. | AC25-15: round-trip/restart/reindex properties preserve IDs and bytes; bad locators/hashes fail typed; Rust/Python/TS/wire fixtures agree. |
| 20 | R25-20: Caller-declared canonical-to-derived, derived-to-derived, and source-set dependencies are queryable in both directions with an explicit liveness rule. | AC25-20: invalid references/cycles reject atomically; source removal exercises each liveness rule; no shadow index is needed. |
| 25 | R25-25: One typed, model-free semantic batch atomically applies already-decided writes, dependencies, graph/fact changes, lifecycle verdicts, and metadata with idempotent operation identity and a complete receipt. | AC25-25: injected mid-batch failure leaves no partial mutation; retry returns the recorded result; refusal and success receipts enumerate required consequences in every SDK. |
| 30 | R25-30: Lifecycle and erasure propagate through registered dependencies, fence incomplete state, resume idempotently, and do not report completion while an active/searchable orphan remains. | AC25-30: supersede/invalidate/delete/reactivate/erase fixtures cover single/multi-source closure, crash/restart, retries, stale-index scans, and no-orphan proof. |
| 35 | R25-35: One Engine-minted frozen snapshot and typed eligibility envelope applies before lexical, vector, and graph truncation; unsupported predicates reject typed. | AC25-35: mutation/validity races cannot mix views; eligible records are never lost to pre-filter truncation; indexed query-plan fixtures prohibit client-side emulation. |
| 40 | R25-40: Projection generations are durable identities correlated with mutation, readiness, blocked/deferred/degraded state, rebuild, and restart. | AC25-40: generation/readiness transitions survive restart and never claim a mutation ready under the wrong generation. |
| 45 | R25-45: Ordered list, graph, and `latest_state` reads use opaque request/view/generation/order-bound cursors; ranked top-K remains a separate contract. | AC25-45: concurrent page walks have no duplicate/omission; mismatch/expiry/drift return typed outcomes; latest-state replacement and point/page reads agree. |
| 50 | R25-50: Opt-in evidence references resolve exact revision/source/version/span/hash and retrieval contributions only under the originating or equivalently constrained visibility envelope. | AC25-50: current, historical, invisible, erased, stale, and mismatched cases return exact bytes or typed non-disclosure outcomes; bare-hit shape/cost is unchanged. |
| 55 | R25-55: Governed tracing explains source↔derived lineage, inclusion/exclusion, lifecycle/dependency state, receipt correlation, and structural integrity without exposing private content unnecessarily. | AC25-55: forward/back traces are reciprocal; exclusion reasons are deterministic; orphan/projection checks detect injected faults and operator repair preserves authority boundaries. |
| 60 | R25-60: Combined graph expansion accepts explicit/query seeds, direction, edge/target constraints, indexed predicates, snapshot, bounds, continuation, and exact path evidence. | AC25-60: constraints are honored before truncation; ordering/pages are deterministic; unsupported constraints fail typed; liveness and erasure apply to every traversed path. |
| 65 | R25-65: Candidate-selection profiles are named, deterministic, bounded, opt-in, explainable, and promoted only after preregistered retrieval-quality and efficiency gates. | AC25-65: entity/alias, dedup/diversity, complementary coverage, and fusion treatments emit replayable receipts; failed treatments cannot become defaults. |
| 70 | R25-70: Temporal and associative/diffusion profiles obey the same eligibility, snapshot, provenance, bound, and benchmark gates. | AC25-70: changed-fact/time-scoped and multi-hop fixtures prove quality without LOCOMO regression; unaccepted profiles remain opt-in/rejected. |
| 75 | R25-75: The integrated data plane demonstrates cross-SDK/wire parity, snapshot concurrency, lifecycle closure, predictable cold/steady performance, resource costs, and retrieval-only evaluation. | AC25-75: registry-installed artifacts run the same conformance fixtures; workloads report distributions/uncertainty and fail on missing execution, platform, or lifecycle evidence. |

## Architecture and interface CRUD draft

| Surface | CRUD proposal | Owning slice |
| --- | --- | ---: |
| `fathomdb-data-plane-architecture-v2.md` | Update during Slice 4, then accept as the versioned successor only after Slice 6 HITL | 4/6/7 |
| Architecture v1 and foldback v1 | Deprecate in place with successor pointer if v2 is accepted; preserve review history | 7 |
| Decision index / ADRs | Create successor decisions for identity/dependencies/actuation/lifecycle; snapshots/pagination/evidence; graph/selection; and executable measurement boundaries. Do not contradict existing accepted ADRs. | 10–70 as public contracts land |
| `dev/interfaces/{rust,python,typescript,wire}.md` | Update additively with typed requests, outcomes, errors, receipts, and serialization in the owning feature slice | 15–75 |
| Schema/migration design | Create only when the owning slice proves persisted state is required; use additive forward migrations and property tests | 15–55 |
| Public documentation | Update only for accepted, implemented application/operator behavior; do not publish proposed names | 15–75 |

## Exact allocation

All feature work above is now present in the corresponding section of
`fathomdb-data-plane-foldback-v2.md`. Repository preparation stays in Slice 7:
release-bound Python proof, dependency/security maintenance, release-state
branch semantics, and bounded authority-pointer hygiene. No Slice 10+ feature
is assigned to Slice 7.

## Non-needs retained

FathomDB does not decide what is worth remembering, meaning, entity identity,
ontology, contradiction/truth, query intent/decomposition, multi-hop strategy,
global synthesis, context packing, answer construction/verification,
provider/model choice, spend, consent, retention policy, or HITL policy. The
caller may record those decisions; FathomDB validates and actuates them.
