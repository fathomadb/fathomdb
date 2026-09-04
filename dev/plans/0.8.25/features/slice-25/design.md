---
title: 0.8.25 Slice 25 — bounded atomic actuation design
status: BLOCKED_DESIGN_FIX_CAP
design_version: 7
review_fix: 5
depends_on: 20
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
decision: dev/adr/ADR-0.8.25-bounded-atomic-actuation.md
---

# Slice 25 design

## Authority and disposition

This design implements S25-R1 through S25-R7, the retained core of
R25/AC25-25, N25-01, Memex needs 3/17 and the compact part of 18, and A25-05.
It is a successor to the single-operation composition assumptions in the
accepted typed-write and `PreparedWrite` ADRs; it preserves their typed/no-SQL
boundary and the existing `Engine::write` API. The additive public method is
governed by
[`ADR-0.8.25-bounded-atomic-actuation`](../../../../adr/ADR-0.8.25-bounded-atomic-actuation.md).

The implementation reuses `ProvenancedNodeV1`,
`SourceDependencyRegistrationV1`, the existing lifecycle state machine, and
the single SQLite writer. It creates no second record, provenance, dependency,
or lifecycle authority. Experimental semantic-operation journal designs are
historical evidence only; Slice 25 stores terminal receipts, not prepared
requests or nonterminal public state.

## Exact public contract

Rust exposes an additive `Engine::actuate` method. Python exposes `actuate`;
TypeScript exposes `actuate`. Each binding accepts its normal snake/camel wire
shape and returns the equivalent typed receipt.

```text
ActuationOperationV1 =
  PutCanonicalNode { type: "put_canonical_node", record: ProvenancedNodeV1 }
  | PutDerivedNode { type: "put_derived_node", record: ProvenancedNodeV1 }
  | RegisterSourceDependency { type: "register_source_dependency",
      dependency: SourceDependencyRegistrationV1
    }
  | TransitionLifecycle { type: "transition_lifecycle",
      logical_id,
      expected_current_revision_id,
      to_state: active | deleted,
      reason?
    }

ActuationBatchV1 {
  schema_version: 1,
  operation_id,
  decision_policy_id?,
  expected_write_boundary?,
  operations: [1..128]
}

ActuationReceiptV1 {
  schema_version: 1,
  operation_id,
  request_sha256,
  outcome: committed | committed_closure_pending | refused,
  refused_operation_index?,
  refused_field_path?,
  reason_codes,
  affected_revision_ids,
  resulting_write_boundary?,
  resulting_dependency_generation?,
  pending_projection_write_cursors,
  closure_operation_ids
}

Engine::actuate(
  request: ActuationBatchV1
) -> Result<ActuationReceiptV1, EngineError>
```

`committed_closure_pending` and `closure_operation_ids` are reserved additive
contract points for Slice 30 and are never emitted/populated by Slice 25.
Unknown request fields and variants reject before execution. Unknown response
fields may be ignored; an unknown outcome or reason code rejects. Rust uses
`u64` internally for boundaries/generations/cursors; Python and TypeScript use
canonical unsigned decimal strings for all three, matching Slice 20.

Rust exports `ActuationBatchV1`, `ActuationOperationV1`,
`LifecycleActuationV1`, `ActuationReceiptV1`, `ActuationOutcomeV1`,
`ActuationRefusalReasonV1`, `ActuationErrorReason`, and `ActuationError`.
The exact constructors are:

```text
ActuationBatchV1::new(
  operation_id: impl Into<String>,
  operations: Vec<ActuationOperationV1>
) -> Result<ActuationBatchV1, ActuationError>

ActuationBatchV1::with_decision_policy_id(
  self, decision_policy_id: impl Into<String>
) -> Result<ActuationBatchV1, ActuationError>

ActuationBatchV1::with_expected_write_boundary(
  self, boundary: u64
) -> ActuationBatchV1

LifecycleActuationV1::new(
  logical_id: impl Into<String>,
  expected_current_revision_id: ArtifactRevisionId,
  to_state: LifecycleState,
  reason: Option<String>
) -> Result<LifecycleActuationV1, ActuationError>
```

The batch constructor validates operation ID/count and defaults schema version
1 plus absent policy/boundary. Its named setters validate only their new field.

The Rust enum uses tuple variants
`PutCanonicalNode(ProvenancedNodeV1)`,
`PutDerivedNode(ProvenancedNodeV1)`,
`RegisterSourceDependency(SourceDependencyRegistrationV1)`, and
`TransitionLifecycle(LifecycleActuationV1)`. `LifecycleActuationV1` contains
`logical_id: String`,
`expected_current_revision_id: ArtifactRevisionId`,
`to_state: LifecycleState`, and `reason: Option<String>`.
`LifecycleActuationV1::new` validates the logical-ID address space and that
`to_state` is `Active` or `Deleted`. The typed `ArtifactRevisionId` has already
validated its grammar; legality from persisted state is database-dependent.

Python request mappings use `schema_version`, `operation_id`,
`decision_policy_id`, `expected_write_boundary`, and `operations`; operation
discriminants are `put_canonical_node`, `put_derived_node`,
`register_source_dependency`, and `transition_lifecycle`. Response attributes
use the receipt's snake-case names. TypeScript uses `schemaVersion`,
`operationId`, `decisionPolicyId`, `expectedWriteBoundary`, and camel-case
receipt fields, with the same lower-snake-case discriminants. Optional request
keys may be absent or null; optional response keys are always present and null
when absent. `refused_operation_index` is zero-based.

`reason_codes` is empty for either committed outcome and has exactly one member
for `refused`. Python raises `ActuationError(reason, field_path)`; TypeScript
throws `ActuationError` with code `FDB_ACTUATION`, `reason`, and `fieldPath`.
Rust adds `EngineError::Actuation(ActuationError)`. Actuation-local request,
idempotency, and receipt-state failures use that variant. Existing nested
provenance, dependency, storage, closing, and infrastructure ownership remains
unchanged outside actuation; inside an admitted batch, nested domain failures
become a refused receipt while `Storage`/`Closing` remain root errors.

`ActuationErrorReason` is the closed set `unsupported_schema_version`,
`unknown_field`, `unknown_operation_variant`, `field_missing`,
`field_type_invalid`, `operation_id_invalid`, `decision_policy_id_invalid`,
`operation_count_invalid`, `logical_id_invalid`, `revision_id_invalid`,
`lifecycle_target_invalid`, `nested_request_invalid`,
`operation_id_conflict`, and `operation_id_erased`.
It exposes the lower-snake-case stable reason and RFC 6901 field path;
persisted-state reasons use `/operationId`. A malformed persisted receipt or
source-ref row is `EngineError::Storage`, preserving the accepted storage-
corruption ownership; there is no `receipt_corrupt` actuation reason.

Strict parsing order is schema discriminator; lexicographically smallest
unknown canonical camel-case field; required field/type in the top-level order
shown above; ID grammar; operation count; then each operation in array order.
Within an operation, validation follows the displayed field order and prefixes
nested paths with `/operations/{zero-based-index}`. Both bindings report these
canonical camel-case RFC 6901 paths even though Python input keys are
snake-case. The shared fixtures pin all precedence cases.

`operation_id` and `decision_policy_id` use the Slice 15 caller-ID grammar:
1–128 ASCII bytes, leading alphanumeric, remaining `[A-Za-z0-9._:-]`, and no
`_fdb:` prefix. They are opaque identifiers and callers must not place source
text or personal data in them.

## Operation semantics

`PutCanonicalNode` requires a complete canonical `ProvenancedNodeV1`.
`PutDerivedNode` requires a complete derived `ProvenancedNodeV1`. The explicit
variant must agree with the record's provenance role. Existing write
validation, record-revision identity, source locator/hash checks, and
same-logical-ID supersession semantics remain authoritative.

`RegisterSourceDependency` reuses Slice 20 validation. Its source and derived
revisions must already exist or have been created by an earlier operation in
this batch. It cannot point at a later put.

`TransitionLifecycle` addresses the existing logical ID and requires the
current immutable revision ID observed by the caller. All four existing moves
are admitted: `pending -> active`, `pending -> deleted`, `active -> deleted`,
and `deleted -> active`. `pending` and `purged` are not transition targets, and
nonexistent semantic states such as `invalidated` are never accepted.
Purge/erasure remain existing operator operations. A later put with the same
logical ID uses existing write-time supersession rather than an invented
lifecycle transition of an old revision.

Operations are interpreted in array order against one transaction-local
prospective state. References may name persisted state or earlier results.
No category regrouping or forward-reference resolution occurs. A repeated
identical dependency registration remains Slice 20's no-op success. Two puts
that violate existing batch-write uniqueness rules refuse the entire request.

## Dependency-stage lifecycle rule

Slice 25 cannot create Slice 30 barriers or closure intents. If a canonical
source revision has one or more registered Slice 20 dependents, either of these
attempts receives terminal refusal `dependency_closure_required`:

- a put that would supersede that source revision; or
- `active -> deleted` or `pending -> deleted` for that source logical ID.

After ordered structural validation, a final prospective closure-safety pass
compares every source revision made non-current/non-live by the request with
both persisted dependencies and every dependency registration that would be
successful in the prospective state. Any match refuses the whole request,
regardless of operation order. A dependency attached to the new replacement
revision is not a dependent of the superseded source and does not trigger the
guard. If the source-loss and dependency operations have different positions,
the refusal names the earliest source-loss operation index and its operation-
root path. This deliberate dependency-stage gate closes the visibility gap
without leaking future schema backward. Slice 30 will supersede this rule when
it can admit and fence closure atomically.

## Validation and refusal precedence

Error ownership has four non-overlapping layers:

1. Rust constructors return `ActuationError` directly. Batch construction uses
   `operation_id_invalid` at `/operationId` or `operation_count_invalid` at
   `/operations`; its policy setter uses `decision_policy_id_invalid` at
   `/decisionPolicyId`. Lifecycle construction uses `logical_id_invalid` at
   `/logicalId` or `lifecycle_target_invalid` at `/toState`. Its revision is
   already typed and cannot be malformed.
2. Python/TypeScript decode closed outer fields to their matching
   `ActuationError`. Invalid raw lifecycle revisions use
   `revision_id_invalid`; invalid logical IDs/targets use the constructor
   reasons. Malformed nested node/provenance/dependency mappings are wrapped as
   `nested_request_invalid` while preserving the nested canonical pointer,
   prefixed by `/operations/{i}/record` or `/operations/{i}/dependency`. No
   Engine call and no receipt occurs.
3. `Engine::actuate` wraps only keyed `operation_id_conflict` and
   `operation_id_erased` as `EngineError::Actuation`. Corrupt receipt/reference
   storage and infrastructure retain `EngineError::Storage` or their existing
   family.
4. A syntactically valid, newly admitted request returns a terminal refused
   receipt for database-dependent domain failure according to the mapping
   below; it is not an exception.

A closed, syntactically valid request admitted under a new operation ID can
produce a persisted terminal `refused` receipt. The first failure in this
order wins and supplies one reason code, operation index when applicable, and
RFC 6901 field path when applicable:

1. expected global write-boundary mismatch;
2. operation order: nested write/provenance validation, role mismatch,
   missing/forward reference, dependency validation, lifecycle validation;
3. dependency closure required;
4. checked write-cursor or dependency-generation exhaustion.

Existing validation maps deterministically:

| Existing failure | Refusal reason | Path/index |
| --- | --- | --- |
| `WriteValidation` or `SchemaValidation` while validating a put | `write_refused` | put index; `/operations/{i}/record` |
| provenance `role_invalid` or explicit variant/role disagreement | `provenance_role_mismatch` | put index; `/operations/{i}/record/provenance/role` |
| provenance `source_revision_missing` | `reference_unavailable` | put index; `/operations/{i}/record/provenance/sourceRevisionId` |
| unavailable persisted/earlier-batch operation reference | `reference_unavailable` | referring index; `/operations/{i}/record/provenance/sourceRevisionId`, `/operations/{i}/dependency/sourceRevisionId`, `/operations/{i}/dependency/derivedRevisionId`, or `/operations/{i}/expectedCurrentRevisionId` |
| any other `ProvenanceError(reason, path)` | `write_refused` | put index; nested path prefixed by `/operations/{i}/record` |
| `DependencyErrorReason::DependencyGenerationExhausted` | `dependency_generation_exhausted` | first dependency operation that would insert; `/operations/{i}/dependency` |
| any other `DependencyError(reason, path)` | `dependency_refused` | dependency index; nested path prefixed by `/operations/{i}/dependency` |
| missing/current-revision mismatch or `NotLifecycleAddressable` | `lifecycle_refused` | transition index; `/operations/{i}/expectedCurrentRevisionId` or `/operations/{i}/logicalId` |
| `IllegalTransition` | `lifecycle_refused` | transition index; `/operations/{i}/toState` |
| prospective source-loss/dependency match | `dependency_closure_required` | earliest source-loss index; `/operations/{i}` |
| expected boundary mismatch | `expected_write_boundary_mismatch` | no operation index; `/expectedWriteBoundary` |
| checked global cursor exhaustion | `write_cursor_exhausted` | first put index; `/operations/{i}/record` |

`Storage`, `Closing`, lock, projection, scheduler, and injected infrastructure
failures never become receipt reasons.

The closed Slice 25 refusal vocabulary is
`expected_write_boundary_mismatch`, `write_refused`,
`provenance_role_mismatch`, `reference_unavailable`, `dependency_refused`,
`lifecycle_refused`, `dependency_closure_required`,
`write_cursor_exhausted`, and `dependency_generation_exhausted`. The stable
reason code and optional path/index are the complete refusal diagnostic; there
is no unlisted detail field. A refused receipt has no affected IDs, resulting
boundary/generation, projection cursors, or closure IDs.

SQLite I/O, commit, lock, and injected infrastructure failures retain their
existing `EngineError` family, roll back, and store no receipt. They are not domain
refusals. This distinction avoids recording a result that did not commit.

## Canonical request digest

The Engine computes SHA-256 over a domain-separated, length-prefixed binary
encoding of the normalized typed request. `schema_version` and `operation_id`
are both included. Maps and JSON serialization are never inputs.

| Value | Encoding |
| --- | --- |
| Domain | exact bytes `fathomdb.actuation.v1\0` |
| `u32`, `u64` | fixed-width unsigned big-endian |
| `i64` | fixed-width two's-complement big-endian |
| string | `u64` UTF-8 byte length, then exact bytes |
| optional | `00` absent; `01` followed by value present |
| array | `u32` item count, then items in order |
| enum | one assigned byte below |

All tags below are hexadecimal bytes. The top-level order/tags are
`01 schema_version:u32`,
`02 operation_id:string`, `03 decision_policy_id:optional string`,
`04 expected_write_boundary:optional u64`, and `05 operations:array`.
Operation tags are `10 put_canonical_node`, `11 put_derived_node`,
`12 register_source_dependency`, and `13 transition_lifecycle`.

Both put operations encode these node field tags in order:
`20 kind:string`, `21 body:string`, `22 source_id:string`,
`23 logical_id:optional string`, `24 initial_state` (`00 pending`, `01 active`),
`25 reason:optional string`, `26 valid_from:optional i64`,
`27 valid_until:optional i64`, and `28 provenance`. Provenance fields are
`30 schema_version:u32`, `31 role` (`00 canonical`, `01 derived`),
`32 artifact_revision_id:string`, `33 source_version_id:string`,
`34 source_revision_id:optional string`, `35 locator:optional`, and
`36 canonical_hash:optional string`. Locator is `00 whole_body` or
`01 utf8_bytes` followed by start/end `u64` values. SHA-256 is fixed, so the
canonical hash encodes only its lowercase digest.

Dependency fields are `40 schema_version:u32`, `41 dependency_id:string`,
`42 source_revision_id:string`, and `43 derived_revision_id:string`.
Lifecycle fields are `50 logical_id:string`,
`51 expected_current_revision_id:string`, `52 to_state`
(`00 active`, `01 deleted`), and `53 reason:optional string`.

One required shared fixture contains a Unicode body, a negative `valid_from`,
present and absent optionals, both provenance roles, a byte locator,
dependency registration, and lifecycle transition. It pins the complete
request, canonical-byte hex, and lowercase 64-character digest. The Engine
always computes the digest; callers cannot supply it. Digesting streams through
fields so it does not duplicate a whole body in memory.

## Persistence and erasure

Schema step 29 creates:

```text
CREATE TABLE _fathomdb_actuation_receipts(
  operation_id TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL CHECK(schema_version = 1),
  request_sha256 TEXT,
  operations_count INTEGER,
  outcome TEXT NOT NULL,
  refused_operation_index INTEGER,
  refused_field_path TEXT,
  reason_codes_json TEXT NOT NULL,
  affected_revision_ids_json TEXT NOT NULL,
  resulting_write_boundary INTEGER,
  resulting_dependency_generation INTEGER,
  pending_projection_write_cursors_json TEXT NOT NULL,
  closure_operation_ids_json TEXT NOT NULL,
  CHECK(outcome IN (
    'committed','committed_closure_pending','refused','erased'
  )),
  CHECK(
    (outcome = 'erased' AND request_sha256 IS NULL) OR
    (outcome != 'erased' AND request_sha256 IS NOT NULL AND
      length(request_sha256) = 64 AND
      request_sha256 NOT GLOB '*[^0-9a-f]*')
  ),
  CHECK(
    (outcome = 'erased' AND operations_count IS NULL) OR
    (outcome != 'erased' AND operations_count BETWEEN 1 AND 128)
  ),
  CHECK(json_valid(reason_codes_json) AND
    json_type(reason_codes_json) = 'array'),
  CHECK(json_valid(affected_revision_ids_json) AND
    json_type(affected_revision_ids_json) = 'array'),
  CHECK(json_valid(pending_projection_write_cursors_json) AND
    json_type(pending_projection_write_cursors_json) = 'array'),
  CHECK(json_valid(closure_operation_ids_json) AND
    json_type(closure_operation_ids_json) = 'array'),
  CHECK(json_array_length(affected_revision_ids_json) <= 256),
  CHECK(json_array_length(pending_projection_write_cursors_json) <= 128),
  CHECK(json_array_length(closure_operation_ids_json) <= 128),
  CHECK(
    (outcome = 'refused' AND json_array_length(reason_codes_json) = 1 AND
      json_array_length(affected_revision_ids_json) = 0 AND
      json_array_length(pending_projection_write_cursors_json) = 0 AND
      json_array_length(closure_operation_ids_json) = 0 AND
      (refused_operation_index IS NULL OR
        (refused_operation_index >= 0 AND
          refused_operation_index < operations_count)) AND
      resulting_write_boundary IS NULL AND
      resulting_dependency_generation IS NULL) OR
    (outcome IN ('committed','committed_closure_pending') AND
      json_array_length(reason_codes_json) = 0 AND
      refused_operation_index IS NULL AND refused_field_path IS NULL AND
      resulting_write_boundary IS NOT NULL AND
      resulting_write_boundary >= 0 AND
      (resulting_dependency_generation IS NULL OR
        resulting_dependency_generation > 0) AND
      ((outcome = 'committed' AND
          json_array_length(closure_operation_ids_json) = 0) OR
        (outcome = 'committed_closure_pending' AND
          json_array_length(closure_operation_ids_json) BETWEEN 1 AND 128))) OR
    (outcome = 'erased' AND
      request_sha256 IS NULL AND operations_count IS NULL AND
      refused_operation_index IS NULL AND
      refused_field_path IS NULL AND
      json_array_length(reason_codes_json) = 0 AND
      json_array_length(affected_revision_ids_json) = 0 AND
      resulting_write_boundary IS NULL AND
      resulting_dependency_generation IS NULL AND
      json_array_length(pending_projection_write_cursors_json) = 0 AND
      json_array_length(closure_operation_ids_json) = 0)
  )
);

CREATE TABLE _fathomdb_actuation_receipt_source_refs(
  operation_id TEXT NOT NULL,
  schema_version INTEGER NOT NULL CHECK(schema_version = 1),
  ref_kind TEXT NOT NULL CHECK(ref_kind IN (
    'source_id','source_revision_id','artifact_revision_id'
  )),
  ref_value TEXT NOT NULL,
  PRIMARY KEY(operation_id, ref_kind, ref_value)
);
CREATE INDEX _fathomdb_actuation_receipt_refs_reverse
  ON _fathomdb_actuation_receipt_source_refs(
    ref_kind, ref_value, operation_id
  );
CREATE TRIGGER _fathomdb_actuation_ref_owner_before_insert
  BEFORE INSERT ON _fathomdb_actuation_receipt_source_refs
  WHEN NOT EXISTS (
    SELECT 1 FROM _fathomdb_actuation_receipts
    WHERE operation_id = NEW.operation_id AND outcome != 'erased'
  )
  BEGIN
    SELECT RAISE(ABORT, 'actuation receipt reference owner invalid');
  END;
```

Only terminal data is stored. Receipt JSON arrays are canonical compact JSON,
bounded by the request limit, and validated lazily on keyed replay; open never
scans the table without a key. Application validation additionally requires
canonical array member types/order and lowercase digest form; SQL checks
structure and outcome/column consistency. Domain rows remain the authority.
The receipt table is an idempotency/audit record, not a recovery journal.

Canonical persisted arrays use UTF-8 JSON with no insignificant whitespace.
Reason codes and revision/closure IDs are JSON strings. Projection cursors are
canonical unsigned decimal JSON strings, ordered ascending. Affected revision
IDs and closure IDs preserve first-effect order with duplicates removed. On
keyed replay the Engine parses, validates bounds/types/order, re-encodes, and
byte-compares every array; any mismatch is `EngineError::Storage`. RED corrupts
each forbidden erased/refused/result field and each array form independently.

For a refused row, keyed validation also requires its sole reason to be in the
closed refusal vocabulary and checks its scalar relationship against the
mapping table. `expected_write_boundary_mismatch` alone has null operation
index and exact path `/expectedWriteBoundary`. Every other reason has an index
in `0..operations_count`, and its path must be the exact value or canonical
descendant class in its mapping row: put/provenance under `/record`, dependency
under `/dependency`, lifecycle under one of its three named fields,
closure-required at the operation root, cursor exhaustion at `/record`, and
generation exhaustion at `/dependency`. Because the prepared request is not
stored, `operations_count` is persisted as a checked integer in the receipt
row solely to validate the terminal receipt; it is 1–128 for every non-erased
row and null after erasure. Unknown reasons, null/mismatched paths, or an index
outside that stored count return `EngineError::Storage`.

The exact Slice 25 collection bounds are:

| Collection | Maximum | Order |
| --- | ---: | --- |
| `reason_codes` | 1 | the deterministic first refusal only |
| `affected_revision_ids` | 256 | for each operation: newly put revision, then its implicitly superseded prior revision; a transition contributes its current revision; first occurrence wins |
| `pending_projection_write_cursors` | 128 | ascending numeric cursor, encoded as decimal strings |
| `closure_operation_ids` | 0 for `committed`/`refused`/`erased`; 1–128 for reserved `committed_closure_pending` | first-effect order |
| source-reference tuples | 1,024 | `(ref_kind, ref_value)` lexical order on persistence/read |

The maxima derive from 128 operations, at most two affected revisions per put,
at most one new projection cursor per put, and a deliberately conservative
eight source-reference tuples per operation. The collector fails closed before
receipt insertion if an internally derived set exceeds its formula.

One operation contributes at most eight distinct source-reference tuples;
therefore one 128-operation request has an exact cap of 1,024. Keyed replay
loads `LIMIT 1025` and fails `EngineError::Storage` if the cap is exceeded. It
validates every relevant row's schema version, closed kind, kind-specific
Slice 15 identity grammar, owning receipt existence/outcome, and uniqueness.
`source_id` uses the existing public or recognized Engine-owned source-ID
grammar; revision kinds use the landed stored-revision union. An erased receipt
must have zero source refs. The insert trigger prevents ordinary orphan or
erased-owner rows without relying on `PRAGMA foreign_keys`; the validator still
fails closed against raw corruption or a dropped/bypassed trigger.

Slice 25 does not tighten `SourceId`: public values remain valid exactly when
the landed constructor accepts them (not trim-empty and not `_`-prefixed),
including an embedded NUL. The reference table stores and compares such values
as SQLite parameters; it adds no SQL spelling constraint. A compatibility RED
covers whitespace-adjacent, Unicode, punctuation, long, and embedded-NUL
boundary-valid IDs through ordinary `write` and `actuate`.

The source-ref index records source IDs, source revision IDs, and artifact
revision IDs named, created, or resolved as affected by the request—including
lifecycle targets and revisions implicitly superseded by a put—without
retaining source text or locators. Refused requests index every valid-shaped
identity in the normalized request plus identities resolved before refusal.
Source erasure and purge collect target identities before domain deletion, use
the reverse index for indexed, target-complete `O(matches)` lookup, and change
every matching terminal row to an opaque `erased` tombstone: it nulls the
digest and optional diagnostic/result fields, replaces
all arrays with `[]`, and removes source-ref rows in the erasure transaction.
Deletion is explicit; correctness does not depend on SQLite foreign keys.
The operation ID remains reserved. Replay then returns typed
`operation_id_erased`; it cannot reveal old bytes, hashes, IDs, or outcomes and
cannot reuse the idempotency key for a different request. Redacting an entire
receipt when any indexed member is erased is permitted safe over-redaction.
Slice 25 does not claim constant or page-bounded work: one source may have
unbounded receipt references. An adversarial many-receipt test proves query-
plan index use and complete redaction. Chunking, fencing, continuation, and
resumable receipt redaction belong to Slice 30 if measurements justify them.
Raw-byte and WAL canaries prove removal across source erasure, lifecycle-target
receipts, refused multi-source batches, purge, and restart.

Replay validates the keyed receipt and its at-most-1,024 reference chain before
returning it. Purge and source erasure stream matching operation IDs through the
reverse index without materializing the unbounded target set, validate each
owning receipt and its bounded reference chain, and only then redact it. A
malformed relevant ref, orphan, erased-owner ref, or over-bound chain aborts and
rolls back the complete purge/erasure transaction; it is never skipped.

## Transaction and runtime flow

For a new request the Engine performs this sequence:

1. Strictly parse and normalize the request; compute its digest.
2. Keyed receipt lookup. An exact terminal match replays without draining
   projections. Conflict or erased tombstone returns an actuation error;
   corruption returns `EngineError::Storage`.
3. If the request contains any lifecycle operation, drain the existing
   projection worker using the same policy as `transition`; write/dependency-
   only requests do not add a drain. Acquire the writer mutex and begin one
   `IMMEDIATE` transaction.
4. Repeat the keyed receipt lookup inside the transaction to close races.
5. Read and compare the current global write cursor. This boundary is the
   existing maximum reserved record/operational cursor, not an actuation or
   dependency generation.
6. Validate the ordered prospective state using side-effect-free validators,
   then run the final prospective closure-safety pass across persisted and all
   prospective dependencies. On domain refusal, insert the receipt and
   source-ref index only, then commit.
7. Reserve checked record cursors for true writes and one dependency generation
   iff at least one dependency row will actually be inserted.
8. Apply operations in order through transaction-scoped record, dependency,
   lifecycle, and synchronous-projection helpers. Insert the terminal receipt
   and source refs in the same transaction.
9. Commit. Only after commit publish the in-memory next cursor and notify the
   projection worker once.

Existing `Engine::write`, `register_source_dependency`, and `transition`
become thin owners around the same validator/apply seams; their signatures and
observable behavior do not change. No helper starts a nested transaction,
publishes a cursor, emits duplicate events, or notifies a worker before the
enclosing owner commits.

The temporary dependency-closure refusal is guaranteed only for mutations
submitted through `actuate`. Slice 25 intentionally leaves standalone existing
methods behavior-compatible; Slice 30 closes those paths with shared barriers
and lifecycle closure.

A non-replay actuation emits one existing Writer `Started` event. A committed
receipt records one write operation with `write_rows` equal to the number of
put operations and emits one Writer `Finished` event. A terminal refusal is a
successfully completed typed outcome: it records one write operation with zero
write rows, emits one Writer `Finished` event, and does not change
`errors_by_code`. Exact terminal replay emits no mutation event and changes no
counters. Operation-ID conflict and erased-ID outcomes emit Writer `Started`,
record the stable `ActuationError` code, then emit Writer and Error `Failed`.
Receipt corruption does the same with `StorageError`; other infrastructure
errors retain their existing stable code/events. No nested helper emits or
counts the same actuation again.

`resulting_write_boundary` is the final global write cursor for committed
batches. A dependency- or lifecycle-only commit returns the unchanged current
boundary. `resulting_dependency_generation` is present only when at least one
new dependency was inserted. `pending_projection_write_cursors` contains, in
ascending order, cursors created by the request that lack a terminal row after
the synchronous apply; terminal/up-to-date cursors are omitted. Slice 40 may
add projection-generation identity but cannot reinterpret these fields.

`affected_revision_ids` contains every created revision and every existing
revision whose currency or lifecycle state changed, once, in first-effect
order. It contains no dependency IDs or logical IDs. A same-logical-ID put
therefore includes the newly created revision and the revision it superseded.

## Limits and compatibility

A request contains 1–128 operations. The four exact variants imply a bounded
reference count, so Slice 25 has no misleading independent 2,048-reference
limit. Existing record-body and SQLite limits remain authoritative; no smaller
actuation-only body cap is introduced. Receipt size is O(operations).

The additive method preserves the public typed/no-SQL boundary, recovery-name
denylist, and existing calls. Public/persisted types carry schema version 1.
Every SDK rejects unknown request fields/variants and unknown semantic response
variants. Interface docs and facade allowlists change in the same GREEN commit.

## TDD and verification design

RED begins with shared wire/digest fixtures and real-database Rust tests for:

- empty/129-operation bounds, ID grammar, unknown/version/type/path precedence;
- exact constructor return/error types and every existing-error-to-refusal
  mapping row, including Rust constructor, Python/TypeScript decode,
  Engine-wrapper, admitted-refusal, and whole-batch boundary/cursor/generation
  ownership and paths;
- all four variants, canonical/derived role mismatch, ordered backward and
  rejected forward references;
- same-logical-ID supersession, revision-pinned lifecycle, illegal states,
  all four legal transitions, duplicate/no-op dependency behavior, and
  dependency-closure refusal including `pending -> deleted`;
- failure injection at each operation and pre-commit boundary with zero partial
  domain/receipt/cursor/projection state;
- same-ID exact replay, conflict, concurrent race, restart, corrupt receipt,
  checked cursor/generation exhaustion, and before/after-commit behavior;
- exact affected-ID order, unchanged boundary for dependency/lifecycle-only
  batches, one dependency-generation advance, projection cursor selection, and
  no duplicate notification/event;
- source erasure, operation tombstone, raw database/WAL canaries, and no
  unbounded open scan, including adversarial-volume reverse-index query-plan
  proof, purge, resolved lifecycle targets, refused multi-source requests, and
  restart;
- receipt corruption in every forbidden erased/refused/result column and every
  noncanonical array representation;
- source-reference raw corruption, invalid identity, orphan owner,
  erased-owner row, 1,025th row, keyed replay, purge, and source erasure;
- ordinary-write/actuation parity for every valid `SourceId` class, including
  embedded NUL;
- dependency-before/after-transition, dependency-before/after-supersession,
  exact dependency replay, and dependency against the replacement revision;
- exact array formulas/order at their maxima and one-over corruption;
- one persisted-refusal corruption case for every reason/index/path rule;
- counter/event behavior for commit, refusal, replay, conflict, erased ID,
  storage corruption, and infrastructure error;
- Rust/Python/TypeScript public exports, fixture parity, locally packaged
  smokes, Windows compile/runtime fixtures, and unchanged recovery denylist.

Property tests cover codec determinism, decode/encode round trips, operation
order, replay equivalence, and every exact receipt/reference limit and formula
above. GREEN changes production code only after the corresponding RED commit is
preserved. CUDA and live-model routes are inapplicable because actuation is
deterministic CPU/SQLite work.

## Existing-design disposition

| Input | Disposition |
| --- | --- |
| Typed-write and `PreparedWrite` ADRs | Preserve typed/no-SQL and existing `write`; successor ADR governs additive actuation composition. |
| Slice 15 revision/provenance design | Reuse without reinterpretation. |
| Slice 20 dependency design | Reuse its authority, validator, generation, and transaction-scoped apply seam. |
| Existing lifecycle/erasure designs | Reuse actual logical-ID/current-revision state machine and operator boundary. |
| Earlier prepared-request journal design | Historical evidence; rejected for retained 0.8.25 core. |
| Slice 30 closure design | Future successor; Slice 25 refuses closure-required operations until Slice 30 exists. |
| Slice 35/40 contexts/generations | Future additive contracts; no type or semantic dependency in Slice 25. |
