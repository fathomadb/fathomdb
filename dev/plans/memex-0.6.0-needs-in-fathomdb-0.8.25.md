---
title: Memex 0.6.0 needs in FathomDB 0.8.25
status: ACTIVE
target_release: 0.8.25
decision_source: HITL 2026-08-31; owner scope adjustment 2026-09-02
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

## Post-design scope disposition

This crosswalk remains the complete needs inventory, not a promise that every
gap ships in 0.8.25. The owner-approved
[`scope-adjustment-2026-09-02.md`](0.8.25/scope-adjustment-2026-09-02.md)
supersedes the original release allocation where they conflict.

| Needs | 0.8.25 disposition | Later allocation |
| --- | --- | --- |
| 1–2 | Retain complete Slice 15 identity/provenance scope. | None. |
| 3, 17–18 | Retain bounded atomic actuation and compact receipt. | Broader operations and complete consequence receipts: 0.8.26. |
| 4 | Retain canonical-source-to-derived registration and lookup. | Multi-source/derived-to-derived/liveness: 0.8.26. |
| 5–6 | Retain lifecycle/erasure closure over the core dependency model. | Extend closure with later dependency forms in their owning release. |
| 7–8, 21 contract | Retain eligibility-before-ranking and optional compact frozen reads. | Full snapshot leases: 0.8.27. |
| 9 and current state | Retain minimal canonical/state pagination and point reads. | Full cursors, generalized graph pages, richer state continuation: 0.8.27. |
| 10–11 | Retain compact evidence identity and eligibility-bound resolution. | Persisted evidence receipt/replay: 0.8.27. |
| 12, 19–20 | Retain compact graph origin, reciprocal trace, orphan/projection checks, and inclusion/degradation explanation. | Rich paths/exclusion: 0.8.28; full jobs/repair: experimental review at 0.8.33. |
| 13 | Retain core generation identity/readiness and compact correlation. | Richer public work-manifest surface requires a later concrete need. |
| 14 | Deferred from 0.8.25. | Multi-source provenance/liveness: 0.8.26. |
| 15 | Retain minimal graph constraint parity. | Rich continuation/path replay: 0.8.28. |
| 16 | Removed from committed 0.8.25 scope. | Temporal/manual profiles: 0.8.28; candidate selection: review 0.8.29; associative/routing: review 0.8.31. |
| 21–24 measurement | Retain representative installed parity, lifecycle/concurrency/performance regression, and retrieval-only proof. | Exhaustive matrices: experimental review at 0.8.33. |

## P0 — clean integration

| # | Need | Existing FathomDB mechanism | Gap and 0.8.25 disposition | Slice |
| ---: | --- | --- | --- | ---: |
| 1 | Durable identity | Typed stable `IdSpace` covers logical, content, and passage identities across restart/reindex. | **Partial.** `write_cursor` is positional and reassigned; add immutable record-revision identity while preserving `IdSpace`. | 15 |
| 2 | Canonical source provenance | `source_id` is mandatory on new writes and carried by every search-hit path. | **Partial.** Add caller source-version identity, exact whole-body/byte-span locator, immutable revision, and canonical content hash. | 15 |
| 3 | Atomic semantic actuation | `Engine.write` atomically commits canonical nodes, edges, and op-store items. Lifecycle and provider-driven consolidation are separate calls. | **Missing.** Add a bounded typed, model-free core batch in 0.8.25; broader operation coverage moves to 0.8.26. | 25 core; 0.8.26 extension |
| 4 | Dependency registration | Row-owned projection linkage exists internally. | **Missing.** Add canonical-source-to-derived identity/lookup in 0.8.25; multi-source and general derived-to-derived forms move to 0.8.26. | 20 core; 0.8.26 extension |
| 5 | Lifecycle closure | Governed `transition`, supersession, `purge`, validity, and projection maintenance exist. | **Partial.** Add dependency propagation, visibility fencing, idempotent/resumable operation identity, and consequence receipts. Reactivation uses `transition(..., active)`; no SDK `restore` verb is added. | 30 |
| 6 | Erasure | `erase_source` and `purge` remove canonical rows, row-owned projections, telemetry references, and WAL residue or fail typed. | **Partial.** Extend closure to registered dependency sets and prove no active/searchable dependent remains. | 30 |
| 7 | Validity and versioned reads | Strict `ReadView`, valid-as-of reads, historical relaxations, and boundary-crossing detection exist. | **Partial.** Add an optional compact Engine-minted frozen read in 0.8.25; full cross-operation lease behavior moves to 0.8.27. | 35 core; 0.8.27 extension |
| 8 | Eligibility before ranking | Closed `SearchFilter`, declared attribute projections, and allowlisted read predicates provide equality/range subsets. | **Partial.** Unify eligibility across lexical/vector/graph paths; add indexed membership/existence where justified; reject unsupported predicates before execution. | 35 |
| 9 | Governed pagination | Op-store reads have `after_id`; ranked searches have bounded top-K limits. | **Missing for canonical/current-state reads.** Add minimal stable continuation in 0.8.25; full lease-bound and graph pagination moves to 0.8.27. Ranked top-K remains distinct. | 45 core; 0.8.27 extension |
| 10 | Source-complete evidence | Compact `SearchHit` carries ID, body, source, score/branch, and optional CE score. | **Missing.** Add compact opt-in `EvidenceRef` plus exact resolver in 0.8.25 without default-hit growth; persisted receipts/replay move to 0.8.27. | 50 core; 0.8.27 extension |
| 11 | Eligibility-bound evidence | Current callers can re-read through `ReadView`, but no evidence-reference contract exists. | **Missing.** Enforce original or equivalent eligibility and typed non-disclosure in the compact resolver; richer replay moves to 0.8.27. | 50 core; 0.8.27 extension |
| 12 | Retrieval explanation | Query traces, per-hit ranks/scores, soft fallback, projection cursor, and privacy-safe query IDs exist. | **Partial.** Add dependency/supersession status, graph edge/path origin, exclusion reasons, degraded operation, and receipt correlation. | 50, 55, 60 |
| 13 | Projection management | Declarative filterable/searchable/rankable projections, synchronous lexical updates, asynchronous dense readiness, status, drain, and operator rebuild exist. | **Partial.** Add durable projection-generation identity and correlate mutation-to-ready, blocked/deferred/degraded work. Keep recovery-denylist verbs off the SDK. | 40 |

## P1 — advanced memory

| # | Need | Existing FathomDB mechanism | Gap and 0.8.25 disposition | Slice |
| ---: | --- | --- | --- | ---: |
| 14 | Multi-source provenance | Single-source provenance is complete for row-owned artifacts. | **Missing.** Deferred to 0.8.26 with bounded source-set liveness; 0.8.25 ships only the single-source core. | 0.8.26 |
| 15 | Constrained graph expansion | Direct neighbors support bounded BFS, direction, depth, and `ReadView`; combined search expansion uses the same graph store. | **Partial.** Add honored seed/direction/edge-kind/target-kind/bound/eligibility constraints and deterministic one-page results in 0.8.25. Rich continuation/path replay moves to 0.8.28. | 60 core; 0.8.28 extension |
| 16 | Deterministic candidate selection | FTS, vector, RRF, CE, graph arm, temporal validity, and candidate limits exist. Prior graph/global/multi-query treatments were not accepted as defaults. | **Experimental.** Manual temporal/profile work is reconsidered in 0.8.28; candidate selection at 0.8.29; associative/routing at 0.8.31. Failed treatments remain durable negative evidence. | 0.8.28; reviews 0.8.29/0.8.31 |
| 17 | Atomic consolidation application | `consolidate_with_provider` lets FathomDB invoke a caller provider and then mutate state. | **Missing model-free actuation.** Accept an already-decided coexist/supersede/invalidate/merge verdict inside the atomic semantic batch. | 25 |
| 18 | Complete mutation receipts | `WriteReceipt` has a high-water cursor, row cursors, and dangling endpoint count; erasure has a compact report. | **Partial target.** Return compact committed/refused actuation receipts in 0.8.25; complete consequence receipts move to 0.8.26. | 25 core; 0.8.26 extension |
| 19 | Operational tracing | Search explanation, source IDs, projection status, telemetry correlation, and orphan-provenance doctor checks exist. | **Partial.** Add source-to-derived and artifact-to-source traversal, visibility/exclusion explanation, and mutation/retrieval/lifecycle receipt correlation. | 55 |
| 20 | Integrity and maintenance | Structural/projection checks, erasure canaries, status/drain, registry-driven projection invalidation, operator rebuild, and backup/recovery contracts exist. | **Partial.** Add core dependency-orphan/projection checks in 0.8.25; full job/repair orchestration is experimental for 0.8.33 review. Semantic correctness remains Memex-owned. | 55 core; review 0.8.33 |

## P2 — competitive and scale

| # | Need | Existing FathomDB mechanism | Gap and 0.8.25 disposition | Slice |
| ---: | --- | --- | --- | ---: |
| 21 | Concurrent read/write guarantees | WAL, reader workers, deferred reader transactions, stress tests, and typed storage/runtime failures exist. | **Partial for new contracts.** Define snapshot behavior and typed drift outcomes during mutation in Slice 35; measure cold/steady contention in Slice 75. | 35, 75 |
| 22 | Predictable local performance | Accepted FTS envelope reaches 50k with exact top-100 equivalence; graph traversal is bounded. | **Partial for new contracts.** Measure evidence, pagination, dependencies, mutation-to-ready, erasure propagation, and projection-generation overhead. | 75 |
| 23 | Cross-SDK parity | Governed facade parity and typed errors are established across Rust, Python, and TypeScript. | **Partial.** Fix the known Rust inability to name `SearchHit`/`IdSpace`; require Rust/Python/TypeScript parity, versioned wire fixtures, and applicable Windows CPU/native proof in every retained public or persisted feature slice. Slice 75 audits the combined installed surface. | 15–60, 75 |
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
