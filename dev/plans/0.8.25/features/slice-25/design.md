---
title: 0.8.25 Slice 25 — bounded atomic actuation design
status: DRAFT_REVIEW_FIX_2
design_version: 4
review_fix: 2
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
  PutCanonicalNode { record: ProvenancedNodeV1 }
  | PutDerivedNode { record: ProvenancedNodeV1 }
  | RegisterSourceDependency {
      dependency: SourceDependencyRegistrationV1
    }
  | TransitionLifecycle {
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
`ActuationRefusalReasonV1`, and `ActuationError`. `ActuationBatchV1::new` takes
the operation ID and operations, validates caller-only structure, and defaults
schema version 1 plus absent policy/boundary; typed setters validate optional
fields. Operation constructors take the exact nested values shown above.

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
`operation_count_invalid`, `operation_id_conflict`, `operation_id_erased`, and
`receipt_corrupt`. It exposes the lower-snake-case stable reason and RFC 6901
field path; persisted-state reasons use `/operationId` except corruption,
which uses `/receipt`.

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
Purge/erasure remain existing operator operations. A later put with the same logical ID uses existing write-time
supersession rather than an invented lifecycle transition of an old revision.

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
- `active -> deleted` for that source logical ID.

After ordered structural validation, a final prospective closure-safety pass
compares every source revision made non-current/non-live by the request with
both persisted dependencies and every dependency registration that would be
successful in the prospective state. Any match refuses the whole request,
regardless of operation order. A dependency attached to the new replacement
revision is not a dependent of the superseded source and does not trigger the
guard. This deliberate dependency-stage gate closes the visibility gap without
leaking future schema backward. Slice 30 will supersede this rule when it can
admit and fence closure atomically.

## Validation and refusal precedence

Construction/parsing failures return `EngineError::Actuation` and create no
receipt: unsupported schema, unknown field/variant, missing/wrong-typed field,
invalid ID grammar, empty or over-128 operations, or malformed nested public
type. Keyed persisted-state outcomes—same operation ID/different digest,
operation-ID erasure tombstone, and corrupt receipt storage—also return
`EngineError::Actuation`, but are checked after digesting and are not parsing
failures.

A closed, syntactically valid request admitted under a new operation ID can
produce a persisted terminal `refused` receipt. The first failure in this
order wins and supplies one reason code, operation index when applicable, and
RFC 6901 field path when applicable:

1. expected global write-boundary mismatch;
2. operation order: nested write/provenance validation, role mismatch,
   missing/forward reference, dependency validation, lifecycle validation;
3. dependency closure required;
4. checked write-cursor or dependency-generation exhaustion.

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
_fathomdb_actuation_receipts(
  operation_id TEXT PRIMARY KEY,
  schema_version INTEGER NOT NULL CHECK(schema_version = 1),
  request_sha256 TEXT,
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
  CHECK(json_valid(reason_codes_json) AND
    json_type(reason_codes_json) = 'array'),
  CHECK(json_valid(affected_revision_ids_json) AND
    json_type(affected_revision_ids_json) = 'array'),
  CHECK(json_valid(pending_projection_write_cursors_json) AND
    json_type(pending_projection_write_cursors_json) = 'array'),
  CHECK(json_valid(closure_operation_ids_json) AND
    json_type(closure_operation_ids_json) = 'array'),
  CHECK(
    (outcome = 'refused' AND json_array_length(reason_codes_json) = 1 AND
      resulting_write_boundary IS NULL AND
      resulting_dependency_generation IS NULL) OR
    (outcome IN ('committed','committed_closure_pending') AND
      json_array_length(reason_codes_json) = 0 AND
      refused_operation_index IS NULL AND refused_field_path IS NULL AND
      resulting_write_boundary IS NOT NULL) OR
    (outcome = 'erased' AND json_array_length(reason_codes_json) = 0)
  )
)

_fathomdb_actuation_receipt_source_refs(
  operation_id TEXT NOT NULL,
  ref_kind TEXT NOT NULL CHECK(ref_kind IN (
    'source_id','source_revision_id','artifact_revision_id'
  )),
  ref_value TEXT NOT NULL,
  PRIMARY KEY(operation_id, ref_kind, ref_value)
)
INDEX _fathomdb_actuation_receipt_refs_reverse
  ON _fathomdb_actuation_receipt_source_refs(
    ref_kind, ref_value, operation_id
  )
```

Only terminal data is stored. Receipt JSON arrays are canonical compact JSON,
bounded by the request limit, and validated lazily on keyed replay; open never
scans the table without a key. Application validation additionally requires
canonical array member types/order and lowercase digest form; SQL checks
structure and outcome/column consistency. Domain rows remain the authority.
The receipt table is an idempotency/audit record, not a recovery journal.

The source-ref index records source IDs, source revision IDs, and artifact
revision IDs named, created, or resolved as affected by the request—including
lifecycle targets and revisions implicitly superseded by a put—without
retaining source text or locators. Refused requests index every valid-shaped
identity in the normalized request plus identities resolved before refusal.
Source erasure and purge collect target identities before domain deletion, use
the reverse index for bounded keyed lookup, and change every matching terminal
row to an opaque `erased`
tombstone: it nulls the digest and optional diagnostic/result fields, replaces
all arrays with `[]`, and removes source-ref rows in the erasure transaction.
Deletion is explicit; correctness does not depend on SQLite foreign keys.
The operation ID remains reserved. Replay then returns typed
`operation_id_erased`; it cannot reveal old bytes, hashes, IDs, or outcomes and
cannot reuse the idempotency key for a different request. Redacting an entire
receipt when any indexed member is erased is permitted safe over-redaction.
Query-plan/bounded-work tests prove use of the reverse index. Raw-byte and WAL
canaries prove removal across source erasure, lifecycle-target receipts,
refused multi-source batches, purge, and restart.

## Transaction and runtime flow

For a new request the Engine performs this sequence:

1. Strictly parse and normalize the request; compute its digest.
2. Keyed receipt lookup. An exact terminal match replays without draining
   projections. Conflict, erased tombstone, or corruption returns typed error.
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
- all four variants, canonical/derived role mismatch, ordered backward and
  rejected forward references;
- same-logical-ID supersession, revision-pinned lifecycle, illegal states,
  duplicate/no-op dependency behavior, and dependency-closure refusal;
- failure injection at each operation and pre-commit boundary with zero partial
  domain/receipt/cursor/projection state;
- same-ID exact replay, conflict, concurrent race, restart, corrupt receipt,
  checked cursor/generation exhaustion, and before/after-commit behavior;
- exact affected-ID order, unchanged boundary for dependency/lifecycle-only
  batches, one dependency-generation advance, projection cursor selection, and
  no duplicate notification/event;
- source erasure, operation tombstone, raw database/WAL canaries, and no
  unbounded open scan, including reverse-index query-plan proof, purge,
  resolved lifecycle targets, refused multi-source requests, and restart;
- dependency-before/after-transition, dependency-before/after-supersession,
  exact dependency replay, and dependency against the replacement revision;
- Rust/Python/TypeScript public exports, fixture parity, locally packaged
  smokes, Windows compile/runtime fixtures, and unchanged recovery denylist.

Property tests cover codec determinism, decode/encode round trips, operation
order, replay equivalence, and receipt-array bounds. GREEN changes production
code only after the corresponding RED commit is preserved. CUDA and live-model
routes are inapplicable because actuation is deterministic CPU/SQLite work.

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
