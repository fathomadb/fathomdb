---
title: Inspector v0.1 Slice 20 — design
date: 2026-10-31
product: FathomDB Inspector
target_release: 0.1
status: ACTIVE
desc: Foundations and record inspection
---

# Slice 20 — design

## Status and intent

Original draft baseline `037a8522e` is reconciled in the [execution plan](plan.md).
Direct independent Sol-high design review and the scoped refinements below
preceded affected implementation. [Status](status.md) records actual runtime
evidence and remaining fresh code review/verification; design approval alone
is not a runtime qualification.

A runnable local Inspector that safely attaches a supported database through an owned Engine lifecycle and browses canonical and operational data with complete record detail.

This feature refines the completed [Slice 15](../slice-15/plan.md)
requirements/verification and [Slice 16](../slice-16/plan.md) architecture
baselines. Reconcile this draft against both; shared contracts belong in the
canonical design and verification matrix, with feature-local detail here.

## Components and data flow

Inspector-owned session/admission adapter, typed references/observations, bounded read handlers, browser structure navigator, record renderer and shared selection state.

```text
Local launch → select database → preflight existing file → qualified Engine startup → establish session → page structure contents → inspect exact item → detach.
```

All new components are Inspector-owned. Use the existing shell/session/typed
reference mechanisms from prior completed slices where available rather than
create parallel frameworks. UI interaction and read/backend behavior form one
slice-complete feature; table-only placeholders do not replace required views.

## Read and presentation contracts

Session/database/class-qualified locators, exact revision evidence where available, collection/key/log-entry identities, page scope and read context. Readable/structured payload and metadata are separate.

Engine startup and its normal workers are permitted. Inspector exposes no
editing, migration or repair operations; cache-only/no-download constraints
remain. Do not promise a byte-preserving or fixed-data session. A real record/result is
never synthesized from layout, equal text or guessed provenance. Product
metadata and factual unknown/unavailable states remain explicit.

## Dependency feasibility and ownership

Admission/schema/lock/read shapes are inspected in unchanged FathomDB source. Existing public libraries may be consumed as-is. Use qualified existing Engine open/read APIs where usable. An Engine code change remains out of scope; startup is allowed and its effects must be disclosed/tested.

Source reads are permitted for conformance and reuse assessment. No design
sentence authorizes edits to Engine, schema, MCP, SDK or agent-access. Validate
existing callable surfaces versus private/test-only helpers. Detail any
necessary Inspector-local storage adapter and its semantics/conformance before
implementation; an unavailable required capability is recorded, not hidden.

## Errors, resources and consistency

Preflight refuses unsupported schema and external-writer conflicts without bootstrap or recovery. Unknown links/completeness remain explicit. Engine-owned background progress changes observations, not session identity. Detach invalidates old references; no immutable connection reads a changing Engine database.

Use explicit operation/read context, bounded work and session-local resource
ownership. Shared connection/scratch state is serialized or isolated. The
Engine-backed allowance requires lifecycle-effect and observation tests, not
byte preservation after successful startup; GUI cancellation and actual
execution interruption remain distinct. Run the relevant slice fixtures and
UI flows from [plan](plan.md), including negative outcomes.

## Design decisions to complete during execution

- Enumerate draft deltas and confirm feature allocations against current code.
- Select the minimal Inspector-local payloads, adapters and view contracts
  needed for this slice, not a speculative universal data-browser framework.
- Record actual existing-library reuse, missing capabilities and effects on
  Engine effects, read-context fidelity and cache constraints; apply adopted D-01 to affected reads/export without reruling it.
- Map each contract to the slice's acceptance and independent oracle, then
  obtain Sol-high review and proceed to TDD implementation.

No Engine/MCP redesign, roadmap promotion, global contract rewrite or separate
worktree is required by this draft. Follow the
[common workflow](../slice-workflow.md) for review, verification and closure.

## Slice 15 baseline input

The [shared matrix](../verification-plan.md) baseline **S15-2026-10-30** and
[canonical operating envelope](../../../requirements.md#verification-and-operating-envelope-slice-15-2026-10-30)
now supply fixture/oracle/owner/stage obligations. Use the assigned rows in
[the plan](plan.md); architecture/toolchain feasibility is Slice 16 work.
Preserve zero/one/many origins, exact observation/revision, class-specific
eligibility, effect allowlists and honest unavailable states. INS-X07–X11 add
local access, inert rendering, keyboard equivalence, installed lifecycle and
aggregate bounds. Verify predecessor policy/completion in Slice 15 status.

## Slice 16 architecture handoff — S16-2026-10-31

Consume the [canonical architecture](../../../design/high-level-design.md) and
[Slice 16 status](../slice-16/status.md) before implementation. Central composition,
operation/reference/policy/resource/protocol contracts are selected there; local
refinement cannot silently replace them. The original draft did not establish execution; the status now records it. B16-01 is resolved by the 2026-10-31 owner ruling: retained-erasure content
is conditional on legitimate existence. Physical deletion remains unchanged.
Consume completed Slice16 before starting; typed supplied-content export
contracts are distinct from real Engine deletion/absence/flag integration.

Owned components: **A/S/P/O/R/K + U document/tree**. Implement the selected Rust supervisor/session process and plain browser shell, one-use local bootstrap, paired protocol, exact typed locators, guarded schema/marker/mean admission, bounded current-layout readers, independent policy gates and narrow retained-erasure export. Public logical point reads never replace exact revision lookup. Package has no Engine operator/test/benchmark/reranker features. Establish child/helper ownership and positive controlled fixtures before claiming lifecycle completion.

Assigned architectural verification: **M01–M07, M09–M13, M15, M17–M18, M20, M24, M29–M36, M39**, using existing fixtures/oracles
in the [single matrix](../verification-plan.md), including this feature's native
UI, keyboard, security, partial/error and resource paths. Test names/RED evidence
remain feature work. Feed semantic changes back to canonical requirements/AC/HLD
and affected matrix rows before implementing; preserve Inspector-only scope.

Postcompletion Astra N16-A01: Implement cleared worker environments before any dependency initialization, exact allowlists and separate private browser launch; add real hostile-parent F-05 header/effect/cache/log tests. The [canonical environment contract](../../../design/high-level-design.md#process-environment-is-part-of-the-selected-composition) and shared matrix own the one selected allowlist.

## Execution refinement — 2026-10-31

Implement one unpublished Rust workspace package under `src/inspector/`.
The binary dispatches internal modes before dependencies initialize. Supervisor
owns loopback HTTP, credential/bootstrap/private profile, bounded queue and
child process group. An inherited socket pair carries length-bounded JSON
frames; paths/policy/build are in its initial frame, never the child environment.
The session child configures Diagnostics before preflight SQLite, guards the
existing layout/markers/profile/generation/mean, then opens unchanged Engine
with explicit None when model assets are unavailable. It owns serialized reads
and Engine close. No fixture/debug mode is exposed as an HTTP operation.

Use a single discriminated request enum with reject-unknown parsing. Named
operations are attach, catalog, page, exact detail, chunk/copy, identity lookup,
cancel, detach and close. All data operations require launch/build/protocol and
attachment pair plus opaque server-minted locator. Page constraints are explicit
kind, role, source or collection; unsupported ones refuse before reading.
Opaque handle registry retains typed exact class/cursor/revision/collection/key
coordinates. A per-request WAL-aware read-only transaction attests one
observation; subsequent requests and chunks have distinct observations.
Canonical and operational payload variants stay separate. Every response states
scope, status, examined/returned work and continuation. Finite response and
statement-progress budgets apply before allocation and delivery.

Plain bundled DOM modules render a structure navigator, bounded record cards,
metadata, prewrapped text chunks and a lazy lossless JSON token tree. Scalar
lexemes remain original text, including large numbers; explicit copy revalidates
the locator through service. Tree expansion, related links and Back are actual
keyboard controls with focus return. Bounded navigation state stores locators
and context, clears payloads on expiry, and announces observation eviction.
Service-generated text is rendered via text nodes; no payload enters URLs.

Integration fixtures use ordinary Engine writes and public collection writers;
controlled source/provenance seams remain test-only. Before any Engine open,
refusal tests fingerprint product/sidecars/cache. Positive attach observes
caller identity/content invariants, allowed metadata/WAL/projection effects and
owned release. Protocol tests and real Chromium flows verify one-use bootstrap,
Host/Origin/cookie/build pairing before data access. Resource tests exercise
real owned child kill/reap and queue/late-response boundaries. Model-positive
runtime remains Slice40; this foundation never calls a fetching constructor.

## Reviewed fixture-control refinement — 2026-10-31

The selected default build excludes the nondefault Inspector-only
`fixture-control` feature. Its separate build identity includes compiler,
target, profile, flags and features. The private example driver can submit
bounded, typed internal fixture jobs over its owned input; HTTP always creates
read jobs and cannot invoke controls, including in fixture builds. Controls use
the same cleared-environment child and actual owned Engine public lifecycle
operations. They do not enable Engine test/operator features, external writers,
retention, or mock storage. An acknowledged one-shot pause and a separate
pre-initialization wrapper exercise ownership and late-response boundaries;
synthetic blockage does not prove interruption of an actual native call.
The independent Sol-high design reviewer accepted this refinement with these
boundaries. Default application reads and package remain free of controls.

## Reviewed observed-field sample refinement — 2026-10-31

A deterministic first-N sample uses the same browse transaction/keyset, typed
class, exact filters and policy as record pages. It returns at most ten retained
records and inspects at most 64 KiB of complete content per permitted record,
with at most twenty top-level field names. It reports candidate rows, sampled
records, inspected/skipped/refused payloads, byte budget and field truncation;
it never proves field absence across the population or infers registered schema.
Oversize, null, nonstructured and unsupported content are explicit. Lossless
key ranges provide exact field names, including keys larger than the tree's
preview; placeholder labels are never sampled facts. Browse and every scanner
share one operation deadline and byte budget. The independent Sol-high reviewer
accepted this refinement before implementation. UI cards, Inspect, filters and
Back expose the actual sample and its bounded scope.

## Reviewed incremental-read refinement — 2026-10-31

Source inspection and the isolated-worker allocation RED `e9250ecb3` found that
SQLite materializes overflow text before `substr(CAST(... AS BLOB))`; a 256 KiB
window over a 128 MiB value grew worker RSS by 136,105,984 bytes. The selected
reader must use read-only incremental handles for ordinary rowid TEXT/BLOB
columns in the same validated request transaction. Direct `typeof(column)` and
`octet_length(column)` probes avoid loading payloads for type/size checks.
NULL, empty, numeric metadata and UTF-8 boundaries remain explicit. Only the
qualified UTF-8 header encoding is admitted before a database connection;
header2/3 sentinels are refusal faults, not UTF-16 compatibility fixtures.
Metadata preview evaluation must be bounded before column materialization,
and every operation retains one work/byte/deadline budget. Handles close within
their request; virtual tables have no fallback to this route. The existing
rusqlite `blob` feature is Inspector-local build wiring, not a dependency pin,
Engine API, schema or data-retention change. The independent Sol design reviewer
confirmed the bundled SQLite/rusqlite source and accepted this refinement before
implementation. Exact chunk/copy and actual memory/scale regressions qualify it.

## Reviewed full metadata fields — 2026-10-31

A closed class-specific field resolver serves exact field detail and chunk
reads, including attribute and revision-table mappings. Full field detail
returns record metadata, selected content and observation from one transaction;
later chunks have separate observations. UI labels bounded previews and keeps
the selected field through Back, refresh and copy. NULL, empty, missing revision
evidence and unsupported fields remain distinct. Integer metadata uses exact
decimal text. REAL metadata uses a round-trip representation of the stored
binary64 value; the original producer decimal lexeme is not retained. Numeric
preview text and length use the same representation as full copy. Independent
Sol-high reviewed this refinement before implementation.

## Reviewed scalar materialization constraint — 2026-10-31

Node browsing accepts an exact stored scalar attribute-name presence constraint.
Projection declarations link to that scoped origin list and existing attribute
detail; truncated declaration names cannot dispatch an exact lookup. This does
not prove body-field absence, declaration coverage or vector readiness: the
selected Engine skips absent, null and composite values. Other classes reject
the constraint before read dispatch. Attribute candidates are explicitly counted
in the same 100,000-row operation budget as origins, under the same connection
deadline, using the existing cursor index and bounded name windows without
reading attr_value. A cap while classifying one origin is partial with no
misleading continuation or empty proof. Apply, sample, pagination, Back and
refresh preserve the constraint. Independent Sol-high accepted this refinement
with the shared candidate-budget condition before implementation.

## Reviewed cooperative work deadline — 2026-10-31

Current-layout enumeration uses one 25-second cooperative work deadline inside
the unchanged 30-second supervisor deadline. Reserve delivery/teardown headroom;
never reset the deadline per class, declaration or attribute. Stop with partial
facts and explicitly incomplete interrupted/unvisited categories, while retaining
the shared 100,000 examined-row maximum. Browse preserves a valid continuation
for completed candidates and never treats a time cap as an empty proof. The
supervisor's hard expiry/termination remains authoritative. Catalog uses the
same classification implementation with only fields needed for its scope,
not a copied policy implementation. Actual native F06 failure and independent
Sol source review established this correction before implementation.

## Reviewed aggregate catalog envelope — 2026-10-31

Collections, projection declarations and kind/namespace group entries share
one 200-item response counter and actual serialized UTF8 JSON byte accounting.
Reserve 64 KiB for bounded database/context/count/scope facts and counter growth;
validate the entire serialized envelope against 2 MiB before IPC. Omitted or
preview-truncated metadata and interrupted/unvisited scans carry independent
completeness flags. Counts retain their own measured exactness, even when
metadata is omitted. Combined collection discovery includes observed unregistered
namespaces and cannot be complete if the relevant operational scans/groups are
partial. UI never describes unenumerated declarations as absent or dispatches
an exact collection/projection lookup using a truncated name. Independent Sol
review confirmed these existing response bounds before affected implementation.

Operational Current/Log category navigation remains available even when catalog
metadata is capped. All-collection cards visibly qualify collection and key;
retained log entries also show their exact entry ID and cursor. These controls
reuse the existing bounded readers and preserve policy, handles and Back scope.
Independent Sol reviewed this local UI refinement before implementation.

Operational copy actions explicitly name the stored record key and retained
log entry ID; full-field access uses the same existing exact column resolver.
Canonical source/attribute associations are identified as not applicable to
operational entries, rather than implying a canonical revision history.

Association target choices show the already-returned class, exact recorded
revision or explicitly absent evidence, and labeled internal write cursor.
Unique descriptions connected with aria-describedby distinguish equal logical
IDs for keyboard users. Selection still dispatches the exact opaque handle;
no identity is inferred from the displayed name. Independent Sol approved this
presentation refinement before implementation.
