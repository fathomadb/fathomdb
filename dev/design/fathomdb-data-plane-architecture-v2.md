---
title: FathomDB data-plane architecture v2
status: ACTIVE
architecture_version: 2.2
implementation_status: active source authority for the integrated profile
target_release: 0.8.32
baseline: dev/design/fathomdb-data-plane-architecture-v1.md
approval_status: APPROVED
approved_by: 0.8.25 Slice 6 HITL seq-272..274
activation_gate: 0.8.25 Slice 7 S7-07 GREEN
activated_on: 2026-09-02
last_reconciled: 2026-11-26 0.8.32 workspace profile (13 members including publishable fathomdb-mcp and fathomdb-inspector; Slice 41 ADR #103)
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

## Current integrated workspace profile

### Workspace and application SDK ownership

The current Rust workspace has **thirteen** members: `fathomdb`, `fathomdb-sdk`,
`fathomdb-engine`, `fathomdb-cli`, `fathomdb-query`, `fathomdb-schema`,
`fathomdb-embedder-api`, `fathomdb-embedder`, `fathomdb-py`, `fathomdb-napi`,
the publishable `fathomdb-mcp` binary, the internal `fathomdb-tc5-benchmark`, and
`fathomdb-inspector` (`src/inspector/`). The dedicated
Rust application SDK is a peer of the Python and TypeScript SDKs, with the
58 live governed operation endpoints across `Engine`, `read`, `graph`, `admin` and
root-level `rerank`. The standalone `embed_batch_cls` and explicit `release_models` operations
remain outside that signed map. Arguments and defaults follow the accepted
[Rust SDK translations](../adr/ADR-0.8.27-rust-sdk-parity.md), including the
TypeScript resolution where Python and TypeScript differ. Calls are
synchronous and return `Result`, with one `Error` type and `ErrorKind`
categories. Its source owner is
`src/rust/crates/fathomdb-sdk/`; Python and TypeScript package owners remain
`src/python/` and `src/ts/`, over `fathomdb-py` and `fathomdb-napi`.

The `fathomdb` crate remains the lower-level facade and `fathomdb-cli` the
operator boundary. SDKs do not expose raw SQL, recovery or test hooks.
The [Rust SDK contract](../interfaces/rust-sdk.md) and
[generated contributor map](../../docs/generated/contributor-map.md) give
current operation and package paths. Rust engine file/module boundaries are
internal implementation ownership, not SDK contracts. The following addenda
describe the implemented diagnostic transport and native ownership boundaries.

### Python diagnostic transport

This addendum records the Slice 100 implementation under the accepted
[`ADR-0.8.27-python-subscriber-delivery`](../adr/ADR-0.8.27-python-subscriber-delivery.md).
The v2.2 approval and activation dates in this document's header apply to
the earlier data-plane model; the subscriber successor was separately
accepted on 2026-10-03 after review of this addendum.

FathomDB is a local, SQLite-backed data plane. Observability must not put
caller-supplied Python code on a connection-owning thread. A statement can
complete while the primary writer, a projection worker, or a pooled reader
still owns its `rusqlite` connection. SQLite's `sqlite3_profile` C callback
may then dispatch through the engine subscriber registry if profiling is
enabled or the statement crosses the slow threshold.
The registry snapshots subscribers without holding its lock across callbacks
and contains each Rust subscriber panic before unwinding could cross SQLite's
C boundary.

The Slice 100 Python adapter copies an event, profile, slow-statement signal,
or debug stress-failure context into a bounded queue at each engine subscriber
callback, including the SQLite profile boundary. A
separate worker resolves the caller-owned logger weakly and invokes
`logger.log`. Python filters and handlers can block or fail, so they run off
the SQLite path; full or contended queues count dropped records instead of
waiting. A handler cannot reenter a FathomDB database operation through the
binding on its delivery thread. Replacement and close detach the engine
subscription without waiting
for a blocked handler. These choices bound queued record count, avoid
recursive reader and profile callbacks, and keep handler latency out of
normal database calls. Payload bytes, including slow-statement SQL text, have
no separate hard cap.

This route serves opt-in diagnostics, not durable evidence, transaction
completion, or operation progress. The engine event has phase, source,
category, and optional code, but no operation identity. The actuation
`operation_id` elsewhere in the data plane is a persisted idempotency key; it
is not a logging correlation ID. A timer in the Python adapter therefore
cannot truthfully report operation progress. The Slice 100 successor
[`ADR-0.8.27-python-subscriber-delivery`](../adr/ADR-0.8.27-python-subscriber-delivery.md)
removes the formerly inert Python heartbeat interval. The
transport mechanics and exact payloads are owned by
[`bindings.md`](bindings.md) and [`lifecycle.md`](lifecycle.md).

### TypeScript native boundary

Slice 110 decomposes `fathomdb-napi` by responsibility while retaining one
native `Engine` class and the existing generated JavaScript registration
identity. `engine.rs` owns that class and its Arc-backed lifetime;
`execution.rs` owns the Tokio `spawn_blocking` and panic boundary; domain
modules own their N-API objects and conversions; `errors.rs` owns the typed
error envelope. `shared.rs` holds narrowly shared translation helpers,
`subscriber.rs` owns the host callback transport, and the root composes and
re-exports the modules for napi-rs registration. These are private Rust
boundaries, not additional JavaScript entrypoints. Keeping one registration
identity matters because napi-rs generates exports from attributes: moving
source text between files is safe only when the loaded runtime and generated
declarations still name the same objects and methods. The reviewed
[Slice 110 design](../plans/0.8.27/features/slice-110/design.md) and its
inventory record that reconciliation.

Ordinary Promise-returning calls retain the accepted Tokio `spawn_blocking`
handoff. It keeps blocking SQLite and engine work off the JavaScript event
loop while an Arc retains the engine through in-flight work. This binding
handoff is separate from the engine-owned scheduler and embed dispatcher;
adding another binding pool would create a competing capacity control without
changing the engine's ownership rules. N-API conversion and synchronous
accessors remain on their respective call boundaries, with panic and typed
error conversion owned by the binding.

Host diagnostics need a different route. An engine event can originate inside
SQLite's profile C callback while a connection is owned, so invoking
JavaScript or waiting for its listener there risks deadlock and couples query
latency to application code. The TypeScript adapter therefore copies records
into a bounded, best-effort Rust queue and uses a coalesced N-API wakeup to
deliver at most 64 records per JavaScript turn. It owns callback faults,
overload accounting, attachment replacement and environment teardown; none
changes the engine's durable write or receipt semantics. The queue bounds
record count, not diagnostic payload bytes. `close()` disables
delivery synchronously at JavaScript call entry, then returns its existing
`Promise<void>` for the engine close. This ordering prevents a callback from
being newly delivered after close begins even when engine close later fails.
The [accepted subscriber ADR](../adr/ADR-0.8.27-typescript-subscriber-delivery.md)
owns the exact queue, handle and payload rules. The TypeScript binding removes
its formerly inert heartbeat option because no operation-scoped engine
heartbeat producer exists; a binding timer would only show that the timer ran.
An actual future engine heartbeat event can still be forwarded.

### Open, storage, and concurrency

`Engine::open` configures the process SQLite runtime before admission. A
missing or zero-length path bootstraps directly at schema 39. A nonempty
database is admitted only when its effective committed schema and required
shape match Schema 39 (via explicit offline `fathomdb migrate` for Schema 37/38
stores; `Engine::open` performs no public automatic migration); otherwise it is
refused under the persistent admission lock before product mutation.

Canonical records, provenance, dependencies, operational state, actuation
receipts, projection state, FTS, and vector state remain in one SQLite product
database with WAL and the existing admission/ownership lock. The primary
mutex-serialized writer connection owns caller-submitted mutation transactions
and schedules post-commit projection work. Projection workers use their own
SQLite connections for async vector publication; their `BEGIN IMMEDIATE`
commits are totally ordered by the shared `commit_gate`. Pooled readers own read
transactions; frozen
authorization, graph selection, and evidence materialization do not move onto
either write lane.

This is a deliberate refinement of the 0.6.0 single-writer architecture. Its
one-primary-write-lane and same-file atomicity rules still govern caller mutation,
while ratified 0.8.14 EXP-S D2/D5 established and preserved asynchronous vector
projection workers with serialized commits. There is never more than one
SQLite write transaction at a time, but projection publication no longer runs
on the primary writer connection.

### Current storage, retrieval and maintenance boundaries

Current format 39 (`S39-COMPOSITE`) builds on format 38's commit-timestamp index
and format 37's maintenance policy/queue/fence metadata and
[transaction-owned visibility coalescing](../adr/ADR-0.8.28-transactional-visibility-coalescing.md),
adding composite `(scope, ...)` keys/indexes, an 8-column `vec0` index
(`scope TEXT`), scoped provenance/dependency/actuation/erasure tables, and the
single-source `TABLE_MANIFEST` contract, while the `0.8.28+` SQLite registry
dependencies continue providing per-connection page caches. Canonical/projection
batches retain one checked generation/nonce advance; vector cleanup stays active,
and outer writer admissions validate clean state under the SQLite write lock.
Runtime opens refuse older formats until upgraded offline with `fathomdb migrate`
(from Schema 37 or 38; older formats remain refused without automatic migration). Runtime vec0 lifecycle/validity metadata enforces dense
eligibility before KNN limits; historical-view fallback and direct filtered body
FTS share the maintained [retrieval owner](retrieval.md).

Production provider admission selects built-in default or none. Numerical
mismatch/verification failure returns no Engine; persistent acceptance cache
limitations remain explicit in [embedder.md](embedder.md). The
[maintenance contract](explicit-maintenance-erasure/contract.md) owns exclusive
explicit windows and durable incomplete obligations; queued intent does not
revoke logical visibility. No SLA or new erasure campaign follows.

### Module model ownership and release

The accepted [0.8.30 ownership decision](../adr/ADR-0.8.30-explicit-module-model-release.md)
adds synchronous Rust SDK/Python `release_models` and asynchronous TypeScript
`releaseModels`. Standalone CLS and cross-encoder models are owned process
caches behind scoped leases; engine reranking leases the cross-encoder cache.
Engine-owned Mean providers retain a separate close lifecycle. One admission
gate covers loading, waits, inference and both native destructors. Overlap
refuses with the existing overload class, leaves both caches untouched and
preserves admitted work. Loading/inference/destruction occur outside cache and
gate locks and, for Python, outside the GIL. Healthy calls may reload; empty
release constructs nothing. Terminal CUDA generation loss and process pool
settings survive release. Selected completed-construction loss teardown is
qualified; reset during inference and same-process recovery are unsupported.

The optimized node-only hybrid route checks matched canonical eligibility
before cap/ranking instead of scanning every canonical node. Fusion still keys
actual stored node/edge versions and ownerless FTS rows, keeps original arm
positions and representative provenance, and never deduplicates by body/public
ID alone. Bounded `graph_expand` query seeding remains eligible body FTS ordered
by BM25/cursor with its own ranked seed cap and separate walk. It does not seed
from fused hits or perform entity resolution.

### Method-specific CUDA allocation evidence

The opt-in built-in default-embedder witness is current-only v2 with one tagged
measurement. Discrete non-private contexts use global-free model/control proof;
integrated private contexts use live USED growth and an exact 64 MiB same-pool
control with checked completion and return to baseline. Device/process/context
identity and a real 384-dimensional forward remain required. Common global
readings are diagnostic for the private method. Unsupported allocator/hardware
combinations refuse the witness instead of selecting a method from its result.
The [allocation-witness ADR](../adr/ADR-0.8.28-attributable-cuda-allocation-witness.md)
owns the precise union, selection, ordering and proof requirements.

### Governed read and evidence paths

Both explanation-enabled frozen search paths complete with a valid non-empty
Engine correlation identity. Explanation-disabled behavior and ranking remain
unchanged by that repair.

Ranked evidence remains an opt-in frozen search sidecar. Graph evidence is a
separate opt-in positional V1 sidecar on the existing graph operation. Its
opaque references bind the authenticated frozen context and normalized graph
disclosure to distinct immutable target and winning terminal-edge revisions.
The graph resolver returns intrinsic artifact/source evidence without
inventing rank, contribution, or full-path claims. Ordinary graph responses
remain unchanged when evidence is omitted or false.

### Governed mutation path

`Engine::actuate` remains the only actuation entry point and accepts one
changed-in-place six-operation V1 grammar: canonical-node write, derived-node
write, source-dependency registration, lifecycle transition, provenance-
bearing derived-edge write, and operational `PutLatestState` upsert. The writer validates the complete prospective
batch before effects, so an edge may name endpoints later in the batch but a
missing final endpoint refuses the whole operation.

Derived edges reuse canonical edge storage, revision identity, projection,
lifecycle, dependency, erasure, and integrity mechanisms. Current V1 request
digests and compact receipts bind the complete request and affected revisions.
There is no functional V2 pair, router, compatibility adapter, historical
receipt/replay interpretation, or partial graph commit.

### Operator boundary

`fathomdb doctor data-plane-integrity` is the version-matched, bounded,
machine-readable integrity route. It resolves the product database identity,
requires quiescence, opens SQLite immutably, and performs no recovery or
mutation. Doctor and recovery remain operator-only and absent from governed
Python, TypeScript, and Rust application SDKs.

### Preserved boundary and deferred destination

FathomDB owns durable identity, provenance, dependency, lifecycle, visibility,
retrieval, projection, evidence, mutation, and integrity mechanisms. The
application or semantic caller owns extraction, ontology, truth/contradiction,
retrieval planning, synthesis, answer verification, model/provider choice,
spend, and HITL policy.

The current profile remains single-source and reproduce-or-fail. Multi-source
causal provenance/liveness, persisted snapshot leases, graph continuation/full
paths, persisted evidence replay, and semantic policy remain deferred.

## Architecture destination and executable profile

The integrated profile above describes behavior present in source. Architecture v2 also
states longer-term constraints; a destination type or behavior below is not
an implemented API merely because it appears in the model. Deferred mechanisms
remain in [the design notes](0.8.x-after-0.8.25-design-notes.md) until their own
requirements and accepted implementation decisions authorize them. Earlier
as-built profiles remain in Git history and their release-local plans.

## Durable artifact classes

Architecture v2 distinguishes three classes that v1 treated too broadly:

1. **Canonical source records** preserve caller content, source identity,
   immutable record revisions, source versions, exact locators, and hashes.
2. **Caller-authored derived semantic records** preserve caller-decided facts,
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
fails with typed outcomes. On Schema `39+` (`ADR-0.8.32` `#103`), frozen-read
snapshot validation is scope-and-partition aware via
`_fathomdb_scope_visibility_watermarks` (`WITHOUT ROWID`),
`_fathomdb_read_visibility_state.(global_struct_gen, global_struct_nonce)`,
and `ReadSurface`-selective validation (`OperationalState`, `CanonicalPage`,
`Preflight`, `SearchNoGraph`, `EvidenceNode`, `GraphOrEdge`, `EvidenceGraph`,
`TraceDependency`) encoded in compact `0x53 0x39` wire tags (`492`–`556 B` vs.
`1,024 B` cap): unrelated additive mutations (disjoint canonical `kind`,
`operational_mutations` append-only logs, `operational_state` vs. canonical
surfaces, isolated tenant scopes including destructive mutations in other
scopes, and unrelated background `projection_worker` commits) preserve frozen
contexts without holding WAL transactions, while same-partition writes,
same-scope destructive/dependency mutations, global structural changes, and
raw uncoalesced SQL across all 20 trigger-bearing tables fail closed. FTS5
`bm25` raw scores reflect live corpus `(N, avgdl, df)` statistics rather than
frozen IDF snapshots (`ADR #103` §1.7). Direct logical lookup (`read.get` /
`read.get_many`) with `include_superseded = true` executes `SEARCH`-only via
`UNION ALL` over `canonical_nodes_scope_logical_{active,sup}_idx` with
Rust-side max-`write_cursor` tiebreaking and `has_source_dependencies`-gated
eligibility SQL (zero schema change).

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
In `0.8.32` (`ADR-0.8.32` `#103` Part 2, amending `ADR-0.8.25` `#48`),
`SearchFilter.property_predicates` is explicitly classified as an
artifact-scoped domain selector (cleared on evidence `source_filter` alongside
`kind` and `source_type` across `authorize_source_access`,
`query_graph_evidence_batch`, and `resolve_graph_evidence`), while
access-bearing `attributes`, `created_after`, `status`, `ReadView`
lifecycle/validity, dependency closures, and Schema 39 `ScopePredicate` remain
strictly source-enforced. By contrast, `trace_dependency` (`dependency_trace.rs`)
preserves `ADR-0.8.25 #49` symmetric endpoint visibility: only its SQL candidate
pre-queries omit `property_predicates`, while Rust `visible_artifact` in
`authorize_candidate_chain` and `include_candidate` continues applying full
`context.eligibility` (including `kind`, `source_type`, and
`property_predicates`) to both root and counterpart endpoints (intentionally
unchanged by `#269` Option A). On `GraphArm` hits, `SearchHit.source_id`
intentionally preserves the traversed edge's legacy `source_id`, whereas
`ResolvedEvidenceV1.source_id` resolves the returned node artifact's canonical
provenance source. When `search_with_evidence` runs with
`include_explanation = false`, lightweight `EvidenceContributionV1` capture is
decoupled from full `explain = true` so status-cache, structural explanation,
and `vec_distance_cosine` work is skipped.

Combined graph expansion reuses the existing graph store. Its 0.8.25 governed
request supports query or explicit seeds, incoming/outgoing/both directions,
canonical edge kind, target kind, indexed predicates, bounded depth/work/result
size, one read context, deterministic one-page targets, and compact graph
origin. Deterministic continuation and full path evidence are later profiles.
Application-specific domain vocabulary (such as typed entity slots) maps to
these generic constraints outside the Engine.

0.8.25 structural explanation covers included results, retrieval/reranking
contributions, fallback/degraded state, projection/graph origin,
dependency/lifecycle status, and privacy-safe operation correlation. In 0.8.32
(`Gate1-D10`, extending `0.8.8-explain-and-telemetry-adr.md` §A.4 Q1
specifically for `vector_cosine`), `PerHitExplain` additionally exposes raw
pre-RRF `vector_cosine: Option<f64>` (`1.0 - vec_distance_cosine` on uncentered
f32 vectors; `Some` iff scored by `execute_vector_arm` / `vector_rank.is_some()`,
`None` on zero-norm/`NULL`/`NaN` or vector-absent hits, finite clamped to
`[-1.0, 1.0]`) while keeping `SearchHit`, `EvidenceContributionV1`, and
`fathomdb-mcp` `PerHitOut` unchanged. Pooled reader explanation/evidence and
query-seeded graph expansion (`search(..., explain=true)`,
`search_frozen(..., explain=true)`, `search_with_evidence`, and query-seeded
`graph_expand`) share one unified snapshot boundary helper and reuse the
scope-partitioned projection-generation status cache with monotonic
cache-epoch guarding and per-connection `PRAGMA data_version` (`+ total_changes`
on `Engine.connection`) invalidation so steady-state `status_in_snapshot`
queries stay `O(1)` under default lifecycle views. Expanded
exclusion/not-selected explanation is a later profile.
Semantic entailment and answer correctness remain external.

## Deterministic retrieval improvement

FathomDB may expose deterministic entity/alias matching, duplicate suppression,
diversity, complementary-evidence selection, coverage selection, candidate
fusion, temporal retrieval, or associative graph diffusion only through named,
bounded profiles that pass preregistered data-plane quality and performance
gates. The caller decides when to invoke an accepted profile. A failed treatment is
retained as experimental evidence and does not change a default.

## Measurement boundary

Every result is classified as data-plane, semantic-control-plane, or end to end.
Receipts state whether `Engine.search` ran and identify shared and differing
components. Retrospective review of GLOBAL-01's initial storage-backed runner
found a bypass, but its dirty code receipt cannot prove that negative and the
executable classification therefore records `unknown_historical`. Its clean
held-out comparison proves search execution but also used caller planning and
answer generation. Neither is a retrieval-only comparison.

Release verification therefore adds a native retrieval-only global witness and
separately reports answer-system results. No answerer or semantic judge may be
hidden inside a FathomDB data-plane claim.

## Compatibility and safety

The following destination rules follow the accepted breaking-contract boundary:
affected actuation contracts change V1 in place, and noncurrent
databases are refused rather than migrated. The later graph-evidence decision
adds a first-generation V1 sidecar without creating a parallel graph API.

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
  and require no data migration. `latest_state` names the `operational_collections`
  collection kind and the `PutLatestState` / `put_latest_state` operations
  backed by `operational_state` — not a separate SQLite table, canonical
  lifecycle axis, or semantic-truth policy.
- Erasure, supersession, access, snapshot, and binding tests precede performance
  or readiness claims.

## Slice 4 architecture reconciliation

The completed 0.8.25 Slice 4 review adds seven named constraints. They are part
of architecture v2 rather than optional implementation guidance:

| Constraint | Architectural decision | Owning slices |
| --- | --- | --- |
| A25-01 | A frozen read is defined by reproducible canonical boundary, validity instant, projection generation, and eligibility envelope (refined on Schema `39+` by `ADR-0.8.32` `#103` `ReadSurface`-selective watermark validation so `OperationalState` and `CanonicalPage` skip unrelated partitions). 0.8.25 provides optional self-contained reproduce-or-fail context with typed unavailable, mismatch, and drift outcomes; persisted leases add expiry only in a later profile. It does not promise a permanently held SQLite reader transaction. | 35; 0.8.32 Slice 41 |
| A25-02 | Exact locators are UTF-8 byte ranges over a named immutable revision, using a declared canonical byte representation and hash algorithm; invalid boundaries and hash mismatch reject typed. | 15, 50 |
| A25-03 | FathomDB exposes governed current operational state backed by `operational_state` (with `latest_state` as the `operational_collections` kind and `PutLatestState` actuation operation, never a separate table or canonical lifecycle state). | 45; 0.8.32 Slice 40 |
| A25-04 | Multi-source dependencies use a bounded Engine-known liveness grammar. `all_required` and `any_surviving` have distinct removal behavior; unsupported rules reject typed. | Deferred; future allocation requires approval. |
| A25-05 | Every new persisted or public type defines schema/wire version and unknown-field/unknown-variant behavior, with feature-local Rust, Python, TypeScript, and applicable Windows CPU/native proof. | active 15–60; audit 75; future slices retain the rule |
| A25-06 | Eligibility and graph constraints execute before seed/candidate truncation and expansion. 0.8.25 reports compact inclusion/degradation state; expanded deterministic ineligible/not-selected explanation follows only when justified. | 35/55/60 core; expanded explanation 0.8.28 |
| A25-07 | Default `SearchHit` remains compact. Evidence handles are opt-in, bound to the originating visibility envelope, and resolve stale or invisible state only to typed non-disclosure. | 50 |

Architecture v1 remains historical. Accepted/locked predecessor designs remain
evidence and are not rewritten; each 0.8.25 feature design records whether it
reuses, amends, or supersedes them.

## Release allocation

Run `python3 scripts/release-current.py` to find the current release state
and its allocation. The [engineering index](../README.md#where-the-live-work-is)
links the plan and board conventions. Accepted ADRs and maintained interface
contracts define semantics. The [.31 release notes](../../docs/release-notes/0.8.31.md)
describe the current published registry and Tegra Pages delivery. The
[.30](../../docs/release-notes/0.8.30.md) and [.29 notes](../../docs/release-notes/0.8.29.md) retain their historical scope. Historical
allocations and activation evidence remain in their release plans and Git
history.
