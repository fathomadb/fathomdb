---
title: Inspector architecture stress test — database 0.9 and 1.0 evolution
date: 2026-10-29
product: FathomDB Inspector
target_release: 0.1
status: ACTIVE
desc: Adversarial evolution scenarios, abstraction limits and recommended contract boundaries
---

# Inspector architecture stress test — database 0.9 and 1.0 evolution

> Current-policy note (2026-10-29): the owner subsequently permitted Engine
> startup when required. Earlier immutable/no-startup/preservation assumptions
> below are historical. Current [requirements](../requirements.md#engine-startup-allowance-2026-10-29)
> distinguish Inspector read operations from qualified Engine lifecycle effects.

## Assessment

The proposed boundaries can survive substantial FathomDB evolution. The
current sketches are not precise enough to establish that they will. A small
adapter is a useful place to contain change, not a promise that an old
Inspector will understand future database releases or that new concepts can
always fit today's UI unchanged.

The most consequential missing rule is **reader/query semantic coherence**.
Separating storage readers from query execution does not make arbitrary
combinations valid. They must agree on identity, revision resolution,
eligibility, representation decoding, read context, and measurement meaning.
Capability names or a matching schema number do not establish that agreement.

This is an adversarial thought experiment against `044f59c5b`, informed by
current source and accepted contracts. No 0.9/1.0 implementation was executed,
no future release content is assumed, and no runtime test pass is claimed.
Recommendations here are inputs to existing slice design reconciliation;
requirements and the current-schema support policy remain unchanged.

## What is actually established today

- The [architecture research](compatibility-architectures.md) and
  [Astra review](../reviews/compatibility-architecture-review.md) recommend one
  bundled UI/service pair, one pinned query build and a small compiled adapter.
- The [high-level design](../design/high-level-design.md) already separates
  sessions, typed observations/references, inventory, retrieval and UI. Safe
  read-only access remains required engineering work, not an implemented API.
- The [.28/.29 comparison](../design/compatibility-0.8.28-0.8.29.md) finds common
  schema/storage mechanisms but different transient query evidence in pinned
  snapshots. This motivates separate compatibility axes; it proves neither
  future-version support nor runtime equivalence.
- [Wire contracts](../../interfaces/wire.md) use the database schema sentinel
  and current-schema refusal, with operation-specific rules for additive fields
  and closed variants. Unknown semantic variants cannot simply be ignored.
- The [accepted 0.8.26 breaking-boundary ADR](../../adr/ADR-0.8.26-breaking-v1-contract-and-fresh-database-boundary.md)
  changes affected V1 contracts in place, provides no historical reader or
  migration, and defers parallel public API generations until post-1.0.
  This stress test introduces none of that machinery.

There is historical 0.9 planning in this checkout, but the
[old draft](../../plans/0.9.x-roadmap-DRAFT.md) is superseded and explicitly voids
its 0.9 placement. The [program document](../../plans/0.8.20-0.9.0-PROGRAM-SEQUENCING.md)
leaves 0.9 product identity/planning as forward work. Those documents are not
used as evidence that any scenario below will ship.

## Evolution scenarios and expected responses

All changes in this table are hypothetical stressors. Pass means the design
contains or explicitly rejects the change without silently inventing facts;
it does not mean the existing unimplemented Inspector already supports it.

| Hypothetical change | What should remain stable | Required response / failure of a naive abstraction |
| --- | --- | --- |
| 0.9 adds an index or optional metadata without changing existing meanings. | Existing domain observations and applicable views. | Requalify the reader/query pair and expose new facts deliberately. Ignore only fields whose contract explicitly permits it; do not assume all additive SQL changes are harmless. |
| 0.9 changes tables, encodings or the schema sentinel but retains the same user concepts. | Document, lexical, vector and graph interaction concepts. | Old Inspector refuses an unknown format without mutation. A newly qualified build updates its storage reader and any affected query integration. A plugin system is unnecessary. |
| 0.9 changes tokenizer, fusion, eligibility or score interpretation while table shapes remain identical. | Separation of storage facts from query observations. | Qualify a new semantic contract for the serving backend. Name actual route, score/metric family, population and read context; do not claim historical execution reproduction. Shape tests alone would pass incorrectly. |
| 0.9 introduces multiple embedding representations, another metric or a different ANN selection procedure. | Representation-scoped navigation and links to origins. | Bind distances/results to representation, authoritative embedder identity, metric, score direction, population and selection procedure. Never compare scores across incompatible operations or display a shortlist as globally exact neighbors. |
| Retrieval returns several passages per record, or one hit maps to multiple origins. | Typed identities, exact observed payloads and recorded links. | Keep hits distinct from canonical records and allow zero/one/multiple verified origin resolutions. Offsets need their content/revision and coordinate convention. A universal hit-to-one-row adapter would fail. |
| A new identity scope, tenant boundary or lifecycle rule appears. | Identity preservation and observation context as first-class concepts. | Extend the relevant typed contract deliberately; invalidate incompatible locators. Never map an unknown namespace/state to an existing default. Database/class qualification alone cannot represent a new scope unless the source contract establishes it. |
| A new FathomDB data structure appears, such as a relationship that cannot be represented as one directed binary edge. | Coordinated selection, ownership and links where their meanings still apply. | Add an appropriate domain payload and renderer/interaction. A generic record DTO or raw JSON/table fallback cannot claim complete FathomDB-native coverage. This legitimately changes domain, protocol and UI together. |
| Query APIs become asynchronous or depend on different native/GPU runtimes. | Request ownership, explicit prerequisites and cancellation outcomes. | Replace the execution collaborator and requalify lifetime/resource bounds. Process isolation may be needed if bounded cancellation cannot be achieved otherwise. Constructor injection does not itself solve native-call interruption. |
| Users request live/WAL, encrypted, remote or non-SQLite stores. | Typed operations and result provenance may be reusable. | Design new admission/access/consistency contracts. An immutable local file assumption cannot be generalized by swapping a connection string. Current Inspector refuses unsupported attachment; these are scope changes. |
| UI and local service are installed independently, or a tab survives a service restart. | Typed transport and session-qualified references. | For current bundled delivery reject mismatched pairing/session. Independent deployment would require an explicitly supported protocol policy. Capability flags cannot repair incompatible payload meanings. |
| 1.0 users expect the latest Inspector to open historical databases. | The architecture still permits bounded reader implementations. | Decide the support window, ownership and cost before promising it. Initial current-schema-only builds may continue to refuse older files. Historical readers/migrations are not authorized by this expectation. |

## The counterexample that an adapter cannot solve

Imagine two producer builds write the same layout and schema number but assign
different meanings to the same stored bytes. Neither records a discriminator
that the reader can observe. A new reader cannot reliably determine the
intended interpretation from those bytes. A larger capability registry or more
`if` statements cannot recover missing information.

A check for an unknown schema catches a declared format change. It cannot
guarantee detection of an undeclared semantic change that looks exactly like a
known file. Even a familiar shape and successful query can be misleading.

Preventing that ambiguity requires a durable distinguishing contract when
persisted meaning changes, or a support restriction grounded in known producer
provenance. That is an upstream format/release-policy issue, not an
Inspector-local trick. Do not add a second database manifest or invent producer
metadata here: the current public sentinel and authoritative contracts remain
unchanged. If the needed discriminator does not exist, record the limitation
rather than promise fail-closed detection of an unobservable difference.

Some semantic changes concern only the serving query build: those can be named
by that build's qualified execution contract. That does not establish the
producer's historical tokenizer, embedding procedure or execution behavior.

## Boundaries worth keeping, and what they must preserve

| Boundary | Responsibility | Stress-test condition |
| --- | --- | --- |
| Admission/session | Safe access, supported layout, observation context, resources, detach/invalidation. | Opening an unsupported future file never invokes recovery, migration or mutation-capable Engine startup. A new access mode needs its own consistency proof. |
| Storage inspection | Decode canonical/operational/representation facts and authoritative links. | SQL tables, row positions and private engine structs do not become UI identities or universal schemas. |
| Query execution | Perform the qualified operation with actual runtime prerequisites and evidence. | Execution does not silently reinterpret the reader's identities, visibility or vector representation. Missing evidence stays missing. |
| Domain observations | Preserve class/namespace/revision, cardinality, context, completeness and measurement meaning. | Normalization never loses a distinction that changes user interpretation; real conceptual change can require a breaking contract change. |
| UI/service protocol | Transport those typed meanings between a supported pair. | A stale/incompatible client is rejected; session references cannot be replayed after reattachment. |
| Structure-specific presentation | Render documents/values, lexical evidence, vector measurements and graph relationships appropriately. | Renderer changes are expected for genuinely new structures; a grid is not a universal compatibility fallback. |

These are responsibilities, not a commission for six crates, traits, registries
or independently deployed services. A few Inspector-local modules with explicit
constructor arguments can enforce them. Extract interfaces where needed by
real behavior and ownership, rather than build speculative factories.

The domain boundary should preserve small shared facts and operation-specific
typed payloads. A single object with dozens of nullable fields loses invariants;
a closed universal model frozen to today's variants prevents honest evolution.
Unknown variants cannot be silently coerced. Whether an additive field is safe
to ignore depends on its contract and meaning, not just its JSON shape.

### Qualify compositions, not independent flags

A candidate backend combination is acceptable only when its storage contract,
query semantics, extensions/runtime and Inspector normalization have been
qualified together. An internal build descriptor can identify that combination;
this is not new persistent database metadata or a user override.

For example, qualifying a decoder for an old layout and qualifying a new
vector backend separately does not prove the latter can operate on that layout
without modifying it, or that its eligibility rules match. If supporting more
than one combination is approved later, retain explicit tested combinations,
not arbitrary combinations of every reader, query build and runtime version.
Do not compile several historical engines or introduce routing now.

The one-pinned-backend recommendation remains sound as the initial strategy.
It is a build/deployment choice, not a promise that there will only ever be one
implementation across all Inspector releases.

## What 0.9 and 1.0 change in practice

At **database 0.9**, a current-schema Inspector release can be rebuilt and
requalified against the selected 0.9 contract. If user concepts remain stable,
most view behavior should remain reusable; changes concentrate in admission,
storage/query adapters and conformance fixtures. If identity or data concepts
change, corresponding domain/protocol/view updates are correct, rather than
signs that every detail belongs behind a storage adapter.

An old Inspector may safely refuse the new database. That is successful
containment of unsupported input, but it is not forward compatibility. Separate
the claim that the architecture accommodates a new build from the claim that
one released binary accepts both old and new databases.

At **database 1.0**, choose precisely which inspection API, persisted formats,
identity rules, measurements and support window are stable. Semantic Versioning
requires a declared public API and defines 1.0 around it. It does not, on its
own, specify a database-file support window or promise that an old Inspector
understands new structures; that depends on what the product contract covers.
[Semantic Versioning 2.0.0](https://semver.org/spec/v2.0.0.html).

Inspector's own release numbering remains independent of FathomDB's numbering.
No universal adapter system needs to be delivered merely because FathomDB
reaches 1.0. The concrete longer-term ownership choice is:

- Continue Inspector-owned storage decoding, accepting the ongoing duty to
  maintain semantic conformance and explicit release qualification; or
- Separately commission a supported, genuinely read-only engine inspection
  contract, if its stability and reduced duplication justify upstream work.

The latter is a future option requiring its own Engine scope and accepted
contracts; it is not authorized work in these Inspector slices. Neither option
automatically creates historical-file support or stable dynamic Rust plugins.

## Recommended strengthening during the existing slices

1. In Slice 20 detailed design, explicitly separate admission, storage facts,
   query execution and result normalization, and qualify their composition.
   Define minimum identity/context/evidence carriers and the bundled protocol
   check. Preserve the one-current-schema/no-historical-shim policy.
2. In Slice 30, bind lexical evidence to the actual route, indexed fields,
   tokenizer and score family. Never equate a current requery with historical
   execution merely because both read the same record.
3. In Slice 40, scope vector operations to representation and actual measurement
   semantics; test changed metrics/selection and unchecked/missing runtime
   prerequisites without weakening required positive fixtures.
4. In Slice 50, test cardinality and identity assumptions: duplicate bodies,
   exact revisions, ambiguous origins, endpoints and class-correct eligibility.
   A new structure/namespace requires deliberate payload and view changes.
5. In Slice 60, qualify the chosen producer/backend/build combinations and
   installed UI/service pair. Define the support statement from evidence rather
   than a version inequality such as FathomDB >= 0.8.28.

These are design/evaluation recommendations, not new requirements, an approved
ADR, a delivered feature or a support-window decision. The existing review/TDD
workflow applies when a slice changes design and implementation.

## Adversarial evaluation to run when implemented

| Test / experiment | Required result or useful disproof |
| --- | --- |
| Real fixtures from each approved producer, using one Inspector build. | Correct identity/content/eligibility/measurement semantics and product/cache preservation for each explicitly supported combination. Source diff alone is insufficient. |
| Unsupported sentinel and malformed current-shape fixtures. | Bounded refusal before mutation, accurate reason, no guessed decoder or generic-table fallback. A negative future-sentinel fixture is not an actual 0.9 database. |
| Same-shape changed query semantics, using independently specified expectations. | Raw value, score family, context and operation evidence preserve changed meaning. No fixture automatically regenerates an oracle from implementation output. |
| Representation/profile mismatch, unsupported metric or missing cached assets. | No false comparability/readiness; no download/repair. Appropriate refusal or explicitly incomplete evidence, never a successful required-positive claim. |
| Many-to-many hit/origin links and equal bodies across revisions/classes. | No identity collapse, guessed origin, latest-revision substitution or dangling cross-session locator. |
| Unknown meaningful enum/structure in an otherwise parseable response. | Explicit incompatibility or bounded unavailable behavior according to contract; no conversion to an existing state. Existing required fixture coverage remains mandatory. |
| Stale tab, service restart and database reattachment. | Reject incompatible pairing and expired references before using them for reads. |
| Large inventory and slow/non-interruptible query calls. | Aggregate limits, owned resources, cancellation/late-result handling and detach cleanup satisfy the actual contracts. Per-page/per-query bounds alone do not establish this. |
| Release-impact inspection of changed files. | Stable concepts should not require duplicated engine-version conditions in every view. Broad view changes should trace to a genuine domain/protocol change; otherwise the boundary is leaking. |

Hypothetical fixtures can expose design assumptions; they cannot certify an
unreleased producer. Executable conformance against actual artifacts is still
needed before any .9/1.0 compatibility statement.

## Engine-side considerations

Follow-up assessment at `bc5e797c2`: the highest-value engine contribution
would be a supported, genuinely read-only inspection contract. A description
file alone cannot supply safe access or authoritative query semantics. The
items below are recommendations for a separately scoped Engine proposal, not
changes authorized or implemented by Inspector v0.1.

### Existing substrate versus the missing boundary

The current [operator inspection path](../../../src/rust/crates/fathomdb-engine/src/operator/data_plane.rs)
provides `inspect_data_plane_integrity`: current-schema, quiescent, immutable,
query-only access without startup maintenance or lock metadata mutation. It
performs integrity operations and closes its connection; it is not a general
browse/search session. Recovery functions in the same module are distinct,
mutation-capable operations and are not inspection substitutes.

[Projection declarations](../../../src/rust/crates/fathomdb-engine/src/projection_registry.rs)
and [generation status](../../../src/rust/crates/fathomdb-engine/src/projection_generation.rs)
already expose useful types and Engine methods. Those methods requiring an
opened Engine do not establish immutable admission. Declared options are not
necessarily execution facts: the source's `ProjectionFts.tokenizer` contract,
for example, permits a recorded override that is not honored by that path.
A future catalog must report effective behavior separately from declarations.

The [schema sentinel](../../interfaces/wire.md#schema-version-sentinel),
[typed identities](../../../src/rust/crates/fathomdb-engine/src/identity.rs),
[embedder-owned identity rule](../../adr/ADR-0.6.0-vector-identity-embedder-owned.md)
and existing provenance/generation registries should remain authoritative.
Do not recreate those facts in a manually maintained Inspector manifest.

### Proposed engine responsibilities

| Surface / responsibility | Information or behavior needed | Why it matters |
| --- | --- | --- |
| Build description, available before opening a database | Exact serving build, current inspection-contract identity, supported storage contract, compiled operations and extension/runtime requirements. | Inspector can assess its serving dependency without ordinary Engine open, model loading or feature execution. The serving build is not the database producer. |
| Safe inspection session | Explicit quiescent admission, immutable/query-only reads, observation identity/context, owned bounded work and detach. No bootstrap, checkpoint, reconciliation, projection workers, downloads or cache repair. | Makes safe semantics reusable instead of making Inspector recreate admission and query ownership from private code. Model-backed work remains explicit and separately governed. |
| Typed database catalog | Canonical node/edge classes and identity rules; operational collections and modes; stored declarations, actual representations and authoritative origin links; physical structures where useful; generation/readiness scope. | Describes this database's contents and storage-to-retrieval relationships. Bounded pages and explicit scope replace full scans or inferred relationships. |
| Semantic read/query operations | Exact retained revisions, complete directed-edge detail, origin resolution and supported lexical/vector operations using the engine's own class-correct semantics. | A catalog does not remove the need for actual operations; returning current content for a retained-revision locator is still wrong. Disclosure rules must be explicit. |
| Request evidence | Actual route, indexed fields/tokenizer/scoring family, vector metric/direction/population/selection, read context, representative versus contributing branches, and incomplete/skipped/failed/cancelled states when known. | A static capability cannot establish what a particular request did. Missing evidence stays unknown. |
| Persisted-meaning change discipline | Change a durable format discriminator when existing bytes acquire incompatible meanings; reuse the established schema authority where adequate. | Prevents different interpretations being indistinguishable to a reader. An independent subformat identifier needs a demonstrated need and its own approved contract. |
| Compatibility qualification | Conformance between catalog claims, actual operations and fixtures, including preservation, runtime-unavailable, cancellation and refusal paths. | Generated descriptions and matching API shapes do not prove behavior. Support statements need tested producer/reader/query combinations. |

These responsibilities can be exposed by typed library methods; a server or
new SQL interface is not required. Suggested operation categories are describe
build, open inspection, read catalog, read detail/links, and execute a supported
inspection query. These are explanatory names, not new public API declarations
or parallel V1/V2 generations.

### Description file versus queryable facts

Prefer one programmatic source of truth with a bounded serializable description.
A generated JSON description packaged with an engine build can help tooling,
documentation and offline compatibility checks. It describes that build's
contract; it cannot describe another database's live readiness or prove which
producer wrote its contents. Contract identifiers need documented change rules,
not just arbitrary version strings.

Database-specific information should be queried through an admitted inspection
session from authoritative stored metadata and effective backend behavior.
Runtime prerequisites should remain separately unchecked, usable, missing or
failed until the appropriate safe check or explicit initialization occurs.
Request evidence belongs to the request result, not the description file.

A separately writable database sidecar would create another source of truth,
can become detached from backups/copies, and conflicts with the current
[wire contract's lack of a secondary version manifest](../../interfaces/wire.md#non-surface).
Do not introduce one here. If new persisted meaning really needs additional
metadata, design it within the database's authoritative format contract under
separate Engine/schema scope. Optional producer/version history would be audit
information, not a substitute for actual format identifiers or compatibility
qualification.

The practical priority is safe session/access first, a minimal queryable catalog
and build description alongside it, then evidence for the operations actually
supported. Shared tests must verify those claims. This would reduce Inspector's
coupling to private engine SQL while preserving FathomDB-specific views.

### Effect of the subsequent startup allowance

With Engine startup permitted, v0.1 may obtain the build description/catalog
from the running owned Engine where existing APIs suffice. A new Engine-free
inspection session is no longer a prerequisite. The main engine-side value is
a coherent queryable catalog and truthful operation evidence; a generated
static description is useful supporting tooling, not a required sidecar.
Cache, identity, observation and lifecycle constraints follow the current
requirements. New Engine API work remains a separately scoped proposal.
