---
title: FathomDB data-plane architecture v2
status: ACTIVE
architecture_version: 2.1
implementation_status: active authority; implemented incrementally by feature slices
target_release: 0.8.25
baseline: dev/design/fathomdb-data-plane-architecture-v1.md
approval_status: APPROVED
approved_by: 0.8.25 Slice 6 HITL seq-272..274
activation_gate: 0.8.25 Slice 7 S7-07 GREEN
activated_on: 2026-09-02
---

# FathomDB data-plane architecture v2

## Purpose and boundary

FathomDB is the durable, provenance-preserving data plane for an agent-memory
system. It owns mechanisms and invariants. A separate semantic component owns
task intent and semantic policy.

Architecture v2 is a multi-release destination. The approved
[`0.8.25 scope adjustment`](../plans/0.8.25/scope-adjustment-2026-09-02.md)
delivers its essential core first and stages the remaining constraints without
weakening or silently deleting them.

FathomDB owns identity, source linkage, dependency representation, validity,
lifecycle transitions, erasure, projection readiness, deterministic retrieval
primitives, governed mutation, structural explanation, and integrity checks.
The semantic component owns extraction, entity resolution, contradiction and
truth judgments, query decomposition, evidence planning, synthesis, answer
verification, model choice, spend, and HITL policy.

The boundary is complete when a semantic component can implement its policy
through governed FathomDB APIs without raw SQL, private shadow indexes,
duplicated liveness rules, or manual projection cleanup.

## Architecture destination and 0.8.25 profile

Architecture v2 describes the multi-release destination. Version 2.1 adds the
approved executable profile for 0.8.25; it does not claim that every destination
mechanism ships in that release.

The 0.8.25 profile is deliberately narrower:

- one immutable canonical-source-to-derived dependency per derived revision;
- one bounded atomic actuation transaction with compact terminal receipts;
- direct-dependent lifecycle/erasure closure;
- mandatory eligibility-before-ranking and optional self-contained frozen read
  contexts, without persisted snapshot leases;
- core projection generations/readiness;
- canonical and `operational_state` keyset pages, without graph pagination;
- stateless compact evidence references for one source;
- bounded one-call reciprocal tracing and read-only integrity checks;
- deterministic one-page constrained combined graph expansion; and
- representative installed/release verification rather than an exhaustive
  scale-by-feature-by-CUDA matrix.

Designs deferred beyond that profile are inventoried in
[`0.8.x-after-0.8.25-design-notes.md`](0.8.x-after-0.8.25-design-notes.md).
An active 0.8.25 slice design must not expose a deferred destination type or
behavior merely because it appears later in this document.

## Durable artifact classes

Architecture v2 distinguishes three classes that v1 treated too broadly:

1. **Canonical source records** preserve caller content, source identity,
   immutable record revisions, source versions, exact locators, and hashes.
2. **Caller-authored derived semantic records** preserve Memex-decided facts,
   relationships, passages, summaries, and metadata with explicit dependencies.
   Their meaning remains caller-owned; FathomDB need not be able to regenerate
   them without caller input.
3. **Engine-owned projections** are rebuildable filter, lexical, vector, graph,
   and rank structures derived from the first two classes.

Canonical source records remain authoritative for source bytes. Derived
semantic records are authoritative only for the caller decision they record.
Engine projections are never a semantic or source authority.

## Core invariants

1. Logical/content/passage identity and immutable revision identity are
   separate. A positional write cursor is not a record revision.
2. Every source-backed artifact has a caller source version, locator, and
   canonical hash sufficient to resolve exact evidence.
3. Every source-backed derived semantic record declares governed dependencies.
   The 0.8.25 profile admits one canonical source; later profiles may admit
   derived or multi-source sets with caller-chosen Engine-known liveness rules.
4. Caller-decided mutations are validated and committed atomically. A rejected
   operation rejects the whole batch; per-operation diagnostics never imply a
   partial commit.
5. Lifecycle and erasure propagate through registered dependencies, fail
   closed during incomplete propagation, and prove that no active/searchable
   orphan remains before reporting completion.
6. Retrieval returns evidence, not a user-facing semantic conclusion.
7. Model, provider, network, GPU, and paid execution remain caller policy.
8. New query capability extends typed governed contracts; it does not add an
   ad hoc query language.

## Read and eligibility model

`ReadView` continues to express caller visibility and valid-time policy. An
Engine-minted frozen read context binds the effective view, validity instant,
canonical high-water point, projection generation, and hard eligibility
envelope used across compatible reads. In 0.8.25 it is an optional,
self-contained context with reproduce-or-fail semantics; persisted expiring
snapshot leases are a later profile. Mismatched, drifted, or unavailable state
fails with typed outcomes.

Eligibility predicates extend the existing allowlisted grammar. Equality and
range remain; membership and existence are admitted only when a declared native
projection supports them. Eligibility is applied before lexical, vector, or
graph candidate truncation. Unsupported predicates fail closed.

Ranked search remains bounded top-K. In 0.8.25, canonical and current-state
reads use compact opaque keyset cursors bound to request, read boundary,
generation, and ordering. General graph and long-lived snapshot-bound
continuation are later profiles.

## Mutation and lifecycle model

A typed actuation batch lets the caller submit already-decided mutations.
0.8.25 admits canonical/derived writes, one-source dependencies, and lifecycle
actions. Later profiles may add fact/edge conveniences and richer caller
verdict variants. FathomDB validates identities, references, allowed
transitions, dependency structure, and projection contracts, then commits the
batch atomically.

0.8.25 mutation receipts identify operation/digest, compact affected IDs,
resulting boundary, projection/closure references, and whole-batch refusal
reasons. Repeating an operation ID is idempotent and returns the recorded
outcome. Exhaustive state/dependency/consequence receipts are a later profile.

Reactivation uses the existing lifecycle transition to `active`; architecture
v2 does not add an SDK verb named `restore`. Operator recovery remains behind
the established recovery boundary.

## Evidence, graph, and explanation

`SearchHit` remains compact. In 0.8.25 an opt-in stateless Engine-owned evidence
reference resolves under its bound read context and eligibility envelope to the
one exact source revision, locator, bytes/span, hash, effective validity,
lifecycle state, projection origin, retrieval contribution, dependency, and
supersession status. Persisted leases and multi-source replay are later
profiles. A stale reference never grants access to invisible or erased bytes.

Combined graph expansion reuses the existing graph store. Its 0.8.25 governed
request supports query or explicit seeds, incoming/outgoing/both directions,
canonical edge kind, target kind, indexed predicates, bounded depth/work/result
size, one read context, deterministic one-page targets, and compact graph
origin. Deterministic continuation and full path evidence are later profiles.
Consumer vocabulary such as a Memex slot maps to these generic constraints
outside the Engine.

0.8.25 structural explanation covers included results, retrieval/reranking
contributions, fallback/degraded state, projection/graph origin,
dependency/lifecycle status, and privacy-safe operation correlation. Expanded
exclusion/not-selected explanation is a later profile.
Semantic entailment and answer correctness remain external.

## Deterministic retrieval improvement

FathomDB may expose deterministic entity/alias matching, duplicate suppression,
diversity, complementary-evidence selection, coverage selection, candidate
fusion, temporal retrieval, or associative graph diffusion only through named,
bounded profiles that pass preregistered data-plane quality and performance
gates. Memex decides when to invoke an accepted profile. A failed treatment is
retained as experimental evidence and does not change a default.

## Measurement boundary

Every result is classified as data-plane, semantic-control-plane, or end to end.
Receipts state whether `Engine.search` ran and identify shared and differing
components. GLOBAL-01's initial storage-backed witness bypassed `Engine.search`;
its held-out comparison used search but also caller planning and answer
generation. Neither is a retrieval-only comparison.

Release verification therefore adds a native retrieval-only global witness and
separately reports answer-system results. No answerer or semantic judge may be
hidden inside a FathomDB data-plane claim.

## Compatibility and safety

- Every new persisted or public request and response carries an explicit
  integer schema/wire version. New revision, dependency, actuation-batch,
  read-context, cursor, evidence, and graph types require Rust, Python,
  TypeScript, wire, and applicable Windows CPU/native parity in their owning
  feature slice.
- Unknown request fields or request variants reject with a typed
  unsupported-version outcome before execution; they are never ignored or
  defaulted into a mutation, visibility, retrieval, or continuation decision.
- Older readers may ignore additive unknown response fields. An unknown
  response variant that affects identity, lifecycle, visibility, mutation,
  ranking, evidence, or continuation rejects with a typed unsupported-version
  outcome and is never mapped to a default variant.
- An older writer must not mutate, rebuild, or reindex a persisted artifact
  whose newer version it cannot fully interpret. Read-only inspection may
  proceed only where that type's version contract explicitly defines it as
  safe.
- Existing bare search-result shape and default retrieval remain unchanged.
- Existing single-source records remain valid. 0.8.25 keeps the provenance
  representation additively extensible but enforces a zero/one-source bound;
  multi-source behavior requires later requirements and design review.
- Governed current-state reads expose the existing `operational_state` table
  and require no data migration. `latest_state` remains consumer terminology,
  not a FathomDB table, lifecycle axis, or semantic-truth policy.
- Erasure, supersession, access, snapshot, and binding tests precede performance
  or readiness claims.

## Slice 4 architecture reconciliation

The completed 0.8.25 Slice 4 review adds seven named constraints. They are part
of architecture v2 rather than optional implementation guidance:

| Constraint | Architectural decision | Owning slices |
| --- | --- | --- |
| A25-01 | A frozen read is defined by reproducible canonical boundary, validity instant, projection generation, and eligibility envelope. 0.8.25 provides optional self-contained reproduce-or-fail context with typed unavailable, mismatch, and drift outcomes; persisted leases add expiry only in a later profile. It does not promise a permanently held SQLite reader transaction. | 35 |
| A25-02 | Exact locators are UTF-8 byte ranges over a named immutable revision, using a declared canonical byte representation and hash algorithm; invalid boundaries and hash mismatch reject typed. | 15, 50 |
| A25-03 | FathomDB exposes governed current operational state backed by `operational_state`; `latest_state` remains a consumer concept. | 45 |
| A25-04 | Multi-source dependencies use a bounded Engine-known liveness grammar. `all_required` and `any_surviving` have distinct removal behavior; unsupported rules reject typed. | 0.8.26 successors to the 20/30 core |
| A25-05 | Every new persisted or public type defines schema/wire version and unknown-field/unknown-variant behavior, with feature-local Rust, Python, TypeScript, and applicable Windows CPU/native proof. | active 15–60; audit 75; future slices retain the rule |
| A25-06 | Eligibility and graph constraints execute before seed/candidate truncation and expansion. 0.8.25 reports compact inclusion/degradation state; expanded deterministic ineligible/not-selected explanation follows only when justified. | 35/55/60 core; expanded explanation 0.8.28 |
| A25-07 | Default `SearchHit` remains compact. Evidence handles are opt-in, bound to the originating visibility envelope, and resolve stale or invisible state only to typed non-disclosure. | 50 |

Architecture v1 remains historical. Accepted/locked predecessor designs remain
evidence and are not rewritten; each 0.8.25 feature design records whether it
reuses, amends, or supersedes them.

## Release allocation

The complete allocation is
[`plan-0.8.25.md`](../plans/plan-0.8.25.md). The requirement inventory is
[`memex-0.6.0-needs-in-fathomdb-0.8.25.md`](../plans/memex-0.6.0-needs-in-fathomdb-0.8.25.md),
and the delivery method is
[`fathomdb-data-plane-foldback-v2.md`](../plans/fathomdb-data-plane-foldback-v2.md).

Slice 4 review, Slice 5 verification analysis, and Slice 6 HITL approval are
complete. Architecture v2 became the active versioned successor when S7-07
recorded complete GREEN at `007a3152`. The rest of Slice 7 remains the repository
preparation gate: Slice 10+ designs may be drafted and reviewed, but cannot
become READY until Slice 7 records completion.
