---
title: 0.8.25 Slice 25 — bounded atomic actuation design
status: DRAFT_REVIEW_FIX_1
design_version: 3
review_fix: 1
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
) -> Result<ActuationReceiptV1, ActuationError>
```

`committed_closure_pending` and `closure_operation_ids` are reserved additive
contract points for Slice 30 and are never emitted/populated by Slice 25.
Unknown request fields and variants reject before execution. Unknown response
fields may be ignored; an unknown outcome or reason code rejects. Rust uses
`u64` internally for boundaries/generations/cursors; Python integers and
TypeScript/wire decimal strings use the Slice 15 parity rules.

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
current immutable revision ID observed by the caller. Only existing
`active -> deleted` and `deleted -> active` transitions are admitted through
actuation. `pending`, `purged`, and nonexistent semantic states such as
`invalidated` are not actuation targets. Purge/erasure remain existing operator
operations. A later put with the same logical ID uses existing write-time
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

The refusal occurs before any domain mutation. With no dependents, existing
supersession or lifecycle behavior applies. This deliberate dependency-stage
gate closes the visibility gap without leaking future schema backward. Slice
30 will supersede this rule when it can admit and fence closure atomically.

## Validation and refusal precedence

Construction/parsing failures return typed `ActuationError` and create no
receipt: unsupported schema, unknown field/variant, missing/wrong-typed field,
invalid ID grammar, empty or over-128 operations, malformed nested public
type, same operation ID/different digest, operation-ID erasure tombstone, or
corrupt receipt storage.

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
`write_cursor_exhausted`, and `dependency_generation_exhausted`. Existing
nested error codes remain available in a non-authoritative diagnostic detail;
the reason code is stable. A refused receipt has no affected IDs, resulting
boundary/generation, projection cursors, or closure IDs.

SQLite I/O, commit, lock, and injected infrastructure failures return an
`ActuationError`, roll back, and store no receipt. They are not domain
refusals. This distinction avoids recording a result that did not commit.

## Canonical request digest

The Engine computes SHA-256 over a domain-separated, length-prefixed binary
encoding of the normalized typed request. The byte stream starts with
`fathomdb.actuation.v1\0`; every field uses a fixed tag, every optional field
uses an absent/present byte, strings use UTF-8 with an unsigned 64-bit
big-endian byte length, integer fields use unsigned 64-bit big-endian, and
operations retain array order and carry a fixed one-byte variant tag. Nested
record and dependency fields are encoded field-by-field in their declared
wire order; maps are not serialized. This avoids JSON key-order and number
ambiguity.

Shared fixtures contain the complete request, canonical-byte hex, and
lowercase 64-character digest. The Engine always computes the digest; callers
cannot supply it. Digesting streams through fields so it does not duplicate a
whole body in memory.

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
  CHECK(outcome IN ('committed','refused','erased'))
)

_fathomdb_actuation_receipt_source_refs(
  operation_id TEXT NOT NULL,
  ref_kind TEXT NOT NULL CHECK(ref_kind IN ('source_id','source_revision_id')),
  ref_value TEXT NOT NULL,
  PRIMARY KEY(operation_id, ref_kind, ref_value),
  FOREIGN KEY(operation_id) REFERENCES _fathomdb_actuation_receipts(operation_id)
    ON DELETE CASCADE
)
```

Only terminal data is stored. Receipt JSON arrays are canonical compact JSON,
bounded by the request limit, and validated lazily on keyed replay; open never
scans the table without a key. Domain rows remain the authority. The receipt
table is an idempotency/audit record, not a recovery journal.

The source-ref index records canonical source identities and source revision
identities named or created by the request without retaining source text or
locators. Source erasure changes matching terminal rows to an opaque `erased`
tombstone: it nulls the digest and optional diagnostic/result fields, replaces
all arrays with `[]`, and removes source-ref rows in the erasure transaction.
The operation ID remains reserved. Replay then returns typed
`operation_id_erased`; it cannot reveal old bytes, hashes, IDs, or outcomes and
cannot reuse the idempotency key for a different request. Raw-byte and WAL
canaries prove removal.

## Transaction and runtime flow

For a new request the Engine performs this sequence:

1. Strictly parse and normalize the request; compute its digest.
2. Keyed receipt lookup. An exact terminal match replays without draining
   projections. Conflict, erased tombstone, or corruption returns typed error.
3. Drain the existing projection worker as required by `write`, acquire the
   writer mutex, and begin one `IMMEDIATE` transaction.
4. Repeat the keyed receipt lookup inside the transaction to close races.
5. Read and compare the current global record write boundary. This boundary is
   the existing maximum reserved canonical record cursor, not an actuation or
   dependency generation.
6. Validate the ordered prospective state using side-effect-free validators.
   On domain refusal, insert the receipt and source-ref index only, then commit.
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

`resulting_write_boundary` is the final global record cursor for committed
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
  unbounded open scan;
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
