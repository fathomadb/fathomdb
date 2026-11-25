---
title: Inspector requirements
date: 2026-11-23
product: FathomDB Inspector
target_release: 0.3.0
desc: Product requirements and adopted scope through FathomDB 0.8.32 / Schema 39
status: ACTIVE
---

# Inspector requirements

Current delivery scope targets FathomDB `0.8.32` (Schema `39`,
`TABLE_MANIFEST`-backed admission refusing Schema `38` and older) under
[Slice 30](../plans/0.8.32/features/slice-30/plan.md) (`R32-06`), building on
Inspector `0.3.0` ([Slice 60](../plans/0.8.31/features/slice-60/plan.md),
`R31-08` / `AC31-60-01`..`AC31-60-09`) and the completed
[owner-adopted 0.2.0](plans/0.2.0/scope.md) and `0.2.1` baselines. Functional
INS/V01/X, T-01–T-05, High investigation goals, `AC31-60-*`, and `R32-06`
requirements remain required; repo-level `dev/acceptance.md` stays locked. The
five-person study remains removed by owner decision
([INS-020-LOCAL-RELEASE](plans/0.2.0/release-decisions.md)); no human sessions
occurred or count as passed. Historical `0.2.0` / `0.2.1` source/design
evidence retains its actual version and execution scope.

## Conceptual distinctions and identity rules

These distinctions specify what the Inspector represents. Definitions belong
in documentation; the UI uses consistent labels, grouping, identity, and links.
The minimum coverage is grounded in the current contracts and source files
listed in the [high-level design](design/high-level-design.md#existing-mechanisms-and-new-work).

| Concept | Required distinction in presentation |
| --- | --- |
| Canonical node and edge | Separate record classes. An edge has endpoints and its own content and identity; a relationship is not merely a field on a node. |
| Kind and structural role | Kind classifies a canonical record. Node structural role is a separate axis; neither is an operational collection. |
| Operational collection | A registered namespace for operational records, identified within it by record key. |
| Operational collection modes | A latest-state collection retains current values; an append-only-log collection retains entries. Updating latest state does not automatically retain its old payload in the log. Equal keys in different collections do not establish a history relationship. |
| Canonical source and derived canonical record | Distinct records/revisions linked by recorded provenance or dependency. Derivation does not make the derived record a projection entry. |
| Projection | A representation associated with canonical content for retrieval; its declaration, materialization, and readiness are distinct facts. |
| Retrieval result | An observation produced by a request, with typed result identity, rank, and available origin evidence. It is not an independently authored database record. |
| Provenance completeness | Recorded `legacy` / `migrated_incomplete` metadata is distinct from complete source/derived provenance, including on current ordinary writes. Unknown linkage is not fabricated. |
| Engine ownership | System bookkeeping and structural role remain explicitly labeled; a canonical table location does not establish caller authorship. |
| Lifecycle, revision, and read context | Existence, supersession, validity, observed revision, and observation context are distinct; a result is not automatically a promise of current content. |

Identity rules:

- Qualify references by database and object class. Canonical nodes and edges,
  operational records, projection entries, and result observations are not one
  interchangeable identity space.
- Preserve the engine's typed search identity namespace and value: logical,
  content, or passage. A content identity does not establish a unique physical
  record, and a passage identity is not a canonical record locator.
- Preserve logical identity separately from revision evidence. A write cursor
  is an internal positional coordinate, not a new cross-session public ID.
- Operational identity includes collection and record key. Append-only entry
  identity additionally identifies the retained log entry. A log entry and a
  latest-state value in different collections are not versions of one record
  merely because their keys match.
- Equal bodies do not establish equal record identity. Different revisions of
  one logical entity remain distinguishable. A result that cannot be uniquely
  resolved to a stored record retains its observed payload and is labeled
  unresolved or ambiguous rather than guessed.
- A source identifier alone is not proof that the source body is retained as a
  canonical record. Navigation follows recorded links, not inferred text matches.

## Critical and high-level requirements

Critical means necessary for the initial Inspector to address the reported
problem. High means additional investigation depth, not optional release scope.
All INS-01 through INS-09 and the cross-cutting conditions are in v0.2.0.
Functional/browser and installed investigation checks cover Critical tasks
and High requirements. Human research is future optional work under the owner ruling. INS identifiers are local to
Inspector and do not add IDs to the locked global acceptance register.

| ID | Priority | High-level user need | Requirement | Traces to |
| --- | --- | --- | --- | --- |
| INS-01 | Critical | Inspect the database's organization. | The Inspector shall expose the structures and distinctions in the conceptual coverage table through consistent grouping, labels, and navigation to actual contents. | UN-02, UN-04, UN-14 |
| INS-02 | Critical | See what this database contains. | The Inspector shall present a scoped inventory of canonical records, relationships, operational collections, and projections, with access to their contents and observed fields, bounded stored display labels and unambiguous count units. | UN-01, UN-02, UN-03, UN-04, UN-08 |
| INS-03 | Critical | Locate and inspect an individual item or subset. | The Inspector shall support bounded browsing, identity lookup, available text/field/source constraints, complete record detail, and relationship navigation. | UN-05, UN-06, UN-07, UN-12 |
| INS-04 | Critical | Inspect a record's retrieval representations. | The Inspector shall expose recorded associations between canonical records, derived representations, projection declarations, and applicable retrieval paths. | UN-14, UN-15, UN-16 |
| INS-05 | Critical | Correlate results across retrieval methods. | The Inspector shall preserve typed identity, revision evidence, result payload, and request context across retrieval results and record navigation, and support pinning and comparing two bounded actual observations within the session. Typed native membership and verified same-exact-canonical-revision correspondence shall be separately labeled; neither merges the other. | UN-05, UN-08, UN-14, UN-16 |
| INS-06 | Critical | Distinguish storage from retrieval visibility. | The Inspector shall present storage, eligibility, projection readiness, and result selection as distinct observable facts with explicit unknown states. | UN-08, UN-13, UN-15, UN-16 |
| INS-07 | High | Investigate provenance, change, and dependencies. | Beyond core object links and revision distinction, the Inspector shall support bounded dependency tracing and retained revision-chain navigation. | UN-07, UN-12, UN-13, UN-14 |
| INS-09 | Critical | Navigate data in a form appropriate to its structure. | The Inspector shall provide coordinated document/structured-value, FTS/BM25, vector, and graph views, linked through verified record and representation identity. A generic SQL table grid alone does not satisfy this requirement. | UN-02, UN-04, UN-06, UN-07, UN-14, UN-16 |
| INS-08 | High | Investigate a particular retrieval outcome. | The Inspector shall expose available request and per-result execution evidence, and available record-specific readiness or eligibility evidence, without synthesizing unsupported causes for absence. Compact summaries shall identify each observation's own request, method, scope, time and evidence source alongside accessible exact evidence. | UN-13, UN-15, UN-16 |

INS-05 comparison preserves database/class/namespace/value/revision, exact
query or seed, options, filters, limits, measured population and observation
context. It shows membership, actual available rank and method-specific values
separately; graph membership has no invented rank. Equal text does not merge
identities; ambiguous origins remain unknown/choices. Capped, policy-filtered,
unsupported and unknown states are not absent rank zero or invented exclusion
causes. Separate observations do not imply a shared snapshot. D-01, refusal,
staleness and eviction apply within the existing observation/resource budgets;
pinning never implicitly re-executes, persists an experiment or broadens export.
Physical vector windows and native entries retain their own identity even when
canonical origins coincide. Representative retrieval arm is evidence, not identity.
Comparison pages contain at most50 entries; derived keys, DOM and controls share
the same ten-observation/20MiB owner as the original observations. Independent
source eviction may remove indexed/vector source detail while an admitted result
keeps its original request/context; neither event authorizes an automatic rerun.
Standalone original evidence remains subject to fresh policy/lifecycle checks
after unpin, including validity changes caused solely by wall-clock progress.

Record workspaces retain the exact permitted document, tree expansion, content
chunk and separate association/read contexts through Back, restoring initiating
keyboard focus. Explicit refresh creates a new observation. Restoring cached
content validates every displayed owner; a permitted anchor alone does not
permit its targets. Structural metadata and payload validation keep distinct
D-01 scopes. Stored presence and successful representation inspection establish
neither retrieval eligibility nor retrieval readiness; session runtime evidence
keeps its own scope and source.

UN-09 and UN-10 apply throughout. UN-02 is covered for registered structures,
record detail, and scoped observed fields; exhaustive schema inference is
deferred. UN-03 is covered for scoped counts and kind/collection summaries;
statistical distributions are deferred. UN-05 is covered for implemented,
declared constraints; arbitrary predicates and SQL are deferred. UN-11, UN-17,
and UN-18 retain only incidental coverage: comprehensive anomaly analysis,
persistent investigation workspaces, and general export workflows are deferred.
The owner-authorized narrow `--show-erased` export below is in v0.2.0.
Record-navigation context is required but does not imply persistent storage.
Critical scope includes direct recorded object/origin links and revision
distinction. INS-07 adds tracing beyond those direct links and navigation
through retained canonical revision chains; it does not promise overwritten
operational values.

## Inspector v0.2.0 scope commitment

The owner originally placed every Need and Should action from the
[Astra assessment](reviews/fable-action-assessment.md) in scope on 2026-10-28.
The assessment is historical evidence; this document and the plan own current
scope. No action below is optional because it was originally called Should.
The original Inspector views and INS-01 through INS-09 remain required.

| ID | Required outcome | Origin | Delivery stage |
| --- | --- | --- | --- |
| V01-N01 | Explicit operator-disclosure policy separating retained history/inactive data, owed physical erasure, and telemetry-redaction metadata. | Need 1 / Fable 1 | Policy before affected read contracts |
| V01-N02 | Faithful incomplete-provenance labels and current-write fixtures. | Need 2 / Fable 2 | Requirements, backend, fixtures |
| V01-N03 | Class-correct node and edge eligibility and lifecycle views. | Need 3 / Fable 4 | Read contracts and tests |
| V01-N04 | Metric, representation, candidate-population, selection-stage, and centering attribution for vector operations. | Need 4 / Fable 5 | Vector contract and UI |
| V01-N05 | Actual lexical index/fields, tokenizer, scoring route, and indexed-content attribution. | Need 5 / Fable 6 | Lexical contract and UI |
| V01-N06 | Current compiled schema only for initial attachment, with truthful older/newer refusal. | Need 6 / Fable 8 | Admission contract |
| V01-N07 | Verified cache-only embedding and optional reranker initialization without download or cache repair. | Need 7 / Fable 11 | Runtime implementation |
| V01-S01 | Labeled engine-owned operational data, structural role/provenance, and scoped count filters. | Should 1 / Fable 1, 3 | Inventory and detail |
| V01-S02 | Source-grounded MCP/agent-access reuse assessment; Engine-backed execution assessment, scratch/connection ownership and truthful cancellation. | Should 2 / Fable 7 | Architecture and execution seam |
| V01-S03 | Actionable non-destructive WAL/sidecar refusal guidance. | Should 3 / Fable 9 | Admission UI and documentation |
| V01-S04 | Automated graph/evidence/metric checks and actual browser checks for selection, expansion/collapse, and coordinated navigation. | Should 4 / Fable 10 | Acceptance and qualification |

V01-N01 is governed by the adopted D-01 policy below, explicitly ruled by the
owner on 2026-10-30. No requirement permits
reconstructing erased content, presenting telemetry bookkeeping as retained
canonical payload, or silently substituting ordinary application eligibility
for operator-disclosure policy.

### Engine startup allowance (2026-10-29)

Engine startup is permitted when required for v0.2.0 inspection. Prefer usable
unchanged Engine APIs over recreating their semantics in an immutable adapter.
Inspector exposes inspection/read operations, not edit, actuation, projection
configuration, repair or migration commands. Normal Engine-owned startup,
shutdown, lock/WAL handling and background maintenance/projection effects may
change product files or materialization state; this mode does not promise
byte preservation or an immutable view of the database as found. Identify the
mode and its possible effects before attachment; qualify the actual selected
Engine lifecycle and options. Do not trigger additional rebuild/drain/repair
operations merely to make inspection succeed.

The initial scope remains one existing local current-schema database with all
external writers stopped. The Inspector-owned Engine may run its own workers;
it owns the product lock rather than competing with a second Inspector lock.
An already-running application, initially nonempty WAL/journal, older schema,
and forensic immutable attachment remain outside the baseline. The actual
schema derives from the selected build. Engine-created WAL after admitted
startup is normal lifecycle state, not an invalidation event.

Cache-only/no-download/no-repair query-model constraints and disclosure policy
remain in force. Cached model initialization may happen during startup where
required; default loading that could download is not automatically acceptable.
Observations must carry actual read/generation context because Engine work can
progress during the session. This clarification supersedes earlier absolute
no-Engine-startup and byte-preservation requirements; it authorizes no Engine,
SDK, MCP or schema code changes.

A vector neighborhood must state whether it uses the engine's bounded binary
candidate selection followed by f32 L2 reranking, or a separately specified
exact operation. Exact distances within a selected set do not imply globally
exact nearest neighbors. A lexical inventory must distinguish actual supported
routes from historical schema declarations. Neither attribution depends on
inventing new public query controls.

### Native lexical observation contract (Slice30)

Body/edge text retrieval returns native RRF values. Named-property retrieval
returns Engine negated SQLite BM25 values (higher better), distinct from raw
SQLite BM25 ordering. BM25F substrate is an explicit separate operation with
native active/current/dependency eligibility; it ignores own-node validity
windows and offers no native filter/view/limit. Incompatible controls refuse
before execution. Bounded Inspector output truncation is not a native bound.

Original query payload windows retain exact original byte counts/completeness;
full later detail is a separate observation. Actual same-reader body snippets
retain UTF8 byte ranges/anchors; absent property/BM25F spans stay unavailable.
Stored indexed text/configured declarations/effective schema tokenizer and later
exact-origin/policy reads have their own scope, never an invented query snapshot.
Native allocation and relaxed scratch amplification require bounded admission;
resource refusal is explicit and does not waive positive lexical functionality.

### FathomDB 0.8.32 contract additions (Schema 39 & active-`scope` UI)

FathomDB `0.8.32` Slice 30 (`R32-06`) upgrades Inspector onto Schema `39` while
preserving all `0.2.x` and `0.3.0` contracts:

- **Schema 39 `TABLE_MANIFEST`-backed admission:** Admit only Schema `39`
  (`compiled_schema = 39`), iterating `fathomdb_schema::TABLE_MANIFEST` as the
  single source of truth while keeping per-table column validation and the
  8-column `vec0` (`scope TEXT`) `malformed_vector_shape` guard. Schema `38`
  (`0.8.31`) and older or missing-layout stores refuse cleanly without mutating
  files.
- **Active-`scope` UI & multi-scope isolation subordinate to `D-01`:** Discover
  visible scopes in `catalog().scopes` strictly over rows permitted by the
  active disclosure policy (`--no-show-inactive`, default erased/telemetry
  hiding): a scope whose only rows are inactive, deleted, or erased-pending
  must never appear in `catalog().scopes`, counts, or UI selectors. Keep
  single-scope `""` stores uncluttered; expose an active-`scope` switcher in
  Inventory, index-backed scoped `page` filters (Schema 39 composite indexes
  with zero full table `SCAN` and zero `TEMP B-TREE`), per-scope native Engine
  reopening (`Session::ensure_engine_scope`) for lexical/serving/graph
  traversals, and scope-qualified SQL joins across `associations`,
  `investigate`, `graph`, `lexical`, `serving`, and `neighborhood`.
- **Hosted CI browser smoke (`inspector-browser-smoke`):** Run Inspector
  tooling tests, JS unit tests, and live headless Playwright smoke (`api.mjs`,
  `ux.mjs`, `scope.mjs`) on Inspector-scoped changes and release qualification
  in `.github/workflows/ci.yml`.

### Inspector 0.3.0 / FathomDB 0.8.31 contract additions (Schema 38, carried forward)

Inspector `0.3.0` (`R31-08`, `AC31-60-01`..`AC31-60-09`) extends the adopted
`0.2.x` baseline onto FathomDB `0.8.31` (Schema `38`) without modifying
`dev/acceptance.md`:

- **Schema 38 strict admission (`AC31-60-01`):** Admit only Schema `38`
  (`compiled_schema = 38`), validating `properties` and `scope` on
  `canonical_nodes` / `canonical_edges`, `scope` on `operational_mutations` /
  `operational_state`, `_fathomdb_projection_registry.{entity_class,vector_kinds}`,
  `canonical_property_values`, `_fathomdb_commit_timestamps`, and
  `_fathomdb_vector_kinds`. Schema `37` (`0.8.30`) and older or missing-layout
  stores refuse cleanly without mutating the file.
- **Outside-`body` `properties` & reserved `scope` (`AC31-60-02`, `AC31-60-03`):**
  Inspect, expand (`detail_field`, `chunk`, `tree` with explicit `field`
  threaded through `drawTree()`), and exactly copy outside-`body` node and edge
  `properties` (preserving 64-bit integer lexemes outside `2^53`, `-0`, empty
  strings, typed primitives, and `SQL NULL`) alongside plain-text `body`
  documents, while surfacing reserved `scope` read-only (empty in `0.8.31`).
  Include `"properties"` in the 64 MiB `graph_admission()` /
  `lexical_admission()` scans and surface bounded `canonical_property_values`
  for both nodes and edges in `associations` and `catalog`. Render every key,
  value, and `scope` purely via DOM `.textContent` (zero HTML/SVG/script
  injection), and withhold `properties`, `properties_bytes`, and
  `typeof(properties)` whenever `!policy.permits(class)` across all 8 `D-01`
  flag combinations.
- **Historical `as_of` graph exploration & 8-flag subordination (`AC31-60-04`):**
  Support mutually exclusive `as_of_write_cursor` and `as_of_epoch_ms`
  (`Option<String>` wire encoding, resolved via O(log N) reverse seek over
  `_fathomdb_commit_timestamps`) for both stored-adjacency (`graph`) and native
  engine (`graph_serving` via `GraphExpandRequestV1::with_as_of_{write_cursor,epoch_ms}`)
  traversals. Lift `INDEXED BY canonical_nodes_logical_active_idx` in
  historical mode using two-branch `UNION ALL` over Schema 38's
  `canonical_nodes_logical_{active,sup}_idx`, preserve non-partial
  `canonical_edges_{from_id,to_id}_idx` in current mode when `!no_show_inactive`,
  retain `as_of` parameters across comparison pins, and subordinate every
  historical target, edge, origin, and inline property payload to all 8 `D-01`
  disclosure flags.
- **Scoped local CI, bounded timeouts, `D4 = B` lock & GitHub Release delivery
  (`AC31-60-05`..`AC31-60-09`):** Default `src/inspector/tools/local-ci.sh` to
  `--scoped` (~5 min Inspector-only source check with fail-closed diff guard,
  `--base <ref>` / `INSPECTOR_SCOPED_BASE`, and mutually exclusive
  `--full-workspace` / `--installed-only` modes); enforce bounded timeouts and
  non-blocking `SIGKILL` + `/proc` descendant reaping (`P7-02`) across shell,
  Node/Playwright, and Rust `launch.rs` harnesses; retain `D4 = B` Docker-free
  Ubuntu 24.04 (`glibc 2.39`) Chromium + bundled SQLite qualification; and
  deliver `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz` plus
  `fathomdb-inspector-0.3.0-linux-x86_64.tar.gz.sha256` via the `build-inspector`
  job in `.github/workflows/release.yml` (gated by `all-builds-passed` and
  attached in `github-release`).

## Document ownership

- [Needs](needs.md) owns user outcomes and research.
- This file owns requirements and v0.2.0 action scope.
- [Acceptance](acceptance.md) owns INS/V01 acceptance signals and fixtures.
- [Design](design/high-level-design.md) owns the selected S16 architecture.
- [Adopted scope](plans/0.2.0/scope.md) owns the release boundary; the
  [legacy allocation](plans/plan-0.1.md) preserves draft sequencing/history.
- [ROADMAP](ROADMAP.md) owns work outside v0.2.0.

FathomDB's accepted ADRs and public interfaces remain authoritative for engine
contracts. Inspector-local IDs do not add rows to the locked FathomDB global
acceptance register.

## Inspector-only execution allocation

Current delivery is allocated by [plans/0.2.0/plan.md](plans/0.2.0/plan.md):
20 foundation, 30 lexical, 40 vectors, 50 graph/history, 60 comparison/person
interaction, 70 installed/integrated qualification. Original 15/16 and 0.1
statuses remain historical evidence. All feature slices consume the shared
baselines and retain slice-local correctness/testing obligations. Existing
Engine, schema, SDK, MCP and agent-access implementations/contracts are
read-only dependencies/reference material, not change targets. Reuse assessment
under V01-S02 does not authorize work in those projects. Missing required
capabilities must be reported as blockers without silently changing scope.

## Verification and operating envelope (Slice 15, 2026-10-30)

The [shared verification plan](plans/0.1/verification-plan.md) is the single
method/fixture/ownership register. Criteria remain in acceptance.md; planned
verification is not evidence of working software. All required positive
capabilities must work on their supported fixtures. A missing Engine interface,
model or participant cannot convert a required test into a pass.

### Initial qualification target

Use one bundled local UI/service with one pinned unchanged Engine/query build,
its current compiled schema and a CPU-only baseline. Slice 16 selects the exact
build/tool/browser/package versions and proves feasible routes in the historical
architecture baseline; current Slice 70 qualifies installed artifacts. Initial engineering target: Linux x86_64,
Ubuntu 22.04 or newer, Chromium desktop, keyboard and pointer, four CPU cores,
8 GiB available RAM and free scratch disk at least twice the fixture size.
This is a qualification target, not an already supported artifact or an owner
commitment to every newer distribution/browser. Record the exact measured OS,
browser and CPU in the eventual support statement. No CUDA/model service,
Internet access, installer publication, historical producer or second platform
is required. Existing cached CPU model assets are prerequisites for positive
model-backed tests; absence must leave independent record/text browsing usable.

F-06's minimum scale is mandatory: 100,000 canonical revisions, 100,000 retained
mutation entries, 10 MiB content and 10,000 directed incident edges. Smaller
fixtures prove semantics; the scale fixture proves bounded work, not global
statistical coverage. Engineering budgets below are acceptance targets until
measured. Slice 16 must demonstrate a credible way to enforce them or record
the concrete infeasibility and revise the affected contract with review before
implementation; no unsupported claim or silent weakening is permitted.

| Boundary | Initial testable budget |
| --- | --- |
| Pages and response | At most 200 items and 2 MiB serialized bytes per response; content chunks at most 256 KiB with exact continuation. Oversize scalar/identity handling preserves exact value via bounded reads/copy. |
| Inventory/count/sample | At most 100,000 examined rows per operation and 30 s total; reached cap returns partial/unknown count with examined scope. No per-table timeout masquerades as a total bound. |
| Graph/trace | At most 500 displayed node+edge objects, 1,000 examined links and depth 3 per expansion/trace request; expose continuation/truncation. Collapse releases hidden expansion state. |
| Concurrency and retained UI state | One executing costly request, at most eight waiting; reject excess with busy, never accumulate abandoned work. Retain at most ten result observations/20 MiB response data; announce eviction, preserve selected locator/context without silently requerying. |
| Timing | Attach/request deadline 30 s; cancel acknowledgement within 250 ms; quiescence/detach/shutdown within 30 s after cancel/close. UI cancellation need not interrupt a native call, but aggregate in-flight work remains capped and late data is discarded. |
| Memory | Service RSS growth over its idle attached baseline at most 1 GiB excluding separately measured cached-model residency (at most 2 GiB); browser heap growth at most 512 MiB over shell idle on F-06. Record peak total RSS too. No whole database/payload/graph preload. |

Finite shutdown is a required positive behavior. If an unchanged native call
cannot be bounded in-process, Slice 16 must choose feasible ownership/isolation
or mark the route blocked. Do not interrupt another Engine/session's connection
or claim that dropping a response terminates execution. Timings use monotonic
clocks, scripted interaction milestones and declared machine load; throughput,
rank stability under ties and broad hardware comparisons are diagnostic only.

### Lifecycle effect classes and observation

Before Engine open, invalid/missing/empty paths, held/missing lock prerequisites,
noncurrent schema and initially nonempty WAL/journal refuse without changing
product or cache files. Use bounded admission, not a database-wide integrity
claim. Once admitted, allow only effects attributable to the qualified unchanged
Engine open/options, ordinary workers, supported read/query operations and
shutdown: lock metadata, WAL/shared-memory lifecycle, existing generation/
projection/closure maintenance, and configured Engine telemetry bookkeeping.
Inspect no payload in an unsafe interval to make a fixture pass. No Inspector
configure/write/rebuild/drain/repair/migration/bootstrap call is allowed.
Startup failure after open may have those effects and must disclose that fact.

The allowlist is semantic, not “anything an equivalent Engine does”: canonical
caller content/identity must not be edited by Inspector; worker materialization
and owed closure must conform to predeclared fixture obligations; no new schema
or download/cache write is authorized. Compare intended pre/post facts and
observed event classes first, equivalent Engine runs second for attribution.
Record effects separately for preflight, startup, each request and shutdown;
worker order/timing need not match. The selected worker environment is an
explicit part of the qualified composition: ambient experimental dependency
settings must not silently alter product PRAGMAs, query candidate populations,
CPU/model selection or recorded effect scope. Inspector owns the cleared child
environment; the canonical HLD specifies its allowlist and browser separation. Normal Engine generations change observation
context, not database identity. Detectable replacement/ownership loss expires
references; separate reads never claim an unobserved shared snapshot.

### Local service, values and access

The bundled service binds loopback only, accepts only its issued local session
credential and allowed UI origin/Host, and rejects stale build/session/reference
pairs before reading. Untrusted origins, cross-site requests, guessed/absent
credentials and remote listeners must fail. Stored HTML, scripts, URLs and
markup render as inert content; do not auto-fetch attachments or remote links.
Credentials, payloads, query text and erased identifiers are absent from URLs,
ordinary logs and crash/error messages. No persistent investigation history is
introduced. Narrow erasure-pending export follows D-01 below. Clipboard actions are explicit and copy exact underlying values.
Keyboard operation covers document/tree, lexical, vector and graph selection,
expansion/collapse, links and return navigation, with visible focus and equivalent
labeled facts. Missing/null/empty/unknown/error/cancelled/partial remain distinct.

Installed launch/attach/all-view/close and documentation-assisted functional
T-01–T-05/High investigation checks are required. The former five-person study
and four-of-five diagnostic are historical, unperformed research; neither is a
0.2.0 gate. Slice80 additionally requires tested local CI, accurate installation/
developer documentation, compatible packages with source/build/checksum identity,
and collision-safe promotion; Slice90 performs the local-only dist release.
See the shared verification plan and approved80/90 plans for methods.

## Adopted operator-disclosure policy — D-01 (2026-10-30)

The owner explicitly ruled the following defaults and exact launch flags.
This is adopted v0.2.0 scope, not a proposal or an extra approval/confirmation UI.
Normally accessible data is shown by default. Retained history/inactive data
is also viewable by default; operator disclosure is not identical to ordinary
retrieval eligibility. A flag permits disclosure, not fabricated retrieval
membership, ranks, retention or erased-content reconstruction.

| Data class | Default | Launch override |
| --- | --- | --- |
| Normally accessible data | Show under its actual observation/retrieval scope. | None required. |
| Retained history and inactive records | Allow viewing retained records, including superseded canonical revisions and class-correct inactive/expired records. | `--no-show-inactive` hides these records. |
| Content physically retained and awaiting physical erasure | Refuse display of that content; withhold its copies, snippets, derived content and export too. Do not refuse unrelated safe inspection. | `--show-erased` permits display and explicit local export of this still-retained content, subject to other applicable gates. |
| Telemetry bookkeeping | Show structure and scoped counts without payloads. | `--show-payloads` makes telemetry payloads accessible, subject to other applicable gates. |
| Content already physically absent/erased | Unavailable; no reconstruction from other representations, text matches or telemetry identifiers. | No flag can restore absent content or guarantee that a worker has not already erased it. |

The launch policy is fixed for that application/session. Default invocation
omits all three flags; each supplied switch enables its named behavior. All
three may appear together in any order; repeated identical switches are
idempotent. Unknown switches/invalid values refuse before Engine open. Binary
name, path syntax and package wiring are Slice 16 design; these exact flags
and defaults are already settled and must appear in launch help and tests.
No reverse switch or additional confirmation requirement is introduced here.
Expose the active policy as factual session state; UI navigation cannot elevate
it. A new invocation/session is required to change it.

### Overlap and path enforcement

Treat classifications as independent gates, not priority-based exceptions.
For an observed record/payload let `I` mean known retained-history/inactive,
`E` mean its content is physically retained but owed physical erasure, and `T`
mean telemetry payload. An otherwise permitted content read/export requires:

```text
(not I or not --no-show-inactive)
and (not E or --show-erased)
and (not T or --show-payloads)
```

The first gate hides the record; a permitted erasure or telemetry flag does
not override it. The second permits owed-erasure content only; it does not
unlock telemetry payloads. The third permits telemetry payloads only; it does
not unlock owed-erasure content or hidden inactive records. Telemetry structure/
aggregate counts can remain visible without payloads, but count scope must
exclude records hidden by `--no-show-inactive` and not disclose payload values.
Append-only collection entries are not classified inactive/history solely by
age: use actual retained-history/lifecycle facts, not guessed semantics.
Node/edge temporal axes remain class-correct. Unknown disclosure classification
must withhold the affected content with an explicit unknown/refusal state until
it can be established; it is not proof of deletion or of permission.

Apply the same gates before every service response, detail/chunk, index/vector
representation, graph label/edge, source/dependency/history link, result body,
match snippet/query evidence, clipboard copy and export. Hidden content cannot
leak through an alternate route or a cached/earlier observation. Counts and
links state their policy-filtered scope; links to withheld content show a safe
unavailable/refused state rather than returning its payload. Ordinary retrieval
keeps its declared Engine eligibility/selection semantics: a flag never causes
ineligible records to acquire a false observed score/rank. When Engine work
changes a content/closure classification, a new delivery/copy/export rechecks
that context; expired original content remains unavailable. Previously shown
bytes cannot be retroactively unobserved, and Inspector never claims erasure
of a user's existing clipboard/export file. Logs, errors and URLs remain free
of inspected payloads/query text/session credentials even when display is allowed.

### Narrow v0.2.0 export

The 2026-10-31 owner ruling resolves B16-01 and qualifies the original
2026-10-30 positive-fixture wording below; D-01 defaults/flags are unchanged.

`--show-erased` explicitly authorizes export of selected still-retained content
awaiting physical erasure, after all applicable gates. This is the only added
export workflow; general result export/share, saved investigations and bulk
archival remain deferred. An export preserves the exact selected value/content
and its database/class/identity/revision/observation scope where disclosable;
no formatter silently changes numeric/text values. Use bounded local streaming
under the existing byte/work/cancel budgets and an explicit user-selected local
destination. No network send, broad database dump, implicit export, database
mutation or additional confirmation UI is required. Failed/cancelled/partial
exports are not labeled complete. Export checks availability and policy at
read time; disappearance from Engine closure work is unavailable, not a reason
to rebuild, delay erasure or reconstruct data. The owner clarified on
2026-10-31: **only if it exists**. Physical deletion is correct; this requirement
does not request retention, a pending-erasure queue, new Engine read APIs or
forensic reconstruction. Positive Engine display/export is required only when
a qualified supported Engine legitimately exposes readable retained-erasure
content. An Autonomous Engine that atomically deletes content has no such
retained target: report unavailable/not-applicable for that content. The current
0.8.29 composition also honors existing ExplicitMaintenance stores: queued intent
retains its exact frozen cursor scope without logical removal. Legitimately
retained targets in that scope obey the erasure-content gate and narrow export;
queuing does not itself make an active record inactive. Inspector neither queues
requests nor changes persisted policy, activates maintenance, delays deletion or
reconstructs bytes. See the [0.8.29 qualification scope](plans/0.1/compatibility-0.8.29-port.md).
Pure typed policy/export contract tests exercise supplied content and independent
intent across all gates; they do not mock a database or fabricate a durable
Engine state. Real Engine integration verifies atomic deletion, absence, all
flag combinations and no reconstruction. Other required positive native views
and available-data behavior remain mandatory.
