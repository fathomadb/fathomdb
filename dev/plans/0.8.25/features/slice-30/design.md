---
title: 0.8.25 Slice 30 — core lifecycle and erasure closure design
status: DRAFT_REVIEW_FIX_3
design_version: 6
review_fix: 3
review: design-review-cycle3.md
depends_on: 25
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 30 design

## Authority, disposition, and limits

This design implements S30-R1 through S30-R5, the retained core of
R25/AC25-30, N25-01/N25-02, Memex needs 5/6, and A25-05. It succeeds Slice
25's dependency-stage refusal while preserving the existing lifecycle,
erasure, provenance, dependency, projection, and `ReadView` authorities.

The dependency shape remains exactly Slice 20's one pinned canonical source
node revision to one complete derived node or edge revision. Multi-source
liveness, derived-to-derived recursion, scheduled validity-loss closure, and
configurable consequence policy belong to 0.8.26. Slice 30 adds no
`invalidated` state, semantic decision, replacement, or automatic restoration.
Public crash-journal administration remains parked.

The design uses only landed write boundaries, dependency generations,
projection cursors/terminal rows, and source links. Projection generations and
evidence references remain owned by Slices 40 and 50.

## Existing substrate and exact delta

Schema steps 27–29 already provide immutable artifact/source revisions,
authoritative source links, indexed direct dependencies, independent dependency
generation, and terminal actuation receipts. Existing code owns node lifecycle,
edge currentness/validity, registry-driven projection removal, source/purge
erasure, telemetry redaction, WAL completion, and typed
`ErasureIncomplete`.

Slice 30 adds one content-free closure/proof table; atomic direct-dependent
effects; a shared dependency visibility predicate; a keyed status read for
Slice 25 closure IDs; internal restart finalization; two dependency refusal
reasons; and cross-SDK parity. It adds no prepared request journal and no public
list or resume API.

## Closed public contract

```text
ClosureRootV1 =
  SourceRevision { source_revision_id }
  | SourceBucket { source_id }

ClosureCauseV1 = superseded | soft_deleted | purged | source_erased
ClosurePhaseV1 = proving | at_rest_pending | complete | incomplete

ClosureLookupV1 { schema_version: 1, closure_operation_id }
ClosureStatusV1 {
  schema_version: 1,
  closure_operation_id,
  root: ClosureRootV1,
  cause: ClosureCauseV1,
  phase: ClosurePhaseV1,
  effective_at_epoch_s,
  admitted_write_boundary,
  admitted_dependency_generation,
  affected_count,
  blocker_code?,
  proof?
}
ClosureProofV1 {
  schema_version: 1,
  proof_write_boundary,
  current_active_dependent_nodes: 0,
  current_derived_edges: 0,
  view_eligible_dependents: 0,
  ownerless_projection_rows: 0,
  post_admission_registrations: 0,
  remaining_dependency_rows?,
  remaining_canonical_rows?,
  remaining_projection_rows?,
  remaining_receipt_reference_rows?
}

Engine::read_dependency_closure(ClosureLookupV1)
  -> Result<Option<ClosureStatusV1>, EngineError>
```

Closure operation IDs are Engine-minted `_fdb:c:<64-lower-hex>` identifiers.
Admission increments a checked, monotonic `_fathomdb_closure_sequence` in the
same transaction and hashes the domain
`fathomdb.dependency-closure.v1\0`, sequence, root kind/value, cause, effective
instant, and admitted boundaries. The ID contains no source text and cannot
collide when a record is deleted, reactivated, and deleted again at the same
write boundary.

`ClosureLookupV1::new(id)` validates the Engine-ID grammar and fixes schema
version 1. `ClosureOperationId` exposes only `as_str`; callers cannot mint one
through the caller-ID grammar. `read_dependency_closure` is a pure point read
and returns `None` for an absent well-formed ID. It is not a recovery or browse
surface. Slice 25 actuation receipts are the only public producer of a closure
ID in 0.8.25.

`DependencyClosureError { reason, field_path }` has the closed reasons
`unsupported_schema_version`, `unknown_field`, and
`closure_operation_id_invalid`. Rust constructors return it directly;
`EngineError::DependencyClosure` wraps it at the Engine boundary. Python raises
`DependencyClosureError(reason, field_path)` and TypeScript throws the same
class with code `FDB_DEPENDENCY_CLOSURE`. Canonical RFC 6901 paths use
`/schemaVersion` then `/closureOperationId`.

Registration/reactivation uses existing `DependencyError` with two additive
reasons: `dependency_source_ineligible` and `dependency_closure_active`.
Derived writes use matching additive `ProvenanceError` reasons
`source_revision_ineligible` and `source_closure_active` at
`/provenance/sourceRevisionId`. Actuation maps them to additive receipt reasons
`dependency_source_ineligible` and `dependency_closure_active` at the nested
source-revision path. Persisted corruption remains `EngineError::Storage`. Unknown request fields are
checked lexicographically after the schema discriminator and before the
required ID. Older clients may ignore additive status/proof fields but reject
unknown root/cause/phase/error variants.

Rust boundaries/counts/times are `u64` except `effective_at_epoch_s: i64`.
Python and TypeScript serialize nonnegative integers as canonical unsigned
decimal strings and the signed epoch as a canonical signed decimal string.
Optional response fields are always present and null when absent. Root is a
closed tagged object: snake-case Python and camel-case TypeScript outer fields,
with lower-snake-case discriminants in both.

## Persistence and invariants

Schema step 30 adds:

```sql
INSERT INTO _fathomdb_open_state(key, value)
  VALUES('_fathomdb_closure_sequence', '0');

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
  effective_at_epoch_s INTEGER NOT NULL,
  admitted_write_boundary INTEGER NOT NULL CHECK(admitted_write_boundary >= 0),
  admitted_dependency_generation INTEGER NOT NULL
    CHECK(admitted_dependency_generation >= 0),
  closure_sequence INTEGER NOT NULL UNIQUE CHECK(closure_sequence > 0),
  retry_fingerprint TEXT NOT NULL CHECK(
    length(retry_fingerprint) = 64 AND
    retry_fingerprint = lower(retry_fingerprint) AND
    retry_fingerprint NOT GLOB '*[^0-9a-f]*'
  ),
  phase TEXT NOT NULL CHECK(phase IN (
    'proving','at_rest_pending','complete','incomplete'
  )),
  affected_count INTEGER NOT NULL CHECK(affected_count > 0),
  blocker_code TEXT CHECK(blocker_code IN (
    'projection_state_unavailable','proof_unavailable',
    'telemetry_redaction','wal_checkpoint'
  )),
  structural_proof_write_boundary INTEGER
    CHECK(structural_proof_write_boundary >= 0),
  proof_json TEXT CHECK(proof_json IS NULL OR json_valid(proof_json)),
  CHECK(
    (phase = 'complete' AND blocker_code IS NULL AND
      structural_proof_write_boundary IS NOT NULL AND proof_json IS NOT NULL)
    OR
    (phase = 'incomplete' AND blocker_code IS NOT NULL AND
      ((cause IN ('purged','source_erased') AND
        structural_proof_write_boundary IS NOT NULL AND proof_json IS NOT NULL)
       OR
       (cause IN ('superseded','soft_deleted') AND
        structural_proof_write_boundary IS NULL AND proof_json IS NULL)))
    OR
    (phase = 'proving' AND blocker_code IS NULL AND
      structural_proof_write_boundary IS NULL AND proof_json IS NULL)
    OR
    (phase = 'at_rest_pending' AND blocker_code IS NULL AND
      cause IN ('purged','source_erased') AND
      structural_proof_write_boundary IS NOT NULL AND proof_json IS NOT NULL)
  )
);
CREATE INDEX _fathomdb_dependency_closures_root
  ON _fathomdb_dependency_closures(root_kind, root_value, phase);
CREATE INDEX _fathomdb_dependency_closures_recovery
  ON _fathomdb_dependency_closures(phase, closure_sequence);
CREATE UNIQUE INDEX _fathomdb_dependency_closures_active_retry
  ON _fathomdb_dependency_closures(retry_fingerprint)
  WHERE phase != 'complete';
```

The table stores opaque/non-PII identifiers, a one-way retry fingerprint, and
scalar counts only—never body, locator, logical ID, request, dependency ID, or
derived revision ID. The retry fingerprint is SHA-256 over the domain
`fathomdb.dependency-closure-retry.v1\0`, normalized originating verb, and
normalized root argument. It locates the post-purge result after the logical
row is gone; it is not an authorization token or evidence identifier.
Only a nonterminal physical operation owns the fingerprint uniquely. Completion
releases that uniqueness, so delete/recreate/delete and
erase/repopulate/erase each admit a new closure with a new sequence and ID.
`proof_json` is emitted and parsed in one canonical key order, contains exactly
the `ClosureProofV1` scalar fields, and must agree with its indexed boundary.
Every point read validates ID hash shape, root grammar, sequence singleton,
closed variants, phase constraints, proof JSON, count conversions, and
`structural_proof_write_boundary <=` current write boundary. Any disagreement
is `Storage`.

The sequence singleton reuses Slice 20's fail-closed generation rules: the
stored value is canonical nonnegative decimal with no leading zero except
`0`, fits `i64`, and is at least `MAX(closure_sequence)`. Open rejects a
missing, malformed, out-of-range, or regressed singleton. Admission performs a
checked exhaustion test before any mutation; `i64::MAX` is exhausted. A
rollback or a no-dependent no-op does not advance the sequence.

## Root admission and atomic consequences

Only source revisions with at least one Slice 20 registration create a closure.
One `BEGIN IMMEDIATE` transaction fixes a single epoch-second instant, write
boundary, dependency generation, and checked closure sequence. It validates
every dependency/source-link/revision chain, inserts the closure row, applies
all direct effects with set-based/indexed operations, and applies the root
mutation. Total direct impact has no refusal cap.

The fixed consequences are:

| Root cause | Derived node | Derived edge |
| --- | --- | --- |
| `superseded` / `soft_deleted` | If the exact dependent revision is current and active, set it to `deleted`; retain provenance and dependency audit. | If the exact dependent revision is current, set `superseded_at` to the root transaction's resulting write boundary; retain provenance and dependency audit. |
| `purged` / `source_erased` | Physically erase the exact dependent revision, row-owned projections, source links/revision identity, receipt references, and graph rows whose integrity depends on its logical endpoint. | Physically erase the exact edge revision, row-owned projections, source links/revision identity, and receipt references. |

Soft closure also removes attribute/property projections without an independent
lifecycle filter. Other owner-valid FTS/vector shadows may remain because
canonical hydration excludes them. Physical effects delete every registered
row-owned projection and any pending projection row. A missing soft target is
allowed only when the validated exact revision is already non-current or
inactive. A physical target must exist when enumerated; its deletion and
dependency removal share the root transaction.

Source erasure already selects all rows in the source bucket. Same-bucket
dependent rows are counted and classified as physically completed before their
rows disappear; there is no later per-dependent work queue. It also deletes
prior completed source-revision closure rows whose roots belong to the erased
bucket. The current source-bucket closure row and ordinary erasure-audit row are
the sole retained content-free event records. Purge removes prior completed
closure rows rooted in each physically removed source revision.

Physical dependency removal advances the dependency generation exactly once in
the root transaction. Soft closure retains immutable registrations and does not
advance it. Root admission, every effect, closure phase, root mutation,
actuation receipt, cursor publication, and dependency generation either all
commit or all roll back.

Before a physical root transaction commits, it evaluates the complete affected
set from the pre-delete plan and stores the canonical structural zero proof and
its write boundary on the `at_rest_pending` row in that same destructive
transaction. The proof covers every affected dependency, exact canonical row,
row-owned projection, source link/revision, actuation receipt reference, and
pending projection row. Delete and proof therefore survive or roll back
together; restart never reconstructs erased member identities from a count.

Ordinary `write`/supersession and `transition` complete their soft proof in this
same transaction, preserving existing return and exact retry semantics. Purge,
`erase_source`, and operator `excise_source` commit physical proof inputs and
phase `at_rest_pending`, then reuse existing telemetry/WAL completion before
reporting success. They preserve the existing `ErasureIncomplete` error and
idempotent retry contract; no new post-commit `WriteReceipt` behavior exists.

Actuation performs the same effects in its existing idempotent transaction but
commits phase `proving`, outcome `committed_closure_pending`, and ordered
closure IDs. Immediately after commit it attempts internal proof finalization;
the immutable receipt still describes the state at its commit. Exact replay
returns that receipt, and keyed status reports current closure state.

## Internal recovery and proof

`Engine::open` validates closure rows, sequence state, and active barriers, but
does not attempt external telemetry or WAL work and does not fail merely
because a valid physical closure is pending. The pre-writer maintenance seam
finalizes nonphysical `proving` rows and retries nonphysical `incomplete`
rows, in `closure_sequence` order and batches of at most 32. It clears a soft
blocker only in the transaction that commits a valid proof. Exact actuation
replay invokes the same bounded maintenance before returning the immutable
receipt. There is no public resume/list administration.

While a physical row is nonterminal, `enable_telemetry`, close, keyed closure
status reads, and ordinary reads under the barrier remain allowed. The exact
originating `purge`, `erase_source`, or operator `excise_source` retry is also
allowed: purge matches its retry fingerprint after the logical row is gone,
and source erasure matches the source-bucket root. Every other writer returns
the existing `ErasureIncomplete { stage: "dependency_closure" }`. Exact retry
does not reapply the root mutation; it validates the committed structural
proof, discharges telemetry redaction and WAL checkpointing, and finalizes the
closure. A crash before the root commit leaves no closure; a crash afterward
leaves a barrier and a self-contained at-rest result.

Soft proof requires zero current active dependent nodes, zero current derived
edges, zero default-view eligible dependents, zero ownerless projection rows,
and zero registrations newer than the admitted generation. Owner-valid raw
FTS/vector rows do not fail soft proof when no governed read can return them.

Physical proof's structural zeros are already stored atomically with deletion.
At-rest retry validates the canonical proof, indexed boundary, unchanged active
barrier, absence of later canonical/dependency writes, and that the root bucket
or revision remains empty. It does not reconstruct or query erased member IDs.
Only then does it complete existing telemetry redaction and
`wal_checkpoint(TRUNCATE)` before marking complete. Updating the content-free
phase after that checkpoint may create a new WAL frame, but cannot reintroduce
erased bytes.

Proof success and phase `complete` commit together. A checkable operational
failure records `incomplete` plus one closed blocker; unclassifiable storage
corruption returns `Storage` and leaves the prior nonterminal row unchanged.
Internal maintenance rechecks a nonphysical `incomplete` operation; an exact
originating retry rechecks a physical `incomplete` operation. Each clears its
blocker only when proof and any external obligations succeed. No empty queue or
affected count is itself proof.

## Eligibility, historical views, and projection races

A nonterminal closure barrier is unconditional: registered dependents matching
its source revision or source bucket are excluded before canonical, FTS,
vector, property, graph seed/frontier, or expansion truncation under every
`ReadView`, including historical relaxations.

Outside an active barrier, dependency source eligibility is evaluated under
the same frozen `ReadView` and resolved instant as the derived candidate. The
default therefore requires the pinned source node revision to be current,
active, and in-window. `include_superseded`, `include_inactive`, and
`include_out_of_window` each relax the same source axis they already relax on
the derived row; they do not bypass erasure or a nonterminal barrier. Search
continues to reject its existing unsupported existence relaxations. Slice 35
extends predicate eligibility without changing this rule.

Registration, explicit reactivation, and every ordinary or actuation-derived
write carrying a source revision are not historical reads: they always require
the pinned source revision to exist with complete provenance and be current,
active, in-window at one fixed current epoch second, and unfenced.
Registration/reactivation return `dependency_source_ineligible` or
`dependency_closure_active`; ordinary writes return
`source_revision_ineligible` or `source_closure_active`; actuation maps those to
the matching dependency receipt reasons at the nested source-revision path.
Validation order is existing provenance shape/hash/role, active closure, source
eligibility, dependency conflict, then lifecycle legality, so malformed
provenance cannot probe fence state while a structurally valid fenced request
gets the distinct closure-active reason. `valid_from` never auto-reactivates;
scheduled `valid_until` propagation is 0.8.26 work.

Projection publication revalidates the canonical owner, dependency source under
the strict current view, and active closure barrier after acquiring its
`BEGIN IMMEDIATE` transaction and atomically with the projection insert. An
ineligible item is terminalized without publishing or has its stale row removed.
This closes both worker-before-admission and admission-before-worker
interleavings without freezing ordinary soft writes. Existing physical erasure
keeps its drain/freeze ordering.

## Compatibility, performance, and verification

No-dependency writes/lifecycle/erasure retain their existing return shapes and
avoid creating closure rows. Read paths may bypass the dependency anti-join
only from an Engine-session cached zero-dependency/zero-nonterminal state that
invalidates on every relevant commit. Slice 75 measures fast-path and
dependent-path overhead.

RED is committed before product code and covers node/edge soft and physical
effects; exact root/actuation atomicity; registration/reactivation eligibility;
ordinary and actuation derived-write admission, typed mapping, and precedence;
barriers under strict and relaxed views; both projection publication races;
crash/reopen at proving and at-rest phases; exact receipt replay; phase/proof
corruption; source erasure of prior closure rows; raw database/WAL canaries;
closure-sequence missing/malformed/regressed/exhausted/rollback/no-op cases;
repeat delete/recreate/delete and erase/repopulate/erase admission; nonphysical
incomplete recovery and exact actuation replay maintenance; reachable
closure-active versus ineligible precedence;
schema/property invariants; strict binding codecs; and existing no-dependent
compatibility.

GREEN runs focused schema/Engine/binding tests, fast verification, heavy
erasure/concurrency tests, all/all-feature/operator routes, installed wheel and
N-API smokes, and Windows CPU/native Rust/Python/Node routes. CUDA, live models,
and network access are N/A. The strict ptrace-capable gate runs unchanged
outside the sandbox if needed.

## Forward obligations

- 0.8.26 extends closure to multi-source, bounded recursive dependencies, and
  scheduled validity loss.
- Slice 35 binds optional frozen reads and richer eligibility to dependency and
  active-closure state.
- Slice 40 replaces cursor-only projection correlation with generation-aware
  readiness.
- Slice 50 evidence resolution rechecks the unconditional barrier.
- Slice 55 diagnoses closure/proof corruption without semantic repair.
- Slice 60 applies the guard before graph seed/frontier truncation.
- Slice 75 verifies installed parity and representative lifecycle performance.
