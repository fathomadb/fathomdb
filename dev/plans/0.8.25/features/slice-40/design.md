---
title: 0.8.25 Slice 40 — core projection generation and readiness design
status: DRAFT_FIX_3
design_version: 6
review_cycle: 3
target_release: 0.8.25
depends_on: 35
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 40 design

## Authority, outcome, and limits

This design implements S40-R1 through S40-R6, the retained core of
R25/AC25-40, Memex need 13, N25-01/N25-04, and A25-05. It succeeds Slices
15–35 without changing their identity, provenance, dependency, actuation,
lifecycle, eligibility, or frozen-token authorities.

The existing projection registry, in-place stores, scheduler, terminal rows,
and rebuild path remain authoritative for their shipped behavior. This design
adds a durable identity for the one physical serving set and a truthful view of
whether its physical dense members are complete. It does not turn the
existing terminal watermark into a new authority.

Slice 40 does **not** add parallel physical generations, retained historical
indexes, a new scheduler, a public work manifest, exactly-once model execution,
application-owned cleanup, profile routing, or semantic policy. Those remain
outside 0.8.25.

## Invariants

1. Exactly one `ProjectionGenerationId` identifies the database's current
   in-place serving projection set.
2. Generation identity changes only on a non-noop projection configuration
   transition or an operator projection rebuild. Ordinary writes, worker
   publication, lifecycle changes, erasure, and deterministic boot repair
   change readiness or visibility within the current generation.
3. A dense physical member is complete only when an `up_to_date`
   terminal, a matching `_fathomdb_vector_rows` sidecar, and a vec0 row all
   exist in the same snapshot. An `up_to_date` terminal alone is never proof.
4. Generation readiness ranges over physical projection members, not
   every integer write cursor. Erased rows, operational cursors, reserved
   migration cursors, rolled-back reservations, and inactive owners cannot
   create permanent readiness holes.
5. A Slice-25 receipt is correlated to the generation current at its commit.
   It is never rebound to a later generation and its stored canonical fields
   rehydrate identically on replay.
6. A status read is pure, uses one reader transaction, and either reports the
   current generation truth or returns a typed failure. It never repairs state.
7. Frozen-read visibility changes in the same transaction as every mutation of
   the new serving-generation authority.

## One global in-place serving epoch

`ProjectionGenerationId` identifies the complete physical serving set:

- node FTS (`search_index` and `search_index_v2`);
- edge FTS (`search_index_edges`);
- exact attributes and property FTS;
- `_fathomdb_vector_rows` and `vector_default`; and
- the registry, embedder profile, enrolment, state, and terminal authorities
  that determine those stores.

The ID is database-local metadata. It is not a physical table suffix, routing
handle, source identity, dependency generation, write cursor, lifecycle
operation ID, or proof that every async projection is complete. There is one
physical serving set before and after a transition; a rebuild remains in place.

`ProjectionGenerationId` has grammar `pgen1:` followed by exactly 32 lowercase
hex digits. The Engine mints 16 OS-random bytes and retries a primary-key
collision at most four times. Four collisions fail atomically as storage
failure. Retired IDs are retained and never reused. There is no arbitrary
history cap; storage is O(explicit configuration/rebuild transitions), not
O(records).

## Persistent state

Schema step 32 is additive shape/state only and performs no canonical or
projection content migration.

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
  retired_boundary INTEGER CHECK(
    retired_boundary IS NULL OR
    (retired_boundary >= 0 AND retired_boundary >= transition_boundary)
  ),
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

ALTER TABLE _fathomdb_actuation_receipts
  ADD COLUMN projection_generation_id TEXT;

CREATE TRIGGER _fathomdb_projection_generation_immutable
BEFORE UPDATE ON _fathomdb_projection_generations
WHEN OLD.role = 'retired'
  OR NEW.generation_id != OLD.generation_id
  OR NEW.schema_version != OLD.schema_version
  OR NEW.declaration_sha256 != OLD.declaration_sha256
  OR NEW.transition_boundary != OLD.transition_boundary
  OR NEW.origin != OLD.origin
  OR OLD.role != 'serving'
  OR NEW.role != 'retired'
  OR NEW.retired_boundary IS NULL
BEGIN SELECT RAISE(ABORT, 'projection generation is immutable'); END;

CREATE TRIGGER _fathomdb_projection_generation_retain
BEFORE DELETE ON _fathomdb_projection_generations
BEGIN SELECT RAISE(ABORT, 'projection generation history is retained'); END;
```

No per-record generation assignment table is added. It would duplicate the
physical-owner and receipt authorities, require O(N) rewrites on a global epoch
transition, and create erasure-sensitive cursor gaps. The nullable receipt
column is the compact mutation-to-generation correlation selected by this
slice.

For receipts created at schema step 32 or later, the receipt column is non-NULL
exactly when a committed receipt's `pending_projection_write_cursors_json` is
non-empty. It is NULL for refused, erased, and empty-pending receipts. Accepted
pre-step-32 receipts may have a non-empty pending list and NULL generation; that
is the only legacy exception and requires every pending cursor to be at or below
the `legacy_unverified` bootstrap transition boundary. A non-NULL value must be a
valid generation that was serving at receipt commit. Receipt persistence,
hydration, replay validation, canonical serialization, and redaction include
the field. Any other combination is storage corruption; it is never repaired
or guessed from the current generation.

Open and both public reads validate one serving row, one matching singleton,
ID/digest grammar, legal role/retirement pairs, immutable history, and receipt
shape when a receipt is addressed. A serving row may transition exactly once
to retired with its retirement boundary; all other row updates and deletion are
rejected by SQLite triggers. Two transitions may share a boundary when no
canonical write occurred between them; their random IDs distinguish them and
this slice does not claim a total historical order. Missing, duplicate, or
contradictory authority is typed `projection_generation_corrupt`.

### Bootstrap and migration truth

`check_embedder_profile` runs before generation bootstrap on every open. Even
`EmbedderChoice::None` supplies and persists the pinned default identity, so
the declaration digest never has an absent-profile state. A changed supplied
identity continues to fail through the accepted embedder mismatch before
generation logic runs.

After step 32 and embedder-profile establishment, Engine bootstrap executes in
one immediate transaction:

- an empty database receives a `fresh` generation at boundary zero;
- a non-empty upgraded database receives `legacy_unverified` at the persisted
  `load_next_cursor` boundary and therefore reports `degraded`; and
- an existing valid generation is reused unchanged.

Bootstrap is idempotent. A partial authority row set fails closed. The Engine
recomputes and compares the serving declaration digest during open and before
each public status read. Open mismatch is
`EngineOpenError::Corruption { kind: ProjectionGenerationDrift, stage:
ProjectionGeneration, recovery_hint: E_CORRUPT_PROJECTION_GENERATION }`;
read-time mismatch is `projection_generation_corrupt`. A legacy generation
becomes certifiable only through an explicit configuration change or operator
rebuild; an idempotent configuration replay does not launder it.

`transition_boundary` and `retired_boundary` use the exact persisted high-water
computed by `load_next_cursor`: the maximum of canonical nodes, canonical
edges, operational mutations, operational state, the reserved migration
cursor, and dependency-closure admitted boundaries. The in-memory atomic is
not authoritative.

## Declaration identity

`declaration_sha256` is SHA-256 over the following exact bytes:

1. ASCII domain `fathomdb.projection-serving-declaration.v1\0`;
2. fixed contract token `projection-serving-set/1`, encoded with the Slice-35
   length-prefixed UTF-8 scalar codec;
3. the fixed arm names `node_fts_v2`, `edge_fts_v1`, `attributes_v1`,
   `property_fts_v1`, and `dense_default_v1`, in that order;
4. registry row count, then rows ordered by raw UTF-8 `name` bytes, each
   encoding `name`, sorted role strings, nullable `fts_tokenizer`, nullable
   `vector_embedder`, boolean `vector_declared`, and nullable `source` with the
   Slice-35 scalar/option/list tags; and
5. the persistent default embedder profile's `profile`, `name`, `revision`, and
   `dimension`, in that order.

The live schema version is excluded so unrelated migrations cannot mint an
epoch. SQL NULL differs from empty text. Learned `mean_vec`, device, runtime,
worker count, availability, equivalence-probe cache/results, and session state
are excluded. Vector identity remains embedder-owned.

A changed digest accepted by `configure_projections` mints a generation. An
exact idempotent replay does not. `rebuild_projections` and `rebuild_vec0`
always mint even when the digest is unchanged, because the physical serving
set is reconstructed.

## Exact physical membership and completion

Readiness describes the physical projection set, not the strict default read
view. Slice 40 introduces one internal `projection_membership_v1` relation and
one `projection_completion_v1` classifier used by generation status, mutation
status, scheduler scans, pending probes, publication revalidation, and integrity
tests. Existing projection writers retain publication ownership but delegate
membership decisions to these helpers.

Membership is independent of wall-clock visibility for nodes. It covers
historical, pending, superseded, and currently out-of-window node rows that the
shipped physical projector retains for legal relaxed `ReadView` calls. Time
passing alone therefore cannot alter generation readiness.

### Synchronous arms

Node/edge FTS and declared attributes are synchronous. Their completion is the
canonical/configuration/rebuild transaction itself:

- all node row kinds receive node FTS;
- body-bearing edges receive edge FTS;
- active, non-superseded nodes receive declared attribute EAV/property FTS;
- pending/inactive nodes do not receive attribute rows until activation; and
- lifecycle removal, supersession, configuration cleanup, and erasure update
  the synchronous stores atomically with their owners.

They do not create pending work. Missing or orphaned synchronous rows are an
integrity defect diagnosed by Slice 55; Slice 40 never labels a partially
committed transaction ready because SQLite cannot expose it.

### Dense node membership

A non-erased canonical node is a dense member exactly when all are true:

1. Slice-30 dependency closure has no active physical barrier for its source;
2. `row_kind` is `leaf` or `coverage`;
3. its `kind` is accepted by `kind_is_vector_committable`; and
4. `vector_projection_declared` is true.

Node lifecycle state, supersession, `valid_from`, `valid_until`, and status-read
wall clock do not change membership. Search eligibility still filters those
rows before ranking under Slice 35. This matches the node projector's retained
physical corpus and supports strict and relaxed read views.

Runtime availability is not identity or applicability. Kind enrolment is
derived scheduler state, not policy; a declared, committable row remains
applicable while no runtime exists or before late enrolment. That state is
`blocked` or `deferred`, never ready.

### Dense edge membership

A non-erased canonical edge is a dense member exactly when all are true:

1. it has a body;
2. it is not superseded and its `t_invalid` does not exclude it under the
   persisted edge-validity rule;
3. Slice-30 dependency closure has no active barrier for its source; and
4. fixed kind `edge_fact` is committable.

This preserves the shipped edge scheduler/rebuild rule: body-bearing edges
auto-enrol `edge_fact` independently of the projection registry, while invalid
or superseded edge projections are physically closed rather than retained for
dense historical reads. Relaxed edge queries use the existing non-dense
fallback when the dense arm cannot represent that view. Runtime absence leaves
the member blocked.

Each status transaction resolves one `effective_at_epoch_s` and echoes it. Edge
`t_invalid` is evaluated against that value in every scheduler/status arm. A
clock crossing can only remove an edge from physical membership; it never adds
new work, so it needs no wake-up path and cannot manufacture false readiness.

### Dense state classification

For either owner, completion requires all three:

1. `_fathomdb_projection_terminal.state = 'up_to_date'`;
2. `_fathomdb_vector_rows` contains the same cursor and expected kind; and
3. `vector_default` contains the same rowid and expected source type/kind.

The three are published in the worker transaction. The normative classifier is:

| Member state in one snapshot | Classification | Progress path |
|---|---|---|
| `up_to_date` + matching sidecar + matching vec0 | complete | None required. |
| no terminal + no sidecar + no vec0 + enrolled | scheduler-pending | Usable runtime dispatches it; absent runtime is blocked; refused runtime is deferred. |
| node only: `up_to_date` + no sidecar + no vec0 + not enrolled | legitimate-stranded | A usable open/configuration graft enrolls the kind, deletes the terminal, rewinds the old scheduler cursor, and notifies. Absent/refused runtime is blocked/deferred. |
| `failed` + no sidecar + no vec0 | failed | Generation is degraded; only governed rebuild retries it. |
| `up_to_date` with exactly one physical row | corrupt | No automatic repair; typed corruption. |
| no terminal with either physical row | corrupt | No automatic repair; typed corruption. |
| `failed` with either physical row | corrupt | No automatic repair; typed corruption. |
| matching terminal/sidecar with wrong kind, source type, or row identity | corrupt | No automatic repair; typed corruption. |
| edge member not enrolled, or usable-runtime node still legitimate-stranded after boot graft | corrupt | Required enrolment/graft did not complete before publication. |

`processing` is emitted only for scheduler-pending work with a usable runtime.
`blocked` and `deferred` apply to scheduler-pending or legitimate-stranded work
with the corresponding runtime state. Every non-degraded incomplete response
therefore has a deterministic progress path. Structural contradictions return
typed corruption rather than a state record.

Rows outside physical membership are excluded from readiness. Owner-specific
physical artifacts that survive erasure/closure are a Slice-30/Slice-55
integrity failure and are caught by their raw-state/orphan checks; they are not
reclassified as current work.

The shared membership implementation replaces the hand-copied node and edge
fragments used by scheduler, pending probe, completion, and worker publication.
The scheduler still uses its cursor as an optimization, but the shared pending
probe must also find scheduler-pending rows below that cursor. A non-empty
below-watermark set rewinds the cursor before dispatch. The existing
`reenqueue_stranded_vector_rows` path remains the legitimate-stranded repair.

## Boundary-qualified readiness

Each status call starts one reader transaction and computes:

- `effective_at_epoch_s`: the one instant used by every edge-membership arm;
- `observed_boundary`: `load_next_cursor` in that snapshot;
- `pending_count`: physical members lacking valid three-part dense
  completion and not carrying a `failed` terminal;
- `failed_count`: physical members carrying `failed`;
- `first_incomplete_cursor`: the minimum cursor in either set; and
- `ready_through`: `observed_boundary` when both sets are empty, otherwise
  `first_incomplete_cursor - 1`.

`ready_through = B` means: every physical projection member with cursor at most
B is complete. It does **not** assert that every integer cursor is a projection
owner. Erased owners leave no residue. Node membership is lifecycle-independent;
only erasure or an active Slice-30 physical closure fence removes a node member.
The explicitly governed edge-closure rule may also remove an edge member.
Reserved, rolled-back, operational,
redaction, closure, consolidation, audit, and migration cursors affect the
observed high-water but never become phantom projection work.

Readiness is derived in this order:

1. invalid generation or physical authority → typed corruption;
2. `origin = legacy_unverified` → `degraded`;
3. `failed_count > 0` → `degraded`;
4. `pending_count = 0` → `ready`;
5. no attached embedder → `blocked`;
6. attached but equivalence-refused/initialization-failed runtime → `deferred`;
7. usable runtime → `processing`.

Counts are exact checked `u64` values. The implementation uses indexed UNION
arms over physical node and edge members and point probes into terminal, sidecar,
and vec0. `EXPLAIN QUERY PLAN` fixtures reject an unindexed canonical-table
scan introduced by correlation fields; the bounded aggregate itself may visit
the physical member set.

Existing coarse surfaces map conservatively: `ready` to `ready`, `processing`
to `embedding`, `blocked`/`deferred` to `unavailable`, and `degraded` to
`embedding`, never `ready`. `read_embedding_readiness` remains scheduler/session
availability and is documented as weaker than generation completeness.

## Mutation-to-ready correlation

Slice 40 strengthens Slice 25's receipt construction without changing the
meaning of an existing field. After all operations are applied but before the
receipt is stored, the actuation transaction classifies every affected write
cursor with `projection_completion_v1`. It includes each physical member that
is not complete, including no-runtime pre-enrolment nodes and body-bearing
edges. Newly written rows cannot be partially published before this transaction
commits; failure/partial-publication cases remain addressable because their
cursor was already recorded as pending at commit.

The same transaction reads the current generation, constructs the sorted
deduplicated pending list, and stores that generation whenever the list is
non-empty. Configuration and worker publication cannot race this check because
the immediate actuation transaction holds the SQLite writer lock. A no-runtime
receipt can therefore replay unchanged, report blocked/deferred, and later
report ready after a usable reopen/graft/drain.

The public mutation query is:

```text
MutationProjectionStatusRequestV1 {
  schema_version: 1,
  operation_id: OperationId,
  write_cursor: u64,
  expected_generation_id: ProjectionGenerationId,
}
```

The Engine point-reads `_fathomdb_actuation_receipts` by primary-key
`operation_id`, validates the stored receipt, proves `write_cursor` is in its
bounded pending list, and compares the required expected ID with the stored
generation. It then evaluates that physical member through the same membership
and completion function.

This order is normative after dynamic input validation:

1. open/closing state;
2. receipt existence and persisted receipt integrity;
3. operation ID equality and pending-list membership;
4. expected versus receipt generation;
5. receipt generation still serving; and
6. current physical membership/completion.

A missing receipt/cursor is `mutation_not_tracked`; an expected/stored mismatch
is `wrong_projection_generation`; a correctly expected retired generation is
`projection_generation_unavailable`. A pre-step-32 pending receipt with NULL
generation returns `projection_generation_unavailable` and is never rebound.
Receipt redaction preserves the existing `erased` tombstone and empty arrays;
mutation status maps that tombstone to non-disclosing `mutation_not_tracked`,
while ordinary actuation replay keeps its accepted `operation_id_erased`
behavior. Source erasure normally redacts the referenced receipt and therefore
also returns `mutation_not_tracked`; if a receipt has no erased source reference
but its owner is gone, status returns `projection_generation_unavailable`.

“Replay unchanged” means the durable canonical receipt fields, including the
nullable generation, rehydrate identically under the current additive response
schema. It does not claim that a pre-step-32 serialized response gains no
additive nullable field.

The response carries schema version, operation ID, write cursor, generation
ID, effective instant, state, observed boundary, ready-through,
pending/failed counts for the addressed owner (0 or 1), and runtime state. The
state enum is only `ready|processing|blocked|deferred|degraded`; unavailable and
wrong-generation conditions are errors, not response states.

## Public API, wire, and error contract

The normative Rust shapes are:

```text
ProjectionReadinessV1 = Ready | Processing | Blocked | Deferred | Degraded
ProjectionRuntimeStateV1 = Absent | Usable | Refused
ProjectionGenerationOriginV1 = Fresh | LegacyUnverified | Configuration | Rebuild

ProjectionGenerationStatusV1 {
  schema_version: u32,
  generation_id: ProjectionGenerationId,
  declaration_sha256: String,
  origin: ProjectionGenerationOriginV1,
  transition_boundary: u64,
  effective_at_epoch_s: i64,
  observed_boundary: u64,
  ready_through: u64,
  readiness: ProjectionReadinessV1,
  runtime_state: ProjectionRuntimeStateV1,
  pending_count: u64,
  failed_count: u64,
}

MutationProjectionStatusV1 {
  schema_version: u32,
  operation_id: String,
  write_cursor: u64,
  generation_id: ProjectionGenerationId,
  effective_at_epoch_s: i64,
  observed_boundary: u64,
  ready_through: u64,
  readiness: ProjectionReadinessV1,
  runtime_state: ProjectionRuntimeStateV1,
  pending_count: u64,
  failed_count: u64,
}
```

`effective_at_epoch_s` is a signed integer epoch second in Rust and the existing
safe integer representation in Python/TypeScript; it is not a write cursor.
Canonical response fixtures order fields exactly as displayed. Binding enum
strings are lower snake case.

Rust adds pure methods `Engine::read_projection_generation_status()` and
`Engine::read_mutation_projection_status(request)`. Python exposes frozen
dataclasses plus `read.projection_generation_status(engine)` and
`read.mutation_projection_status(engine, request)`. TypeScript exposes readonly
interfaces plus `read.projectionGenerationStatus(engine)` and
`read.mutationProjectionStatus(engine, request)`. They do not become unrelated
top-level Engine facade verbs.

All Python, TypeScript, wire, and dynamic-binding `u64` values are canonical
unsigned decimal strings, matching Slice 25; Rust stores them as `u64`.
TypeScript never accepts numeric substitutes. Leading zeroes, signs,
whitespace, exponent notation, booleans, and values above `u64::MAX` reject.

Requests reject unknown fields and schema versions. Response readers ignore
additive fields but reject unknown schema versions and semantic enum values.
Field paths are RFC 6901 camel-case wire paths. Canonical fixtures pin Rust,
Python, TypeScript, large values, every enum/error, and exact bytes.

`EngineError::ProjectionGeneration` maps to stable code
`FDB_PROJECTION_GENERATION`, Python `ProjectionGenerationError`, and TypeScript
`ProjectionGenerationError`. Closed reasons are:

- `unsupported_schema_version`;
- `unknown_field`;
- `invalid_operation_id`;
- `invalid_write_cursor`;
- `invalid_generation_id`;
- `mutation_not_tracked`;
- `wrong_projection_generation`;
- `projection_generation_unavailable`; and
- `projection_generation_corrupt`.

Dynamic validation order is schema version, lexicographically first unknown
field, operation ID, canonical write cursor, then generation ID. Bindings must
not allow native conversion to throw before that order.

## Transition and mutation-path classification

Every current physical mutation path is classified below. The implementation
adds a closed-source audit so a future unclassified mutator fails tests.

| Path | Generation action | Readiness/visibility action |
|---|---|---|
| Ordinary write/batch/actuation | Reuse | Receipt records current ID when async cursors exist; sync rows and pending dense state commit atomically. |
| Worker vector publication/failure | Reuse | Revalidate owner, publish terminal/sidecar/vec0 atomically, invalidate visibility. |
| Exact config replay | Reuse | No write and no visibility change. |
| Non-noop config or drop | Mint | Retire/install metadata in configuration transaction; synchronous cleanup/backfill and dense re-enrolment are visible under new ID. |
| Full or vec0 operator rebuild | Mint | Freeze/drain, retire/install metadata in rebuild transaction, rebuild in place, expose processing/degraded until complete. |
| Boot registry rederive | Reuse | Complete atomically before Engine publication; visibility changes if physical rows change. |
| Boot/late runtime graft and unstrand | Reuse | Enrol/reopen pending work atomically, then notify after commit. |
| Pending to active lifecycle | Reuse | Add sync attributes atomically; retained node dense membership is unchanged. |
| Supersede/invalidate/close | Reuse | Retain node FTS/dense history for relaxed reads; remove an edge from its current dense membership under the shipped edge rule. |
| Purge/source erasure | Reuse | Remove owner, terminal, and physical rows under Slice-30 fence; no generation residue. |
| Historical tokenizer repair | Reuse | Predates generation bootstrap and completes before Engine publication. Any future tokenizer semantic change must mint. |
| Operator projection repair | Mint | Must use governed rebuild. Raw or application-owned repair is prohibited. |

Configuration and rebuild transitions do not rewrite historical receipts.
`ProjectionJob` gains the generation ID captured with its cursor/kind/body from
the dispatcher's one read snapshot. Worker commit compares that expected ID
with the current singleton inside its immediate transaction, after the
Slice-30 physical fence and owner-membership check. A mismatch writes no
terminal, sidecar, vec0, mean state, or failure record; it discards the computed
result and requests a new pending scan. The current generation then rediscovers
the still-incomplete owner normally.

Configuration and rebuild still freeze/drain where they already do, but the
captured ID is the defensive correctness rule for queued, computing,
write-lock-waiting, and publication-race jobs. Race tests pause at all four
points and prove either old publication-before-transition or discard and
rediscovery-after-transition—never cross-generation publication.

## Frozen-read binding

The Slice-35 token prefix, fields, HMAC, maximum size, and codec remain v1.
Slice 40 changes only the serving-digest algorithm to v2.

The v2 digest is SHA-256 over domain
`fathomdb.projection-serving-binding.v2\0`, then the current generation's
`generation_id`, `declaration_sha256`, `transition_boundary`, `role`, and
`origin`, followed by the exact Slice-35 v1 encoding: ordered
`_fathomdb_projection_state` rows first, then ordered
`_fathomdb_projection_terminal` rows. Scalars, options, collections, and
integers use the existing Slice-35 codec. Sidecar/vec0 bytes are not added to
the bounded digest; their existing real-table visibility triggers remain the
physical-row invalidation authority. No receipt row enters this serving digest.

Golden fixtures pin both complete pre-hash byte streams and their v1/v2 hashes.
A pre-step-32 token consumed after migration returns existing non-disclosing
state drift. A post-step-32 token is stable across restart when state is
unchanged and drifts on generation transition or readiness/physical changes.

Step 32 extends the visibility-trigger manifest with exactly:

- `_fathomdb_read_visibility_pg_ai`, `_au`, `_ad` on
  `_fathomdb_projection_generations`; and
- `_fathomdb_read_visibility_pc_ai`, `_au`, `_ad` on
  `_fathomdb_projection_generation_current`.

The receipt generation column is correlation metadata and does not affect
serving visibility. Virtual-table writes remain transactionally coupled to
existing real owner/terminal/sidecar rows. The source audit proves every new
generation-authority mutation is trigger-covered.

## Lifecycle, erasure, concurrency, and restart

Slice-30 closure remains the publication fence. Worker commit obtains the write
transaction, checks no physical closure is pending, and revalidates physical
membership and generation before any terminal/sidecar/vec0 publication.
Closure or generation transition winning first causes stale publication to be
discarded; publication winning first is removed by the following closure or
rebuild transaction.

Erasure removes canonical bytes and all owner-specific physical/terminal state.
Generation history contains no source ID, body, locator, operation ID, or
cursor assignment. Existing secure-delete, WAL, orphan, and closure proofs
remain authoritative. A status call after erasure returns a non-disclosing
typed unavailable result.

Open validates authority before spawning readers/workers. Boot repair happens
before Engine publication. A crash before a configuration/rebuild transaction
commits preserves the old generation and stores; a crash after commit observes
the new generation and resumes current pending work. Frozen readers linearize
before or after the transaction; no half-transition is visible.

## Performance and storage contract

The measurement contract is committed before candidate runs. The runtime
baseline is Slice-35 product commit `0aff1cb08c61a8bb2a004813bbd5604b6ff1a403`;
the source closeout boundary is `cb62921f`. The candidate is the reviewed Slice
40 GREEN commit. Inputs, query order, database seed, and five repetitions are
identical.

Raw artifacts live under
`/home/coreyt/projects/fathomdb/data/performance-benchmarking/scale-02/slice40-runs/`.
Portable configs and receipts live under
`experiments/configs/scale-02/slice40-*.json` and
`experiments/runs/scale-02-slice40-*`.

Preregistered bounds:

- storage/write fixture: exactly 10,000 canonical node operations, 128
  operations per actuation batch except the final tail (79 receipts), one
  declared dense projection, no runtime during writes, and therefore 100% of
  receipts carrying pending cursors and a generation ID; checkpoint WAL with
  `PRAGMA wal_checkpoint(TRUNCATE)` before file-size measurement;
- ordinary synchronous actuation-write p50 and p95 95% upper relative
  regression at most 3%;
- generation metadata plus nullable receipt-column database-size increase at
  the fixed 10k fixture at most 64 KiB after checkpoint, with the two generation
  tables and receipt-column contribution reported separately and WAL bytes
  reported before and after checkpoint;
- current-generation and mutation-status p95 at most 5 ms and p99 at most
  10 ms at 50k;
- open/restart p95 upper regression at most 10% or 25 ms absolute, whichever is
  larger;
- configuration/rebuild metadata transition cost reported separately from
  existing backfill, with no hidden O(N) assignment rewrite; and
- CPU/CUDA status p95 difference at most 2 ms on the same database. Embedding
  throughput is reported but not attributed to status.

Runs report cold/steady state, failures, timeouts, page/WAL bytes, retained
generation rows, query plans, full-owner scan count at 50k, exact operation and
batch counts, percentage of pending receipts, runtime/device, and
candidate/baseline source and artifact hashes. A separate one-operation-batch
observation reports per-receipt logical and physical amplification without
changing the registered representative bound. A miss blocks closeout or needs
a newly preregistered treatment; thresholds are not rewritten after observation.

## TDD and verification

RED is committed separately and uses real SQLite databases.

| Target | Required proof |
|---|---|
| `step32_projection_generation` | Additive shape, checks/indexes/triggers, fresh/upgrade bootstrap, no content migration, partial/corrupt authority rejection. |
| `slice40_projection_generation` | ID grammar/collision retry, restart/copy/no reuse, declaration digest goldens, no-op stability, config/rebuild mint, legacy degradation. |
| `slice40_projection_completion` | Exact node/edge physical membership, every state-table row, below-watermark rediscovery, no-runtime, late graft, inactive/superseded/out-of-window nodes, edge expiry, failure, erasure, and unsupported kinds. |
| `slice40_mutation_projection_status` | Same-predicate pending construction, no-runtime receipt to blocked/deferred/ready, persist/replay, operation/cursor membership, required expected ID, retired/legacy/redacted/erased behavior, canonical wire/property round trips. |
| `slice40_projection_generation_races` | Write/publication, closure, erasure, configure/rebuild, restart at transition points, duplicate publication, captured-generation stale-job discard at queued/computing/lock/publish seams, and old/new reader linearization. |
| `slice35_frozen_read` additions | Token codec unchanged, v1-to-v2 drift, generation/readiness drift, restart stability, trigger/source manifest. |
| Binding/package parity | Rust/Python/TypeScript names, shapes, validation precedence, `u64::MAX`, fresh wheel/npm imports and offline runtime smoke. |
| Performance receipt | Exact operations/calls/artifacts and all registered latency/storage/reopen bounds. |

Mandatory platform routes are:

- host CPU: focused Rust/Python/TypeScript tests and fresh local native artifacts;
- RTX 3090 CUDA: `nvidia-smi`, then a fresh Python wheel built with exact
  features `pyo3/extension-module,embed-cuda` and N-API artifact with
  `default-embedder,embed-cuda`, followed by the generation/readiness reopen
  smoke and `scripts/release/cuda-preflight.sh` witness validation;
- Windows cross-build on the release workflow's `windows-latest` runner:
  PyO3/maturin action arguments `--release --out dist --features
  pyo3/extension-module,default-embedder -i python3.11`, then from `src/ts`
  `npm ci`, `CARGO_BUILD_TARGET=x86_64-pc-windows-msvc npm run build:native`,
  and `npm exec -- tsc -p tsconfig.build.json`; and
- Windows native, when the documented VM is reachable:
  `scripts/release/smoke/smoke-local-native-artifacts.ps1` over freshly built
  artifacts. Runtime unavailability is recorded but never substituted for the
  mandatory cross-build.

The enclosing gate is `./scripts/agent-verify.sh --tier=fast`, plus applicable
operator/heavy routes and full-workspace Clippy/check. Ptrace checks run
unchanged outside the sandbox. CUDA and Metal are separate feature routes.

## Forward obligations

- Slice 45 binds governed pagination to the current generation.
- Slice 50 evidence resolution rechecks generation and eligibility.
- Slice 55 diagnoses generation/physical inconsistencies without semantic
  claims.
- Slice 60 reports constrained graph origin under the frozen context.
- Slice 75 audits installed parity and separately investigates vector latency.
- Rich work manifests, side-by-side stores, and historical-generation reads
  require a future successor design.
