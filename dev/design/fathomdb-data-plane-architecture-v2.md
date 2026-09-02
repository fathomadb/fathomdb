---
title: FathomDB data-plane architecture v2
status: COMPLETE
architecture_version: 2
implementation_status: planned
target_release: 0.8.25
baseline: dev/design/fathomdb-data-plane-architecture-v1.md
approval_status: APPROVED
approved_by: 0.8.25 Slice 6 HITL seq-272..274
activation_gate: 0.8.25 Slice 7 completion
---

# FathomDB data-plane architecture v2

## Purpose and boundary

FathomDB is the durable, provenance-preserving data plane for an agent-memory
system. It owns mechanisms and invariants. A separate semantic component owns
task intent and semantic policy.

FathomDB owns identity, source linkage, dependency representation, validity,
lifecycle transitions, erasure, projection readiness, deterministic retrieval
primitives, governed mutation, structural explanation, and integrity checks.
The semantic component owns extraction, entity resolution, contradiction and
truth judgments, query decomposition, evidence planning, synthesis, answer
verification, model choice, spend, and HITL policy.

The boundary is complete when a semantic component can implement its policy
through governed FathomDB APIs without raw SQL, private shadow indexes,
duplicated liveness rules, or manual projection cleanup.

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
3. Every derived semantic record declares its canonical or derived
   dependencies. Multi-source records declare a source set and a caller-chosen
   liveness rule; the Engine enforces but does not choose that rule.
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
Engine-minted frozen snapshot binds the effective view, validity instant,
canonical high-water point, projection generation, and hard eligibility
envelope used across search, graph traversal, pagination, and evidence
resolution. Expired, mismatched, or drifted snapshots fail with typed outcomes.

Eligibility predicates extend the existing allowlisted grammar. Equality and
range remain; membership and existence are admitted only when a declared native
projection supports them. Eligibility is applied before lexical, vector, or
graph candidate truncation. Unsupported predicates fail closed.

Ranked search remains bounded top-K. Ordered list, graph, mutation-log, and
current-state reads use opaque cursors bound to their request, snapshot,
projection generation, and ordering.

## Mutation and lifecycle model

A typed semantic batch lets the caller submit already-decided canonical and
derived writes, dependencies, facts/edges, lifecycle actions, consolidation
verdicts, and policy metadata. FathomDB validates identities, references,
allowed transitions, dependency cycles, and projection contracts, then commits
the batch atomically.

Mutation receipts identify the caller operation and policy version, created and
affected IDs, previous/resulting states, dependency changes, snapshot/write
boundary, projection work/readiness, lifecycle/erasure consequences, and
whole-batch refusal reasons. Repeating an operation ID is idempotent and returns
the recorded outcome.

Reactivation uses the existing lifecycle transition to `active`; architecture
v2 does not add an SDK verb named `restore`. Operator recovery remains behind
the established recovery boundary.

## Evidence, graph, and explanation

`SearchHit` remains compact. An opt-in Engine-owned evidence reference resolves
under its bound snapshot and eligibility envelope to the exact record revision,
source version, locator, bytes/span, hash, effective validity, lifecycle state,
projection origin, retrieval contribution, dependencies, and supersession
status. A stale reference never grants access to invisible or erased bytes.

Combined graph expansion reuses the existing graph store. Its governed request
supports query or explicit seeds, incoming/outgoing/both directions, canonical
edge kind, target kind, indexed predicates, bounded depth/page size, frozen
snapshot, deterministic continuation, and exact seed/edge/path explanation.
Consumer vocabulary such as a Memex slot maps to these generic constraints
outside the Engine.

Structural explanation covers why an artifact was included or excluded,
retrieval/reranking contributions, fallback/degraded state, projection/path
origin, dependency/lifecycle status, and privacy-safe operation correlation.
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

- New revision, dependency, semantic-batch, snapshot, cursor, evidence, and
  graph request/response types require Rust, Python, TypeScript, and wire parity.
- Existing bare search-result shape and default retrieval remain unchanged.
- Existing single-source records remain valid; multi-source provenance is
  additive.
- Governed current-state reads expose the existing `operational_state` table
  and require no data migration. `latest_state` remains consumer terminology,
  not a FathomDB table, lifecycle axis, or semantic-truth policy.
- Unknown mutation, predicate, cursor, graph, or evidence fields fail closed.
- Erasure, supersession, access, snapshot, and binding tests precede performance
  or readiness claims.

## Slice 4 architecture reconciliation

The completed 0.8.25 Slice 4 review adds seven named constraints. They are part
of architecture v2 rather than optional implementation guidance:

| Constraint | Architectural decision | Owning slices |
| --- | --- | --- |
| A25-01 | A frozen snapshot is defined by reproducible canonical boundary, validity instant, projection generation, and eligibility envelope, with typed unavailable, drift, and expiry outcomes. It does not promise a permanently held SQLite reader transaction. | 35 |
| A25-02 | Exact locators are UTF-8 byte ranges over a named immutable revision, using a declared canonical byte representation and hash algorithm; invalid boundaries and hash mismatch reject typed. | 15, 50 |
| A25-03 | FathomDB exposes governed current operational state backed by `operational_state`; `latest_state` remains a consumer concept. | 45 |
| A25-04 | Multi-source dependencies use a bounded Engine-known liveness grammar. `all_required` and `any_surviving` have distinct removal behavior; unsupported rules reject typed. | 20, 30 |
| A25-05 | Every new persisted or public type defines schema/wire version and unknown-field/unknown-variant behavior, with feature-local Rust, Python, TypeScript, and applicable Windows CPU/native proof. | 15–70; audit 75 |
| A25-06 | Eligibility and graph constraints execute before seed/candidate truncation and expansion; explanation distinguishes ineligible, not selected, unavailable, and degraded outcomes. | 35, 55, 60 |
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
complete. Architecture v2 is approved as the versioned successor. It becomes
the active implementation authority when Slice 7 records completion; until
then, Slice 10+ designs may be drafted and reviewed but cannot become READY.
