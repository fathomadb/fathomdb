---
title: 0.8.25 Slice 40 — core projection generation and readiness design
status: DRAFT_FIX_2
design_version: 5
review_cycle: 2
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
whether its currently applicable dense rows are complete. It does not turn the
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
3. A dense-applicable live owner is complete only when an `up_to_date`
   terminal, a matching `_fathomdb_vector_rows` sidecar, and a vec0 row all
   exist in the same snapshot. An `up_to_date` terminal alone is never proof.
4. Generation readiness ranges over currently applicable live owners, not
   every integer write cursor. Erased rows, operational cursors, reserved
   migration cursors, rolled-back reservations, and inactive owners cannot
   create permanent readiness holes.
5. A Slice-25 receipt is correlated to the generation current at its commit.
   It is never rebound to a later generation and is replayed byte-for-byte.
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

ALTER TABLE _fathomdb_actuation_receipts
  ADD COLUMN projection_generation_id TEXT;
```

No per-record generation assignment table is added. It would duplicate the
current owner and receipt authorities, require O(N) rewrites on a global epoch
transition, and create erasure-sensitive cursor gaps. The nullable receipt
column is the compact mutation-to-generation correlation selected by this
slice.

The receipt column is non-NULL exactly when a committed receipt's
`pending_projection_write_cursors_json` is non-empty. It is NULL for refused,
erased, empty-pending, and pre-step-32 receipts. A non-NULL value must be a
valid generation that was serving at receipt commit. Receipt persistence,
hydration, replay validation, canonical serialization, and redaction include
the field. A malformed combination is storage corruption; it is never repaired
or guessed from the current generation.

Open and both public reads validate one serving row, one matching singleton,
ID/digest grammar, legal role/retirement pairs, and receipt shape when a receipt
is addressed. Missing, duplicate, or contradictory authority is typed
`projection_generation_corrupt`.

### Bootstrap and migration truth

After step 32, Engine bootstrap executes in one immediate transaction:

- an empty database receives a `fresh` generation at boundary zero;
- a non-empty upgraded database receives `legacy_unverified` at the persisted
  `load_next_cursor` boundary and therefore reports `degraded`; and
- an existing valid generation is reused unchanged.

Bootstrap is idempotent. A partial authority row set fails closed. A legacy
generation becomes certifiable only through an explicit configuration change
or operator rebuild; an idempotent configuration replay does not launder it.

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
5. either an absent option tag or the persistent default embedder profile's
   `profile`, `name`, `revision`, and `dimension`, in that order.

The live schema version is excluded so unrelated migrations cannot mint an
epoch. SQL NULL differs from empty text. Learned `mean_vec`, device, runtime,
worker count, availability, equivalence-probe cache/results, and session state
are excluded. Vector identity remains embedder-owned.

A changed digest accepted by `configure_projections` mints a generation. An
exact idempotent replay does not. `rebuild_projections` and `rebuild_vec0`
always mint even when the digest is unchanged, because the physical serving
set is reconstructed.

## Exact applicability and completion

Slice 40 introduces one internal `projection_completion_v1` implementation
used by generation status, mutation status, scheduler pending probes, and new
integrity tests. Existing projection writers continue to own publication, but
their applicability predicates must delegate to the same helpers instead of
maintaining copies.

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

### Dense node owner

A current node is dense-applicable exactly when all are true in the read
snapshot:

1. it is current, active, and within its validity window;
2. Slice-30 dependency closure has no active barrier for its source;
3. `row_kind` is `leaf` or `coverage`;
4. its `kind` is accepted by `kind_is_vector_committable`; and
5. `vector_projection_declared` is true.

Runtime availability is not identity or applicability. Kind enrolment is
derived scheduler state, not policy; a declared, committable row remains
applicable while no runtime exists or before late enrolment. That state is
`blocked` or `deferred`, never ready.

### Dense edge owner

A current edge is dense-applicable exactly when all are true:

1. it has a body;
2. it is not superseded and is within edge validity;
3. Slice-30 dependency closure has no active barrier for its source; and
4. fixed kind `edge_fact` is committable.

This preserves the shipped edge rule: body-bearing edges auto-enrol
`edge_fact` independently of the projection registry. Runtime absence leaves
the owner applicable and blocked.

### Dense completion and failure

For either owner, completion requires all three:

1. `_fathomdb_projection_terminal.state = 'up_to_date'`;
2. `_fathomdb_vector_rows` contains the same cursor and expected kind; and
3. `vector_default` contains the same rowid and expected source type/kind.

The three are published in the worker transaction. An `up_to_date` terminal
without both physical rows is pending/degraded according to runtime and is
never complete. A `failed` terminal is a terminal quality failure and makes the
generation `degraded`; it is not silently counted as complete. Unsupported
node kinds are not applicable and remain reported by the existing
`vector_unsupported_kinds` surface.

Owner eligibility must be centralized across node/edge scheduler scans,
pending probes, completion reads, worker publication revalidation, lifecycle,
and erasure. The current `projection_owner_is_eligible` dependency guard is one
input, not the whole row-lifecycle predicate.

## Boundary-qualified readiness

Each status call starts one reader transaction and computes:

- `observed_boundary`: `load_next_cursor` in that snapshot;
- `pending_count`: current applicable owners lacking valid three-part dense
  completion and not carrying a `failed` terminal;
- `failed_count`: current applicable owners carrying `failed`;
- `first_incomplete_cursor`: the minimum cursor in either set; and
- `ready_through`: `observed_boundary` when both sets are empty, otherwise
  `first_incomplete_cursor - 1`.

`ready_through = B` means: every currently applicable live projection owner
with cursor at most B is complete. It does **not** assert that every integer
cursor is a projection owner. Erased/currently ineligible owners disappear
from the set and leave no residue. Reserved, rolled-back, operational,
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
arms over current node and edge owners and point probes into terminal, sidecar,
and vec0. `EXPLAIN QUERY PLAN` fixtures reject an unindexed canonical-table
scan introduced by correlation fields; the bounded aggregate itself may visit
the applicable owner set.

Existing coarse surfaces map conservatively: `ready` to `ready`, `processing`
to `embedding`, `blocked`/`deferred` to `unavailable`, and `degraded` to
`embedding`, never `ready`. `read_embedding_readiness` remains scheduler/session
availability and is documented as weaker than generation completeness.

## Mutation-to-ready correlation

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
generation. It then evaluates that current owner through the same applicability
and completion function.

This order is normative after dynamic input validation:

1. open/closing state;
2. receipt existence and persisted receipt integrity;
3. operation ID equality and pending-list membership;
4. expected versus receipt generation;
5. receipt generation still serving; and
6. current owner applicability/completion.

A missing receipt/cursor is `mutation_not_tracked`; an expected/stored mismatch
is `wrong_projection_generation`; a correctly expected retired generation is
`projection_generation_unavailable`; an erased or now-ineligible owner is also
`projection_generation_unavailable` and reveals no source data. Historical
NULL-generation receipts are unavailable, never rebound.

The response carries schema version, operation ID, write cursor, generation
ID, current generation ID, state, observed boundary, ready-through,
pending/failed counts for the addressed owner (0 or 1), and runtime state. The
state enum is only `ready|processing|blocked|deferred|degraded`; unavailable and
wrong-generation conditions are errors, not response states.

## Public API, wire, and error contract

Rust adds immutable versioned request/response types and pure Engine methods.
Python exposes frozen dataclasses plus
`read.projection_generation_status(engine)` and
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
| Pending to active lifecycle | Reuse | Add sync attributes and newly applicable dense state atomically. |
| Supersede/invalidate/close | Reuse | Remove owner from current applicable set and close physical visibility atomically. |
| Purge/source erasure | Reuse | Remove owner, terminal, and physical rows under Slice-30 fence; no generation residue. |
| Historical tokenizer repair | Reuse | Predates generation bootstrap and completes before Engine publication. Any future tokenizer semantic change must mint. |
| Operator projection repair | Mint | Must use governed rebuild. Raw or application-owned repair is prohibited. |

Configuration and rebuild transitions do not rewrite historical receipts.
Outstanding work from the retired epoch may finish computation, but commit
must revalidate owner eligibility and the current generation; stale results are
discarded. The new generation derives readiness over current physical truth.

## Frozen-read binding

The Slice-35 token prefix, fields, HMAC, maximum size, and codec remain v1.
Slice 40 changes only the serving-digest algorithm to v2.

The v2 digest is SHA-256 over domain
`fathomdb.projection-serving-binding.v2\0`, then the current generation's
`generation_id`, `declaration_sha256`, `transition_boundary`, `role`, and
`origin`, followed by the exact existing Slice-35 ordered state/terminal/vector
binding bytes. Scalars, options, collections, and integers use the existing
Slice-35 codec. No receipt row enters this serving digest.

Golden fixtures pin the v1 digest before step 32 and v2 after step 32. A
pre-step-32 token consumed after migration returns existing non-disclosing
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
transaction, checks no physical closure is pending, and revalidates current
owner eligibility and generation before any terminal/sidecar/vec0 publication.
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

- ordinary synchronous actuation-write p50 and p95 95% upper relative
  regression at most 3%;
- generation metadata plus nullable receipt-column database-size increase at
  10k at most 64 KiB after checkpoint, with WAL bytes reported separately;
- current-generation and mutation-status p95 at most 5 ms and p99 at most
  10 ms at 50k;
- open/restart p95 upper regression at most 10% or 25 ms absolute, whichever is
  larger;
- configuration/rebuild metadata transition cost reported separately from
  existing backfill, with no hidden O(N) assignment rewrite; and
- CPU/CUDA status p95 difference at most 2 ms on the same database. Embedding
  throughput is reported but not attributed to status.

Runs report cold/steady state, failures, timeouts, page/WAL bytes, retained
generation rows, query plans, exact operation counts, runtime/device, and
candidate/baseline source and artifact hashes. A miss blocks closeout or needs
a newly preregistered treatment; thresholds are not rewritten after observation.

## TDD and verification

RED is committed separately and uses real SQLite databases.

| Target | Required proof |
|---|---|
| `step32_projection_generation` | Additive shape, checks/indexes/triggers, fresh/upgrade bootstrap, no content migration, partial/corrupt authority rejection. |
| `slice40_projection_generation` | ID grammar/collision retry, restart/copy/no reuse, declaration digest goldens, no-op stability, config/rebuild mint, legacy degradation. |
| `slice40_projection_completion` | Exact node/edge applicability; terminal+sidecar+vec0 completion; no-runtime, late graft, pending-to-active, supersession, failure, erasure, and unsupported kinds. |
| `slice40_mutation_projection_status` | Receipt persist/replay, operation/cursor membership, required expected ID, retired/legacy/erased behavior, canonical wire/property round trips. |
| `slice40_projection_generation_races` | Write/publication, closure, erasure, configure/rebuild, restart at transition points, duplicate/stale publication, old/new reader linearization. |
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
