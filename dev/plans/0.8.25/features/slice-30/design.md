---
title: 0.8.25 Slice 30 — core lifecycle and erasure closure design
status: DRAFT_RECONCILED_AWAITING_REVIEW
design_version: 3
depends_on: 25
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 30 design

## Authority, disposition, and limits

This design implements S30-R1 through S30-R5, the retained core of
R25/AC25-30, N25-01/N25-02, Memex needs 5/6, and A25-05. It succeeds the
dependency-stage refusal in Slice 25 while preserving the existing lifecycle,
erasure, provenance, dependency, and projection authorities.

The implemented dependency shape remains exactly Slice 20's one pinned
canonical source revision to one complete derived node or edge revision.
Multi-source liveness, derived-to-derived recursion, scheduled validity-loss
closure, and caller-configurable consequence policy belong to 0.8.26. Slice 30
does not add an `invalidated` state, infer a replacement, restore a dependent,
or make a semantic decision.

The design uses only landed write boundaries, dependency generations,
projection cursors/terminal rows, and source links. Projection-generation
identity is Slice 40; evidence resolution is Slice 50. Their plans must consume
the visibility rule defined here, but their future types do not appear in the
Slice 30 contract.

## Existing substrate and exact delta

Already present are immutable artifact/source revisions and source links
(schema step 27), direct dependency registration and reciprocal indexes (step
28), compact actuation receipts with reserved closure fields (step 29), node
lifecycle transitions, edge currentness/validity, registry-driven projection
erasure, source/purge erasure, telemetry redaction, WAL completion, and typed
`ErasureIncomplete`.

Slice 30 adds:

- durable content-free closure operations and bounded work rows;
- atomic root-mutation/barrier admission;
- one shared strict dependency-eligibility and active-closure read guard;
- deterministic node/edge soft and physical consequences;
- bounded read/list/resume APIs and a structured zero proof;
- parity codecs/errors in Rust, Python, and TypeScript; and
- closure admission from ordinary writes, lifecycle verbs, erasure verbs, and
  Slice 25 actuation.

## Closed public contract

```text
ClosureRootV1 =
  SourceRevision { source_revision_id }
  | SourceBucket { source_id }

ClosureCauseV1 = superseded | soft_deleted | purged | source_erased
ClosurePhaseV1 = admitted | propagating | proving | at_rest_pending |
                 complete | incomplete
ClosureEffectV1 = soft_delete_node | retire_edge |
                  hard_erase_node | hard_erase_edge

ClosureLookupV1 { schema_version: 1, closure_operation_id }
ClosureResumeV1 {
  schema_version: 1, closure_operation_id, max_work: 1..1000
}
ClosureListRequestV1 {
  schema_version: 1, after_closure_operation_id?, limit: 1..100
}
ClosureListPageV1 {
  schema_version: 1, items: [ClosureStatusV1], next_after_closure_operation_id?
}
ClosureStatusV1 {
  schema_version: 1,
  closure_operation_id,
  root: ClosureRootV1,
  cause: ClosureCauseV1,
  phase: ClosurePhaseV1,
  admitted_write_boundary,
  admitted_dependency_generation,
  total_count,
  processed_count,
  pending_count,
  blocker_code?,
  proof?
}
ClosureProofV1 {
  schema_version: 1,
  proof_write_boundary,
  current_active_dependent_nodes: 0,
  current_derived_edges: 0,
  strict_searchable_dependents: 0,
  ownerless_projection_rows: 0,
  post_admission_registrations: 0,
  remaining_dependency_rows?,
  remaining_canonical_rows?,
  remaining_projection_rows?,
  remaining_receipt_reference_rows?
}

Engine::read_dependency_closure(ClosureLookupV1)
  -> Result<Option<ClosureStatusV1>, EngineError>
Engine::list_incomplete_dependency_closures(ClosureListRequestV1)
  -> Result<ClosureListPageV1, EngineError>
Engine::resume_dependency_closure(ClosureResumeV1)
  -> Result<ClosureStatusV1, EngineError>
```

`next_after_closure_operation_id` is a feature-local lexical recovery scan, not
the governed data cursor designed by Slice 45. The next marker is present only
when another nonterminal row exists after the returned page.

Closure operation IDs are Engine-minted `_fdb:c:<64-lower-hex>` identifiers and
contain no source text. A standalone lifecycle call that has committed its root
mutation but cannot finish closure returns
`EngineError::DependencyClosureIncomplete { closure_operation_id, phase,
blocker_code }`. Python raises `DependencyClosureIncompleteError` with the same
fields; TypeScript rejects with code `FDB_DEPENDENCY_CLOSURE_INCOMPLETE` and
matching properties. The operation remains discoverable and resumable.

Malformed request fields return a versioned `DependencyClosureError` with the
closed reasons `unsupported_schema_version`, `unknown_field`,
`closure_operation_id_invalid`, `work_limit_invalid`, and
`closure_not_found`. Registration/reactivation uses existing
`DependencyError` with two additive reasons:
`dependency_source_ineligible` and `dependency_closure_active`. Persisted-row
corruption remains `EngineError::Storage`; it is never recast as caller error.

Unknown request fields/variants reject before execution. Older clients may
ignore additive response fields, but reject unknown root/cause/phase/effect or
error variants. Counts and boundaries are `u64` in Rust and canonical unsigned
decimal strings in Python/TypeScript. `max_work` and list `limit` remain JSON
numbers. Status contains counts, never an unbounded affected-ID list.

## Persistence

Schema step 30 adds two content-free tables:

```sql
CREATE TABLE _fathomdb_dependency_closures(
  schema_version INTEGER NOT NULL CHECK(schema_version = 1),
  closure_operation_id TEXT PRIMARY KEY,
  root_kind TEXT NOT NULL CHECK(root_kind IN (
    'source_revision','source_bucket'
  )),
  root_value TEXT NOT NULL,
  cause TEXT NOT NULL CHECK(cause IN (
    'superseded','soft_deleted','purged','source_erased'
  )),
  admitted_write_boundary INTEGER NOT NULL CHECK(admitted_write_boundary >= 0),
  admitted_dependency_generation INTEGER NOT NULL
    CHECK(admitted_dependency_generation >= 0),
  phase TEXT NOT NULL CHECK(phase IN (
    'admitted','propagating','proving','at_rest_pending',
    'complete','incomplete'
  )),
  total_count INTEGER NOT NULL CHECK(total_count >= 0),
  processed_count INTEGER NOT NULL CHECK(
    processed_count >= 0 AND processed_count <= total_count
  ),
  blocker_code TEXT,
  proof_write_boundary INTEGER,
  proof_json TEXT,
  CHECK((phase = 'complete') = (proof_json IS NOT NULL)),
  CHECK(proof_json IS NULL OR json_valid(proof_json))
);
CREATE INDEX _fathomdb_dependency_closures_recovery
  ON _fathomdb_dependency_closures(phase, closure_operation_id);
CREATE INDEX _fathomdb_dependency_closures_root
  ON _fathomdb_dependency_closures(root_kind, root_value, phase);

CREATE TABLE _fathomdb_dependency_closure_work(
  closure_operation_id TEXT NOT NULL,
  dependency_id TEXT NOT NULL,
  derived_revision_id TEXT NOT NULL,
  artifact_class TEXT NOT NULL CHECK(artifact_class IN ('node','edge')),
  effect TEXT NOT NULL CHECK(effect IN (
    'soft_delete_node','retire_edge','hard_erase_node','hard_erase_edge'
  )),
  work_state TEXT NOT NULL CHECK(work_state IN ('pending','complete')),
  PRIMARY KEY(closure_operation_id, dependency_id),
  UNIQUE(closure_operation_id, derived_revision_id)
);
CREATE INDEX _fathomdb_dependency_closure_work_pending
  ON _fathomdb_dependency_closure_work(
    closure_operation_id, work_state, derived_revision_id
  );
```

All joins validate the source link, dependency row, revision registry, and
artifact class. Any disagreement is `Storage`. Content, source locators, source
IDs, logical IDs, and bodies never enter work rows, blocker codes, or proofs.
Erasure may retain completed content-free closure audit rows.

## Admission and barriers

The single SQLite writer admits closure in the same `BEGIN IMMEDIATE`
transaction as the root mutation. It:

1. fixes the root and cause, current write boundary, dependency generation,
   and operation instant;
2. rejects an overlapping nonterminal closure over the same source revision or
   source bucket;
3. materializes the complete direct work set with one indexed
   `INSERT ... SELECT`, choosing effect from artifact class and physical versus
   soft cause;
4. records the exact total; and
5. applies the root mutation and commits.

The nonterminal closure row is the barrier. A source-revision barrier also
matches a source-bucket closure containing that revision. From admission until
proof, all provenance writes and dependency registrations referencing that
source reject `dependency_closure_active`; no later dependency can escape the
fixed work set. Total impact has no refusal cap. The `1..1000` limit bounds each
resume transaction, not the closure.

Ordinary writes that supersede more than one depended-on source admit one
closure per source revision. Purge does the same for depended-on revisions of
the logical record. Source erasure admits one source-bucket closure. All of a
single root call's barriers and root mutation commit atomically; a conflict
rolls back the call.

Slice 25 actuation replaces `dependency_closure_required` with this admission.
A batch that admits closure stores `committed_closure_pending` and the ordered
closure IDs in its already-versioned receipt. This outcome records the state at
batch commit and is immutable even if closure later completes; callers resolve
current state through `read_dependency_closure`. Exact replay returns the same
receipt.

Standalone `write`, `transition`, `purge`, `erase_source`, and operator
`excise_source` drive admitted operations to terminal before reporting success.
If a post-admission step fails, their root mutation remains committed and they
return `DependencyClosureIncomplete`; retry first discovers and resumes the
matching nonterminal operation instead of reapplying the root mutation.

## Strict eligibility and visibility fencing

One internal predicate is shared by canonical point/list, FTS, vector, graph,
property, expansion, and later evidence paths. Before each arm's candidate,
seed, or frontier truncation, a registered derived revision is eligible only
when its pinned canonical source revision:

- exists with complete provenance;
- is current (`superseded_at IS NULL`);
- is active for a node, or current and in-force for an edge;
- is valid at that operation's fixed instant; and
- is not matched by a nonterminal source-revision or source-bucket closure.

The same predicate applies when registering a dependency or reactivating a
derived node. `valid_from` becoming effective never auto-reactivates a
dependent. Scheduled detection of a future `valid_until` loss is deferred to
0.8.26, but every read still evaluates current strict validity. Missing tables,
malformed rows, limit exhaustion, or inability to prove eligibility fails the
whole operation closed; it never degrades to an unguarded result.

Projection workers do not publish work for fenced derived revisions. A work
item already in flight may finish, but remains ineligible under the shared
guard and must be drained or removed before closure proof.

## Consequence processing

Each resume transaction selects at most `max_work` pending rows in
`derived_revision_id, dependency_id` order and performs exactly one fixed
effect:

| Root cause | Derived node | Derived edge |
| --- | --- | --- |
| `superseded` / `soft_deleted` | Set the exact current derived revision to `deleted`, retain dependency/provenance audit, and remove projections without an independent lifecycle read filter. | Retire the exact current edge revision through `superseded_at`, retain dependency/provenance audit, and remove its row-owned projections. |
| `purged` / `source_erased` | Physically erase the exact revision, its row-owned projections, source links/revision rows, receipt refs, and any graph rows whose integrity depends on its logical endpoint. | Physically erase the exact edge revision, its row-owned projections, source links/revision rows, and receipt refs. |

Already-ineligible/absent soft targets are idempotent success only after their
persisted identity/dependency chain validates. Missing physical targets are
idempotent only when the work row proves an earlier completed effect; otherwise
they are corruption. A work effect and its `complete` transition share one
transaction. Physical deletion advances the dependency generation exactly once
per transaction if registrations are removed; soft closure retains immutable
registrations and does not advance it.

Dependents are never reactivated automatically. A node can return only through
the existing caller decision after the pinned source again satisfies strict
eligibility. A retired or physically erased edge requires a new caller-authored
revision and dependency.

## Proof and at-rest completion

After no pending work remains, the operation enters `proving`. The proof runs
against a later write boundary and is not inferred from queue emptiness. Soft
closure requires zero current active dependent nodes, zero current derived
edges, zero strict-view searchable dependents, zero ownerless projection rows,
and zero post-admission registrations. Raw FTS/vector rows retained by existing
soft-delete policy do not fail proof if they have a valid owner and cannot be
returned; projections whose read path lacks a lifecycle/closure guard must be
removed.

Physical closure additionally requires zero affected dependency, canonical,
row-owned projection, source-link/revision, actuation-receipt-reference, and
pending projection rows. It then enters `at_rest_pending`, completes existing
telemetry redaction and `wal_checkpoint(TRUNCATE)`, and only afterward stores a
content-free proof and marks `complete`. A busy WAL or redaction failure leaves
the closure nonterminal with its barrier and returns the existing typed erasure
stage through `DependencyClosureIncomplete`; success is never reported early.

Proof and barrier retirement are the same transaction. A later failed or
corrupt proof sets `incomplete`, preserves the barrier, and records only a
closed non-content blocker code. Resume re-runs the unfinished phase
idempotently. Completed operations are immutable audit records.

## Compatibility, performance, and failure boundaries

No-dependent lifecycle/write/erasure calls retain their existing return shape
and fast path. The only new hot-read work is the indexed strict dependency/
closure anti-join, executed only for databases with registered dependencies;
an Engine-session cached zero-dependency/zero-barrier state may bypass it but
must invalidate on every relevant commit. Slice 75 measures this fast-path and
dependent-path overhead.

Closure work is O(number of direct registered dependents plus affected
row-owned projections), page-bounded, restart-safe, and deterministic. There is
no recursive traversal or unbounded public list. Ordinary lifecycle closure
does not promise physical removal of every projection shadow; erasure does.

Rust, Python, TypeScript, and wire interfaces document the additive requests,
responses, errors, decimal boundaries, and unknown behavior in the same
change. Windows must compile and run the CPU/native lifecycle fixtures. CUDA,
live models, and network access are N/A.

## RED/GREEN and verification map

RED is committed before product code and covers:

- source supersession and soft deletion with active node and edge dependents;
- purge/source erasure with raw dependent body, FTS/property/vector, source
  link, receipt-reference, telemetry, and WAL canaries;
- registration and reactivation against inactive, superseded, invalid, and
  barriered sources;
- immediate pre-truncation invisibility while the last row of a greater-than-
  one-page closure remains unprocessed;
- bounded progress, restart at every durable phase, exact resume/replay, and
  deterministic incomplete listing;
- concurrent registration/admission, projection publication, and proof races;
- corrupt/missing dependency, work, projection, and proof rows failing closed;
- soft-proof versus physical-proof projection semantics;
- schema migration contiguity and property-based status/work invariants; and
- strict Python/TypeScript codecs, error parity, installed package behavior,
  and existing no-dependent compatibility.

GREEN runs focused schema/Engine/binding tests, fast verification, heavy
erasure/concurrency tests, all/all-feature/operator routes, installed wheel and
N-API smokes, and Windows CPU/native Rust/Python/Node routes. The final verifier
must inspect raw storage and rerun the unchanged ptrace-capable strict gate
outside the sandbox if required.

## Forward obligations

- 0.8.26 extends the same barriers/proofs to multi-source and bounded recursive
  dependencies, including scheduled validity-loss closure.
- Slice 35 binds its optional frozen context and eligibility envelope to the
  dependency generation and active-closure state.
- Slice 40 succeeds cursor-only proof with generation-aware readiness.
- Slice 50 evidence resolution rechecks this guard.
- Slice 55 diagnoses closure/work/barrier corruption without semantic repair.
- Slice 60 applies the guard before graph seed/frontier truncation.
- Slice 75 verifies installed parity and representative latency/concurrency.
