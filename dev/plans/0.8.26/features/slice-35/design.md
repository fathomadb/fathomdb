---
title: FathomDB 0.8.26 Slice 35 — breaking V1 actuation spike design
status: APPROVED
---

# Slice 35 design — breaking V1 actuation spike

## Fixed decisions

- 0.8.26 changes affected V1 contracts in place and introduces no parallel
  functional V1/V2 API pairs (`seq-283`).
- There is one actuation method per binding and one functional V1 grammar.
- Current V1 includes canonical node, derived node, derived edge, source dependency,
  and lifecycle transition operations.
- No V2 parser, method, router, redirect, or interaction mode is introduced.
- No historical request, digest, replay, receipt, integrity, data, operation-ID,
  upgrade, downgrade, or database-migration compatibility is retained.
- 0.8.26 accepts fresh databases only.

## Endpoint and receipt decisions

- Derived edges require both endpoints in the complete prospective batch
  state; endpoints later in the batch count (`seq-284`). Missing endpoints
  refuse the batch, and no dangling count is admitted.
- `ActuationReceiptV1` changes in place and stays compact (`seq-285`). Retain
  the current truthful outcome, operation identity and digest, refusal,
  affected revisions, transaction/projection boundaries, generation/closure,
  and internal source-reference concepts; add only edge fields the spike
  proves necessary. Do not add a graph-consequence manifest.

## Prototype seams

### Ingress

The Rust facade continues to export V1 types. Python, TypeScript, PyO3, and
N-API validate the changed V1 contract as one closed schema. The package
version identifies the breaking revision; the public schema-generation name
does not change.

Use one V1 parser and the existing actuation method. Do not add V2 types, a
V1/V2 union input, a version router, a redirect response, or separate
`actuate_v1`/`actuate_v2` methods.

### Fresh database

Fresh creation may reuse internal ordered schema-construction functions. The
spike adds a read-only pre-open classifier parameterized by an expected schema,
not the public-open cutover: missing or zero-length paths classify as fresh,
while a non-empty SQLite file is opened read-only/no-create and its
`user_version` is compared with prototype schema 34. A test-only migration list
appends one no-op step 34 to the current schema-33 list and proves a fresh
database can bootstrap and run the current engine at that version. A real
schema-33 database must be refused without changing the database, WAL, journal,
lock, or recovery-sidecar bytes. Slice 40 owns the real step 34 and makes the
classifier mandatory on every public open entry point; this slice adds no
production migration step, matrix, or converter.

### Current V1 transaction

Current V1 validates its closed grammar and digest, simulates the bounded
complete batch with the existing node/lifecycle ordering rules, and computes
the final active logical-identity set. It refuses any derived edge whose
endpoints are absent or non-active in that complete prospective state,
including when a same-batch lifecycle operation removes eligibility after a
node put. The edge's own position in the batch does not change this test. The edge
uses canonical `ProvenancedEdgeV1` validation, identity, provenance,
projection, lifecycle, and traversal machinery as implementation reuse, not as
historical database compatibility.

Implementation uses the existing rollback-only savepoint simulation. Derived
edges are applied there with the canonical `PreparedWrite::ProvenancedEdge`
validation/projector just like nodes. After all operations have run, one pass
visits derived-edge operations in request order and performs prepared indexed
existence probes against `canonical_nodes(logical_id)` with
`superseded_at IS NULL AND state='active'`. It checks `from` before `to` and
returns `reference_unavailable` at `/operations/{i}/record/from` or `/to`.
Because the check occurs after the whole simulated batch, an edge may precede
its endpoints while later puts, supersession, and lifecycle transitions still
determine the final state. The committed pass reuses the already-proven order
and transaction; it does not add a second policy interpretation.

### Current V1 receipt and replay

Use one current V1 digest domain, operation-ID namespace, receipt schema, source-reference
model, integrity checker, erasure behavior, and replay loader. Do not add a
request-version column or cross-release branch. Exact current V1 replay returns
its current V1 receipt; changed bytes conflict; erased operation IDs remain
reserved.

`PutDerivedEdge` receives a new unique digest tag followed by an explicit,
length-delimited encoding of every `ProvenancedEdgeV1` field and nested
provenance field. Receipt schema stays unchanged: the new edge revision and any
superseded edge revision enter `affected_revision_ids`; unfinished edge
projection enters `pending_projection_write_cursors` and binds the current
`projection_generation_id`. Direct and resolved edge source/revision identities
enter the existing reverse source-reference table so erasure redacts the
receipt. Receipt validation admits the edge-owned pending cursor and the two
new endpoint refusal paths, but no dangling count or graph manifest.

Canonical edge application can retire at most two distinct active revisions:
one selected by logical-ID G0 and one selected by fact-edge triple G11. The new
edge makes the truthful maximum three affected revisions per operation. Raise
the per-operation receipt formula from two to three and the global bound from
256 to 384, collect both prior revisions before application, deduplicate them,
and add acceptance/corruption tests for exactly three, 384, and one-over-bound.

### Bindings and failure precedence

Python accepts `put_derived_edge` with a snake-case edge record and TypeScript
accepts the same discriminator with a camel-case record. Both reuse the
ordinary provenance-bearing edge translators, wrapped by the actuation closed-
shape validator so nested failures are rooted at `/operations/{i}/record`.
The existing top-level precedence is retained exactly: schema version, sorted
top-level unknown field, required operation ID/type, decision-policy type,
expected-write-boundary type/canonical number, operations presence/list type,
operation-ID grammar, decision-policy grammar, operation count, then each
operation discriminator and nested record. Collision fixtures combine a
malformed edge record with every earlier-precedence family rather than claiming
a reordered parser.
The new endpoint refusal paths are `/operations/{i}/record/from` before
`/operations/{i}/record/to`. Embedded NUL remains valid only in `source_id` /
`sourceId`; unpaired UTF-16 remains rejected before N-API conversion without
changing native precedence.

## Performance design

The edge path runs during simulation and committed application, increasing
serialized writer occupancy. Endpoint checks must use bounded indexed probes
over the complete prospective state. The spike measures before refactoring;
ordinary `Engine.write` is outside the optimization scope unless evidence
shows a shared defect that cannot be isolated.

The spike crosses the edge's position with node-put and lifecycle-operation
orderings so the endpoint test cannot accidentally use the edge's insertion
position or pre-batch active state. Existing last-operation/lifecycle semantics
still determine the final prospective state.

The measurement harness creates a fresh database per arm and scenario, disables
no product checks, and reports exact logical-unit and API invocation counts. It measures the
canonical three-operation unit (derived node, dependency, edge), a 128-item
mixture that remains within the source-reference cap, 1,000 sequential unique
operation IDs, eight concurrent unique IDs, and eight callers replaying one
exact ID. The fixed control performs the corresponding pre-edge two-operation
actuation plus ordinary edge write so the comparison is reproducible in-tree;
it is characterization, not a claim of byte-identical 0.8.25 packaging.

Timing surrounds each complete logical unit: one `actuate` call for the
candidate and one `actuate` plus one ordinary `write` call for the control.
Response bytes are canonical JSON bytes for the actuation receipt plus the
ordinary write receipt for the control. Eight-call runs report end-to-end
latency and completion spread for the in-process connection-mutex queue. The
lock metric is narrowly the count of calls failing with the public storage/
locking failure channel; it does not attribute an unexposed SQLite lock owner.
Slow-event incidence comes from the existing attached lifecycle subscriber.
Pending projections are settled through the public cursor completion boundary
before size capture; each engine is closed, WAL-checkpointed with the supported
test helper, and database plus remaining sidecar bytes are summed at the same
checkpoint. Receipt size and database growth are reported per logical unit and
with raw totals.

## Existing versus net-new

Existing: atomic immediate transaction, rollback-only simulation, canonical
node/edge validation and projection, source dependency/lifecycle machinery,
receipt persistence/replay/integrity/erasure, indexed active logical-node
lookup, binding translators for ordinary edges, and writer/slow telemetry.

Net-new in this spike: the fifth V1 operation, edge digest encoding, final-
prospective endpoint refusal pass, edge receipt/source-reference inclusion,
closed binding variants/fixtures, read-only fresh-boundary classifier, and
measurement harness. Public-open enforcement, release version/schema cutover,
packaging, migration-test retirement, and release smoke remain Slice 40/50.

## Stop conditions

Stop on a parallel V2 parser or execution path, historical receipt/integrity
loading, earlier-database mutation, migration code, partial commit, ambiguous replay,
unindexed endpoint work, cross-binding drift, or semantic policy entering the
engine.
