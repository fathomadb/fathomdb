---
title: 0.8.25 Slice 40 — core projection generation and readiness design
status: DRAFT_FIX_1
design_version: 4
review_cycle: 1
target_release: 0.8.25
depends_on: 35
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 40 design

## Authority, disposition, and limits

This design implements S40-R1 through S40-R6, the retained core of
R25/AC25-40, Memex need 13, N25-01/N25-04, and A25-05. It succeeds Slices
15–35 without changing their identity, provenance, dependency, actuation,
lifecycle, eligibility, or frozen-token authorities.

The accepted projection registry, projection runtime, freshness, dense
readiness, and in-place rebuild designs remain historical authority for their
shipped behavior. This document is their 0.8.25 successor only for durable
serving-epoch identity and boundary-qualified readiness. It does not rewrite
those records or claim that an in-place store is a side-by-side generation.
The public interface documents and decision index receive additive successor
pointers when implementation lands.

Slice 40 deliberately does **not** add parallel physical indexes, reader
routing among physical generations, a new durable scheduler, public work-item
administration, exactly-once embedding computation, application-owned cleanup,
or retained historical projection reads. Those are richer post-0.8.25 work.

## Chosen generation model

### One physical serving-set epoch

A `ProjectionGenerationId` identifies one database-local epoch of the complete
physical serving projection set:

- node body FTS (`search_index` and `search_index_v2`);
- edge FTS (`search_index_edges`);
- declared exact attributes (`canonical_attributes`);
- declared attribute FTS (`property_search_index`);
- the shared dense sidecar and vec0 partition
  (`_fathomdb_vector_rows` and `vector_default`); and
- the existing cursor/terminal state that declares publication progress.

It is **not** per registry declaration. Declarations share physical stores and
the dense pipeline has one corpus-wide readiness authority, so per-declaration
generation IDs would be false precision. Every current search arm reads the one
current physical serving set and therefore the same serving generation.
Generations are plural over time, not concurrently readable physical copies.

Ordinary canonical writes attach work to the current generation; they do not
mint a new ID. A non-no-op projection configuration change or an operator
in-place rebuild retires the prior metadata epoch and mints a new serving epoch
inside the transition transaction. Because the physical tables are updated in
place, the new epoch is serving immediately but may report `processing` or
`degraded`. No `building` role or atomic parallel-store swap is exposed.

CPU, CUDA, runtime presence, worker count, and device ordinal are session facts
and never enter generation identity. Persistent embedder/profile identity does.

### Boundary meanings

The generation row has one immutable `transition_boundary`: the global
canonical write high-water point observed when the epoch is minted. It is not a
claim that later writes are absent or ready.

Every status read reports:

- `observed_boundary`: the global canonical write high-water point in the same
  reader transaction;
- `ready_through`: the existing contiguous projection cursor in that
  transaction; and
- readiness qualified by those exact two values.

`ready` means all projection-applicable work through `observed_boundary` has an
`up_to_date` terminal and no assigned failed terminal. It is never an
unqualified permanent property. A later write may make the same generation
`processing` at a larger observed boundary.

Dependency generation, lifecycle operation IDs, actuation operation IDs, and
closure sequences are not projection mutation boundaries. The only public
mutation correlation key is the canonical `write_cursor`. Slice 25 already
returns each cursor with outstanding projection work in
`pending_projection_write_cursors`; Slice 40 does not reinterpret that list.

## Public and wire contract

### Types and methods

```text
ProjectionGenerationId = "pgen1:" + 32 lowercase hex characters

ProjectionGenerationReadiness =
  ready | processing | blocked | deferred | degraded

ProjectionRuntimeState = usable | no_runtime | equivalence_refused

ProjectionGenerationStatusV1 {
  schema_version: 1,
  generation_id: ProjectionGenerationId,
  declaration_sha256: 64 lowercase hex,
  transition_boundary: u64,
  observed_boundary: u64,
  ready_through: u64,
  readiness: ProjectionGenerationReadiness,
  pending_count: u64,
  failed_count: u64,
  runtime_state: ProjectionRuntimeState
}

MutationProjectionStatusRequestV1 {
  schema_version: 1,
  write_cursor: u64,
  expected_generation_id: ProjectionGenerationId
}

MutationProjectionState =
  ready | processing | blocked | deferred | degraded |
  generation_unavailable

MutationProjectionStatusV1 {
  schema_version: 1,
  write_cursor: u64,
  generation_id: ProjectionGenerationId,
  work_mask_version: 1,
  work_mask: u32,
  state: MutationProjectionState,
  reason_code?: string
}

Engine::read_projection_generation_status()
  -> Result<ProjectionGenerationStatusV1, EngineError>

Engine::read_mutation_projection_status(
  request: MutationProjectionStatusRequestV1
) -> Result<MutationProjectionStatusV1, EngineError>
```

Both reads are pure and bounded. The generation read returns exactly one
record. The mutation read performs indexed point lookups and never scans an
unbounded work list. It returns no source bytes, IDs, query, owner/scope value,
or model prompt.

When Slice 25 creates at least one pending projection cursor, its additive
response field `projection_generation_id` names the generation stored on those
assignments. It is absent when the pending list is empty and on pre-Slice-40
historical receipts. Mutation-status reads require that field; historical
receipts without it do not receive a guessed current-generation correlation.
No existing receipt field changes meaning.

Rust uses the types and spellings above. Python exports frozen dataclasses and
`read.projection_generation_status` /
`read.mutation_projection_status`; integer fields are Python `int`. TypeScript
exports readonly interfaces and `engine.readProjectionGenerationStatus()` /
`engine.readMutationProjectionStatus(request)`. JSON, TypeScript, and canonical
fixtures encode every `u64` as a canonical unsigned decimal string; leading
zeroes, signs, whitespace, exponent notation, booleans, unsafe JS numbers, and
values above `u64::MAX` reject.

Requests reject unknown fields and versions; response readers ignore additive
fields but reject an unknown schema version or semantic enum value. Field paths
are RFC 6901 camel-case wire paths. Canonical fixtures pin Rust, Python, and
TypeScript bytes, large boundaries, every enum, and every error.

### Errors and precedence

`EngineError::ProjectionGeneration(ProjectionGenerationError)` maps to
`FDB_PROJECTION_GENERATION`, Python `ProjectionGenerationError`, and TypeScript
`ProjectionGenerationError`. The error carries only `reason` and `field_path`.
Closed reasons are:

- `unsupported_schema_version`;
- `unknown_field`;
- `invalid_write_cursor`;
- `invalid_generation_id`;
- `generation_history_exhausted`;
- `mutation_not_tracked`;
- `wrong_projection_generation`;
- `projection_generation_unavailable`; and
- `projection_generation_corrupt`.

Dynamic bindings validate in this order before calling the Engine: schema
version, lexicographically first unknown field, write-cursor type/canonical
range, then optional generation ID. Engine validation then checks open/closing,
persisted current-generation authority, assignment existence, expected versus
assigned generation, generation availability, and terminal state. Malformed
input therefore cannot be converted by FFI before the specified typed error.
Storage corruption never falls through to `mutation_not_tracked` or a ready
record.

## Persistent state and canonical encodings

Schema step 32 is additive shape/state only. It moves no canonical or
projection content and contains no `INSERT…SELECT` migration.

```sql
CREATE TABLE _fathomdb_projection_generations(
  schema_version INTEGER NOT NULL CHECK(schema_version = 1),
  generation_id TEXT PRIMARY KEY,
  declaration_sha256 TEXT NOT NULL,
  transition_boundary INTEGER NOT NULL CHECK(transition_boundary >= 0),
  role TEXT NOT NULL CHECK(role IN ('serving', 'retired')),
  origin TEXT NOT NULL CHECK(origin IN (
    'fresh', 'legacy_unverified', 'configuration', 'rebuild'
  )),
  retired_boundary INTEGER,
  CHECK(
    (role = 'serving' AND retired_boundary IS NULL) OR
    (role = 'retired' AND retired_boundary IS NOT NULL)
  )
);

CREATE UNIQUE INDEX _fathomdb_projection_one_serving
  ON _fathomdb_projection_generations((1)) WHERE role = 'serving';

CREATE TABLE _fathomdb_projection_generation_current(
  singleton INTEGER PRIMARY KEY CHECK(singleton = 1),
  generation_id TEXT NOT NULL UNIQUE
    REFERENCES _fathomdb_projection_generations(generation_id)
);

CREATE TABLE _fathomdb_projection_assignments(
  schema_version INTEGER NOT NULL CHECK(schema_version = 1),
  write_cursor INTEGER PRIMARY KEY CHECK(write_cursor > 0),
  generation_id TEXT NOT NULL
    REFERENCES _fathomdb_projection_generations(generation_id),
  work_mask_version INTEGER NOT NULL CHECK(work_mask_version = 1),
  work_mask INTEGER NOT NULL CHECK(work_mask BETWEEN 0 AND 15)
);

CREATE INDEX _fathomdb_projection_assignments_generation_cursor
  ON _fathomdb_projection_assignments(generation_id, write_cursor);
```

The current singleton and partial unique index must agree. Open and both public
reads validate exactly one serving generation, exact singleton equality,
generation-ID grammar, digest grammar, legal role/retirement combinations,
assignment foreign keys, and assignment/terminal compatibility. Any mismatch
is typed corruption; no repair occurs on a normal read.

Generation IDs encode 16 random bytes from the Engine's existing OS-random
source. Minting inserts under the primary-key constraint and retries collision
at most four times; four collisions return storage failure without changing the
old generation. No counter can wrap and no ID is derived from configuration.
Backup/file copy preserves the database and generation identity. A separately
created database receives unrelated IDs. Retired IDs are never deleted or
reused in 0.8.25. The table is capped at 1,024 total generations; a transition
that would exceed the cap refuses atomically with `generation_history_exhausted`
before touching physical stores. This bounds content-free history by explicit
configuration/rebuild transitions rather than record count and avoids
application-owned garbage collection.

`declaration_sha256` is SHA-256 over the ASCII domain
`fathomdb.projection-serving-declaration.v1\0` followed by:

1. schema version 32 and fixed physical-arm contract tokens in the physical
   inventory order above;
2. every projection registry row in name-byte order with exact `name`, sorted
   role set, `fts_tokenizer`, `vector_embedder`, `vector_declared`, and
   `source`, using the Slice-35 scalar/optional codec; and
3. the persistent default embedder profile identity/equivalence digest, or an
   absent tag.

SQL NULL is distinct from empty text. Device, worker count, session runtime,
and transient availability are excluded. A changed digest through
`configure_projections` mints a generation; exact idempotent replay does not.
An operator rebuild always mints even when the digest is unchanged.

### Assignment work mask

Version 1 assigns four physical work classes:

| Bit | Class | Applicability | Completion authority |
|---:|---|---|---|
| 0 | node body FTS | canonical node with body | same canonical transaction |
| 1 | edge FTS | canonical edge with body | same canonical transaction |
| 2 | declared attribute EAV/FTS | canonical node and at least one effective declaration | same canonical/configuration transaction |
| 3 | shared dense | vector-eligible canonical node/edge under an effective dense declaration | `_fathomdb_projection_terminal` plus vector sidecar/vec0 publication |

One assignment row is written for each canonical write cursor that enters the
existing projection cursor protocol, including a zero mask when the protocol
requires an explicit skip. There is never one row per declaration or per work
class. Storage is O(projected canonical mutations), not
O(mutations × declarations × work kinds × generations). Existing no-projection
boundary markers remain the global gap authority and are not reclassified as
canonical mutation assignments.

The assignment is inserted in the canonical transaction before its terminal
decision. Synchronous work and deterministic non-applicability reach the
existing `up_to_date` terminal in that transaction. Dense-applicable work has
no terminal until publication; successful vector row, sidecar, terminal, and
generation-consistent assignment revalidation occur in the worker's one
transaction. Exhausted embedding failure records `failed`, never ready.

Assignment rows are deleted with the owning row's projection terminal during
physical erasure. A later status request returns
`projection_generation_unavailable` or `mutation_not_tracked` without source
information. Retired generation metadata contains no source-bearing payload.

## Readiness derivation and compatibility

Within one reader transaction, the Engine reads the serving generation,
`observed_boundary`, `ready_through`, assigned missing terminals, assigned
failed terminals, and session runtime. Readiness is derived in this exact order:

1. invalid authority or assignment state → typed corruption, no status;
2. `legacy_unverified` origin → `degraded`;
3. any failed assignment through the observed boundary → `degraded`;
4. no pending assignment and `ready_through >= observed_boundary` → `ready`;
5. pending and no attached runtime → `blocked`;
6. pending and attached runtime refused by equivalence/initialization →
   `deferred`;
7. pending with usable runtime → `processing`;
8. a cursor gap without a pending assignment → `degraded`.

`pending_count` and `failed_count` are exact indexed counts for the current
generation through the observed boundary. They are capped at `u64::MAX` by
checked conversion; overflow is corruption rather than saturation.

Mutation state applies the same precedence to one assignment. An assignment to
a retired generation returns `generation_unavailable`; it is never rebound to
the new generation. The required `expected_generation_id` mismatch returns
`wrong_projection_generation` before interpreting terminal state.

Existing status surfaces remain source-compatible:

- no effective dense declaration remains `not_declared`;
- new `ready` maps to existing dense `ready`;
- new `processing` maps to existing `embedding`;
- new `blocked`/`deferred` map to existing `unavailable` when runtime is absent
  or refused; and
- new `degraded` maps conservatively to existing `embedding`, never `ready`.

`read_embedding_readiness` remains explicitly scheduler/runtime readiness, not
generation completeness; its documentation gains that distinction. It may say
the scheduler has no outstanding work after a terminal failure, while the new
generation status truthfully remains degraded. Ordinary callers are not forced
onto the new API.

## Write, configuration, rebuild, and restart flows

### Ordinary write and publication

The current generation is read and validated inside the same `BEGIN IMMEDIATE`
transaction that owns the write. The assignment, canonical row, synchronous
projections, terminal decision, and additive Slice-25 receipt generation ID are
atomic. Post-commit in-memory cursor publication and worker notification retain
their current ordering.

The worker acquires its existing immediate transaction and Slice-30 physical
closure guard. Before writing vec0 it rechecks that the assignment still names
the current generation and the owner remains eligible. Mismatch deletes no
current projection and publishes no terminal. Vector/sidecar/terminal changes
remain atomic and idempotent. A crash may repeat external embedding computation,
but durable publication is at-most-once by terminal/assignment checks; exactly-
once provider execution is not promised.

### Configuration transition

`configure_projections` computes the prospective declaration digest before
mutation. Exact digest replay uses the existing generation. A real change,
including an explicit drop, performs current-generation retirement, new ID
mint, new current singleton, assignment reconstruction for affected canonical
rows, synchronous backfill/cleanup, and dense re-enrolment in its existing
configuration transaction. The new epoch is serving at commit. It is ready only
if the derivation above proves it at the post-commit observed boundary.

### In-place rebuild

The operator rebuild remains in-place and preserves its freeze/drain and
Slice-30 guards. It does not create a hidden second physical store:

1. freeze new worker dispatch and complete the current drain requirement;
2. acquire the rebuild `BEGIN IMMEDIATE` transaction;
3. retire the old metadata epoch and mint/install a new serving generation;
4. truncate/rebuild selected physical stores through the existing shared
   projection primitives;
5. reconstruct assignments and synchronous terminals for the new generation;
6. enqueue dense-applicable rows by terminal absence; and
7. commit, publish runtime state, unfreeze, and notify once.

Frozen readers begun before the transaction linearize before it. New readers
after commit see the new generation and possibly processing/degraded readiness.
There is no atomic old/new physical cutover claim. A crash before commit leaves
the old epoch and stores; a crash after commit reopens the new epoch and the
existing terminal-absence scheduler resumes publication.

### Migration/bootstrap

Migration step 32 creates only empty tables, constraints, indexes, and Slice-35
visibility triggers. Engine bootstrap then uses one immediate transaction:

- an empty database receives a fresh ready generation at boundary zero;
- a non-empty pre-step-32 database receives one `legacy_unverified` serving
  generation and reports `degraded` until an explicit configuration rebuild or
  operator rebuild establishes assignments under the new authority; and
- no legacy projection row is copied, rewritten, or falsely certified by the
  migration.

This conservative rule avoids an unbounded open-time content audit and cannot
create false readiness. Bootstrap is idempotent across restart. If any Slice-32
row exists without a complete valid singleton/serving pair, open fails closed
instead of minting over corruption.

## Slice-35 frozen reads

The v1 token prefix, payload fields, byte encoding, HMAC domain, maximum size,
and golden fixture remain unchanged. Slice 40 extends the serving digest input
with the current generation ID, declaration digest, transition boundary, and
role under the existing projection-serving domain. It does not add a token
field.

Generation, current-singleton, and assignment tables join the Slice-35
visibility-trigger manifest in migration step 32. Their insert/update/delete
increments the existing checked read-visibility generation. Virtual-table
mutations remain coupled to canonical, assignment, state, or terminal real rows
in the same transaction; the closed source audit gains every new write site.

All pre-step-32 frozen tokens drift after migration because the serving digest
input changed. They are not mapped or rebound. A token minted after step 32
validates the complete current generation state; any generation transition,
assignment publication, readiness change, erasure, or lifecycle-visible change
committed before consume yields the existing non-disclosing
`state_drifted`/`state_unavailable`. This is the one-to-one rule: a successfully
validated token corresponds to exactly the singleton current generation in its
snapshot; zero or multiple current generations is storage corruption.

## Lifecycle, erasure, and integrity

Slice-30 closure remains the publication fence. Projection publication always
revalidates owner eligibility and current generation after acquiring the write
transaction. Lifecycle or erasure that wins first prevents later publication;
publication that wins first is removed by the following closure transaction.

Only the current epoch has physical serving bytes. Retired generations are
content-free metadata, not retained indexes. Erasure removes canonical bytes,
all current physical projections, terminal and assignment rows, and waits for
the existing closure proof before completing. It does not need to scrub a
fictional retired physical store. Secure-delete/WAL proof remains the Slice-30
authority. Raw-state tests cover every new table and every generation/runtime
state.

Normal reads never repair generation state. The existing operator rebuild is
the only 0.8.25 path that clears `legacy_unverified` or reconstructs corrupt
projection content. Full integrity-job orchestration and sophisticated repair
planning remain experimental/Parked.

## Performance and storage boundary

Before measurement, commit a source-bound Slice-40 plan/config and preserve raw
outputs under the external performance data tree. Commit only portable receipts
under `experiments/runs/scale-02-slice40-*`. Compare the Slice-35 parent and
Slice-40 candidate on identical 10k inputs, query order, fresh databases, and
five repetitions.

The feature-local acceptance boundaries are:

- ordinary synchronous write p50 and p95 95-percent upper relative regression
  at most 5%;
- incremental checkpointed database size at most 192 bytes per assignment at
  10k, with WAL bytes reported separately;
- current-generation and mutation-status reads p95 at most 5 ms and p99 at most
  10 ms at 50k, with query plans using the singleton/primary indexes;
- restart/open p95 upper regression at most 10% or 25 ms absolute, whichever is
  larger;
- generation metadata transition overhead at most 10 ms, excluding the
  separately reported existing projection rebuild/backfill work; and
- CPU versus CUDA generation-status p95 difference at most 2 ms on the same
  database. Embedding throughput is not attributed to the status API.

Failures, timeouts, cold/steady state, page/WAL/storage bytes, runtime/device,
and exact operation witness counts are reported. A miss blocks Slice 40
closeout or requires a new preregistered design treatment; it is not deferred to
Slice 75 and the threshold is not rewritten after observation.

## TDD and verification matrix

RED is committed before GREEN and uses real SQLite, never a database mock.

| Target | Required proof |
|---|---|
| `step32_projection_generation` | additive shape; exact checks/indexes/triggers; fresh/upgrade bootstrap; no content migration; invalid/multiple current generations reject. |
| `slice40_projection_generation` | ID grammar/random collision retry; restart/copy/no reuse; digest fixture; no-op config stability; config/rebuild mint; boundary-qualified state; legacy degraded state. |
| `slice40_mutation_projection_status` | Slice-25 receipt flow; work masks; ready/pending/failed; expected-generation mismatch; retired/untracked/erased; exact indexed plans; property round trips. |
| `slice40_projection_generation_races` | write/publication, lifecycle, erasure, configure, rebuild, restart at each transition, duplicate publication, and old/new reader linearization. |
| `slice35_frozen_read` additions | codec bytes unchanged; pre-step-32 token drift; generation/assignment/readiness drift; exact singleton current generation. |
| binding parity | canonical Rust/Python/TypeScript records, enum/error mapping, unknown/version precedence, `u64::MAX`, injection-shaped IDs, fresh artifacts. |
| CUDA/Windows/package | no runtime, equivalence refusal, CPU/CUDA reopen/recovery, installed wheel/npm, Windows runtime or explicit unavailability. |
| performance receipt | exact candidate/baseline artifacts, operations, call counts, latency/storage/reopen bounds, no hidden model or answerer. |

Focused and enclosing commands are owned by the Slice-40 plan. The final gate
includes `./scripts/agent-verify.sh --tier=fast`, applicable heavy and operator
routes, fresh wheel and offline npm/native package smokes, RTX-3090 CUDA tests,
and Windows execution when available. Ptrace-dependent tests run unchanged
outside the sandbox. CUDA and Metal remain mutually exclusive feature routes.

## Forward obligations

- Slice 45 binds pagination cursors to the current serving generation and
  refuses a retired/mismatched epoch.
- Slice 50 evidence references bind to the same generation and recheck current
  eligibility/lifecycle state.
- Slice 55 diagnoses generation/assignment/terminal integrity without making
  semantic judgments.
- Slice 60 returns graph-path generation origin under the same frozen context.
- Slice 75 audits installed parity and investigates the separate vector-latency
  and bulk-ingest signals without changing this slice's accepted thresholds.
- Post-0.8.25 work may introduce side-by-side physical stores, retained
  generations, or richer work manifests only through a successor design.
