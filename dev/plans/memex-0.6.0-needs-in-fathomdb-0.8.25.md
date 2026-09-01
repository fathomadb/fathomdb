---
title: Memex 0.6.0 needs in FathomDB 0.8.25
status: ACTIVE
target_release: 0.8.25
decision_source: HITL 2026-08-31
---

# Memex 0.6.0 needs in FathomDB 0.8.25

## Purpose

This is the durable, code-grounded crosswalk between the complete Memex 0.6.0
data-plane needs inventory and FathomDB's current mechanisms. It allocates every
confirmed gap to the dependency-linear 0.8.25 ladder without assigning Memex's
semantic policy to FathomDB.

Status terms are strict:

- **Present:** the required mechanism exists on the governed public path.
- **Partial:** a useful substrate exists but the stated contract is incomplete.
- **Missing:** the mechanism has no governed equivalent.
- **Qualification:** candidates exist or can be built, but adoption requires a
  registered quality/performance decision.

## P0 — clean integration

| # | Need | Existing FathomDB mechanism | Gap and 0.8.25 disposition | Slice |
| ---: | --- | --- | --- | ---: |
| 1 | Durable identity | Typed stable `IdSpace` covers logical, content, and passage identities across restart/reindex. | **Partial.** `write_cursor` is positional and reassigned; add immutable record-revision identity while preserving `IdSpace`. | 15 |
| 2 | Canonical source provenance | `source_id` is mandatory on new writes and carried by every search-hit path. | **Partial.** Add caller source-version identity, exact whole-body/byte-span locator, immutable revision, and canonical content hash. | 15 |
| 3 | Atomic semantic actuation | `Engine.write` atomically commits canonical nodes, edges, and op-store items. Lifecycle and provider-driven consolidation are separate calls. | **Missing.** Add one typed, model-free batch for caller-decided canonical, derived, dependency, fact/edge, lifecycle, and metadata operations. | 25 |
| 4 | Dependency registration | Row-owned projection linkage exists internally. | **Missing.** Add explicit canonical-to-derived, derived-to-derived, source-set, and queryable dependency identities. | 20 |
| 5 | Lifecycle closure | Governed `transition`, supersession, `purge`, validity, and projection maintenance exist. | **Partial.** Add dependency propagation, visibility fencing, idempotent/resumable operation identity, and consequence receipts. Reactivation uses `transition(..., active)`; no SDK `restore` verb is added. | 30 |
| 6 | Erasure | `erase_source` and `purge` remove canonical rows, row-owned projections, telemetry references, and WAL residue or fail typed. | **Partial.** Extend closure to registered dependency sets and prove no active/searchable dependent remains. | 30 |
| 7 | Validity and versioned reads | Strict `ReadView`, valid-as-of reads, historical relaxations, and boundary-crossing detection exist. | **Partial.** Add one Engine-minted frozen snapshot usable across search, graph, pagination, and evidence resolution with typed drift/expiry. | 35 |
| 8 | Eligibility before ranking | Closed `SearchFilter`, declared attribute projections, and allowlisted read predicates provide equality/range subsets. | **Partial.** Unify eligibility across lexical/vector/graph paths; add indexed membership/existence where justified; reject unsupported predicates before execution. | 35 |
| 9 | Governed pagination | Op-store reads have `after_id`; ranked searches have bounded top-K limits. | **Missing for canonical/graph/current-state reads.** Add opaque request/snapshot/generation/order-bound cursors. Ranked top-K remains distinct from ordered pagination. | 45 |
| 10 | Source-complete evidence | Compact `SearchHit` carries ID, body, source, score/branch, and optional CE score. | **Missing.** Add opt-in Engine-owned `EvidenceRef` and resolver for exact revision, source version, bytes/span/hash, validity, lifecycle, projection, dependency, and ranking evidence. | 50 |
| 11 | Eligibility-bound evidence | Current callers can re-read through `ReadView`, but no evidence-reference contract exists. | **Missing.** Bind resolution to the originating or equivalently constrained snapshot and hard predicates; return typed invisible/erased/mismatched/unavailable outcomes. | 50 |
| 12 | Retrieval explanation | Query traces, per-hit ranks/scores, soft fallback, projection cursor, and privacy-safe query IDs exist. | **Partial.** Add dependency/supersession status, graph edge/path origin, exclusion reasons, degraded operation, and receipt correlation. | 50, 55, 60 |
| 13 | Projection management | Declarative filterable/searchable/rankable projections, synchronous lexical updates, asynchronous dense readiness, status, drain, and operator rebuild exist. | **Partial.** Add durable projection-generation identity and correlate mutation-to-ready, blocked/deferred/degraded work. Keep recovery-denylist verbs off the SDK. | 40 |

## P1 — advanced memory

| # | Need | Existing FathomDB mechanism | Gap and 0.8.25 disposition | Slice |
| ---: | --- | --- | --- | ---: |
| 14 | Multi-source provenance | Single-source provenance is complete for row-owned artifacts. | **Missing.** Represent source sets and caller-declared dependency-liveness rules without making semantic entailment decisions. | 20 |
| 15 | Constrained graph expansion | Direct neighbors support bounded BFS, direction, depth, and `ReadView`; combined search expansion uses the same graph store. | **Partial.** Add query/explicit seeds, direction, `edge_kind`, target kind, predicates, frozen snapshot, deterministic pages, and exact edge/path evidence to combined expansion. | 60 |
| 16 | Deterministic candidate selection | FTS, vector, RRF, CE, graph arm, temporal validity, and candidate limits exist. Prior graph/global/multi-query treatments were not accepted as defaults. | **Qualification.** Evaluate opt-in entity/alias, complementarity, diversity/dedup, coverage, fusion, temporal, and associative/diffusion primitives under preregistered gates. Failed treatments remain rejected evidence. | 65, 70 |
| 17 | Atomic consolidation application | `consolidate_with_provider` lets FathomDB invoke a caller provider and then mutate state. | **Missing model-free actuation.** Accept an already-decided coexist/supersede/invalidate/merge verdict inside the atomic semantic batch. | 25 |
| 18 | Complete mutation receipts | `WriteReceipt` has a high-water cursor, row cursors, and dangling endpoint count; erasure has a compact report. | **Missing.** Return operation/policy identity, affected stable IDs, before/after states, dependencies, snapshot, projection work/readiness, consequences, and whole-batch refusal reasons. | 25 |
| 19 | Operational tracing | Search explanation, source IDs, projection status, telemetry correlation, and orphan-provenance doctor checks exist. | **Partial.** Add source-to-derived and artifact-to-source traversal, visibility/exclusion explanation, and mutation/retrieval/lifecycle receipt correlation. | 55 |
| 20 | Integrity and maintenance | Structural/projection checks, erasure canaries, status/drain, registry-driven projection invalidation, operator rebuild, and backup/recovery contracts exist. | **Partial.** Add dependency-orphan/source-set checks and propagation observability; semantic correctness remains Memex-owned. | 55 |

## P2 — competitive and scale

| # | Need | Existing FathomDB mechanism | Gap and 0.8.25 disposition | Slice |
| ---: | --- | --- | --- | ---: |
| 21 | Concurrent read/write guarantees | WAL, reader workers, deferred reader transactions, stress tests, and typed storage/runtime failures exist. | **Partial for new contracts.** Define snapshot behavior and typed drift outcomes during mutation in Slice 35; measure cold/steady contention in Slice 75. | 35, 75 |
| 22 | Predictable local performance | Accepted FTS envelope reaches 50k with exact top-100 equivalence; graph traversal is bounded. | **Partial for new contracts.** Measure evidence, pagination, dependencies, mutation-to-ready, erasure propagation, and projection-generation overhead. | 75 |
| 23 | Cross-SDK parity | Governed facade parity and typed errors are established across Rust, Python, and TypeScript. | **Partial.** Fix the known Rust inability to name `SearchHit`/`IdSpace`; require Rust/Python/TypeScript parity, versioned wire fixtures, and applicable Windows CPU/native proof in every public or persisted feature slice. Slice 75 audits the combined installed surface. | 15–70, 75 |
| 24 | Data-plane evaluation support | Typed experiment receipts and the completed performance program exist. | **Partial.** Make measurement classification executable and add retrieval-only receipts, including a native `Engine.search` global witness. | 10, 75 |

## Additional Memex integration need — current operational state

The schema already maintains authoritative current rows in
`operational_state`, but governed reads expose only the append-only mutation
log. Slice 45 adds current-state point lookup and ordered pagination over the
existing table. `latest_state` remains a Memex/consumer concept. This is a
read-surface addition, not a data migration.

## Memex-side modernization, not FathomDB scope

Memex should remove workarounds as their governed equivalents become available:

- use existing lifecycle transitions and purge instead of tombstone emulation;
- use native mutation-log continuation instead of complete scans;
- map owner/scope/status/expiration onto native validity and projections;
- pass graph direction and relation constraints once Slice 60 lands; and
- replace client-side `latest_state` collapse after Slice 45.

These adapter changes do not justify duplicate FathomDB mechanisms.

## Boundary

FathomDB validates and executes typed decisions atomically, maintains identity,
dependencies, visibility, indexes, lifecycle, provenance, erasure, explanation,
and reversibility. Memex decides what is memory, what records mean, entity and
ontology policy, contradiction/truth, query intent, reasoning, synthesis,
answers, models, spend, consent, retention, and HITL policy.

The integration is complete when Memex can implement those policies through
governed FathomDB APIs without raw SQL, private dependency indexes, duplicated
liveness logic, or manual projection cleanup.
