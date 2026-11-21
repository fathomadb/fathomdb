---
title: FathomDB Inspector — high-level design
date: 2026-10-30
product: FathomDB Inspector
target_release: 0.2.0
desc: Inspector-owned composition, operation contracts, capability evidence and evolution boundaries
status: ACTIVE
blast_radius: Inspector-owned composition and feature handoffs; dependency source remains unchanged
---

# FathomDB Inspector — high-level design

Current delivery scope is [owner-adopted 0.2.0](../plans/0.2.0/scope.md). Original v0.1
mandatory obligations and remaining qualification carry forward unchanged;
historical source/design baselines and V01 identifiers retain their meaning.
The unpublished 0.1.0 port is the record foundation, not full-view completion.
This scope adoption is not approval of future detailed designs or execution.

## Current0.2.0 execution and release design

The implemented record/lexical/vector/graph/provenance/comparison paths are
qualified through20–60,70 installed technical checks and75 UX remediation.
Earlier architecture feasibility tables below retain their baseline evidence.
[INS-020-LOCAL-RELEASE](../plans/0.2.0/release-decisions.md) removes the unperformed
human-study requirement; T-01–T-05 and High functional goals remain.
[80 design](../plans/0.2.0/slice-80/design.md) owns local CI, compatible packaging,
source/build/checksum binding and atomic no-overwrite local promotion.
[90 design](../plans/0.2.0/slice-90/design.md) owns execution into ignored dist.
No Engine contract, pin or schema changes are introduced.

## Authority and selected baseline

Architecture baseline **S16-2026-10-31** consumes completed
[S15-2026-10-30](../plans/0.1/slice-15/status.md), canonical
[requirements](../requirements.md), [acceptance](../acceptance.md) and the
[single verification matrix](../plans/0.1/verification-plan.md). This document
owns architecture; feature designs refine it, rather than define competing
session, identity or lifecycle contracts. Source feasibility is distinct from
implementation and runtime qualification. Current implementation and executed evidence are tracked in [Slice 20 status](../plans/0.1/slice-20/status.md); this architecture record does not itself establish a runtime pass.
**B16-01 resolved by the owner's 2026-10-31 “only if it exists” ruling.**
Physical deletion is correct and unchanged. Retained-erasure display/export is
conditional on legitimate readable content; absence in the selected Engine is
unavailable/not-applicable, not an architecture blocker. See the
[resolution and source evidence](#retained-erasure-scope--b16-01-resolved).

The original S16 source baseline was `728884bfe` (package 0.8.27/schema 34).
This checkout ports Inspector snapshot `7d3637657` onto unchanged `dev/0.8.29`
source `6edc1155f039f87cd0a37660cb27ae4373bbce71` (workspace packages 0.8.28,
compiled schema 37) with that base's existing dependency pins and Rust 1.95.
The [port record](../plans/0.1/compatibility-0.8.29-port.md) owns current
compatibility checks; historical reviews and statuses retain their source scope.
The current composition reads format 37 only, not historical schema-34/35 files.
Use exact format-37 layout facts, clean visibility coalescing and actual queued
request cursor membership. A schema number alone does not qualify a composition.

The original S16/current-source artifact identity below is historical 0.1.0;
0.2.0 packaging remains to qualify without weakening the 22.04 target. The new
INS-05.7–05.10/M40 comparison requires detailed design; S16 does not approve it.
Choose one Linux x86_64 offline tar artifact, `fathomdb-inspector-0.1.0-linux-x86_64`:
a Rust executable with supervisor, HTTP shell and internal worker modes, plus
bundled HTML/CSS/JavaScript modules. Use standard browser DOM, SVG and ES modules;
no UI framework, graph library, bundler, Node runtime, Python binding, MCP server
or development server is needed by the installed product. Build graph layout
and coordinated interaction inside Inspector. Reuse locked Rust dependencies
(serde/serde_json, rusqlite 0.41, sqlite-vec 0.1.9, sha2); no upstream pin change
is needed. Implement bounded HTTP framing in the Inspector shell using std TCP,
with explicit parser limits and protocol tests, rather than add a server stack.

Qualification target: Ubuntu 22.04 x86_64, CPU only, Chrome for Testing
156.0.8078.4 (Playwright browser build 1248 locally available), Playwright 1.62.1 for
browser automation. At S16 these exact browser/tool binaries were inventoried; actual feature
execution is now recorded in the Slice 20 status. Slice 20 records hashes of the actual build
and test tools; Slice 60 records OS/CPU/browser and installed artifact hashes.
A changed browser or dependency tree requires relevant requalification, not an
automatic support expansion. Use Rust tests and JavaScript `node:test` for
contracts, Playwright for actual browser/keyboard checks. Test-only Node tools
are excluded from the artifact. No files/pins are changed by this design.

Launch contract: `fathomdb-inspector [--no-show-inactive] [--show-erased]
[--show-payloads] [--] DATABASE_PATH`; omitted path opens the local selector.
`--help` describes defaults, Engine effects and the three exact switches.
Repeated switches are idempotent; unknown flags/values refuse before open.
Path selection never appears in a URL. Local selected-path submission is
credentialed and is not a remote filesystem browsing API. Policy is fixed for
the launch, including reattachments; changing it requires a new invocation.

## Components and ownership

```text
Browser shell and coordinated views
    ↕ same-origin credentialed typed HTTP, bundled build pair
Supervisor: protocol + admission queue + policy + observation registry
    ↕ bounded inherited IPC, never exposed as HTTP operations
Session child: sole Engine owner + serialized query adapter
    ↔ Inspector request-owned read-only SQLite connections
    ↔ in-process CPU Default model over retained sealed cached assets
```

These are modules and modes of one product, not plugins or separately released
services. Names used below:

| Component | Responsibility and boundary |
| --- | --- |
| A — admission/session | Path/header/sidecar/schema preflight; effect disclosure before explicit attach; child lifecycle, file identity and session expiry. |
| S — storage reader | Named current-layout catalog, exact records, bounded values, projections, origins, adjacency and revision links. No generic SQL endpoint. |
| Q — query adapter | Public unchanged Engine operations and their returned evidence; no copied private ranking, tokenizer, fusion or eligibility implementation. |
| P — policy | Independent D-01 gates across all deliveries, reference creation, copy and narrow export. Unknown classification withholds affected content. |
| O — observations | Typed references, request/result context, exact resolution, bounded history and eviction. |
| R — resource supervisor | Queue, monotonic deadlines, cancellation, child/helper ownership, byte/work/memory accounting and teardown. |
| U — views | Structure navigator, document/tree, lexical, metric bars, directed SVG graph, associations and comparison; accessible selection and back context. |
| K — package/protocol | Bundled build identity, static assets, local authentication, installed lifecycle and fixtures/tools. |

Supervisor holds no Engine handle. The session child owns the Engine, every
request connection, scratch and sealed model resource; no external Engine/app shares the
checkout's database. Engine owns the product lock. Cooperative external writers
must stop, including raw SQLite tools; the lock cannot exclude uncooperative
writers. No new Inspector lock competes with Engine's lock. No HTTP route can
invoke write, actuation, configure, rebuild, drain, repair, migration or bootstrap.
Fixture builders may use existing writers; that is not a product route.

## Existing mechanisms and new work

### Dependency capability map

Current source implements the Slice20 record foundation, Slice30 native lexical,
Slice40 vector/cached CPU and Slice50 graph/history routes. Per-slice statuses
record executed evidence; installed technical qualification completed70/75,
while the unperformed human study is removed. Historical S16 source findings, including missing snippet APIs,
are not assertions that those APIs remain absent on the newer 0.8.29 baseline.

All relative source links below name actual source in the selected tree.
**Public** means callable Rust API, not necessarily a stable end-user Inspector
contract. **Private** means a source fact the Inspector may understand through
its own current-layout reads, never an imported private module. **Test-only**
means fixture/conformance access only. **New** means Inspector work still to
implement/test. Prerequisites and evidence limits are part of each route.

| Required capability / owner | Existing callable or storage mechanism | Selected Inspector route, prerequisites and limit |
| --- | --- | --- |
| Admission/lifecycle / 20 | Public `Engine::open_with_choice_and_config`, `Engine::close`; private admission in [open.rs](../../../src/rust/crates/fathomdb-engine/src/open.rs), [runtime_lifecycle.rs](../../../src/rust/crates/fathomdb-engine/src/runtime_lifecycle.rs). | A/R child ownership and preflight described below. Open may reconcile projections/workers; it is not immutable inspection. No general Engine interrupt API exists. |
| Catalog/count/sample / 20 | Public collection and projection reads; [schema](../../../src/rust/crates/fathomdb-schema/src/lib.rs), `read_projections`, [read_api.rs](../../../src/rust/crates/fathomdb-engine/src/read_api.rs). | S named allowlisted table/column reads with examined-row counters, policy filters and byte caps. Distinguish canonical revisions/logical entities, collections/log/state, registry, generation and physical materialization. No exhaustive field inference. |
| Exact retained records/full edges / 20,50 | Public `read_get` chooses latest matching cursor; `graph_neighbors` returns nodes. Canonical rows + artifact revisions in schema; [read.rs](../../../src/rust/crates/fathomdb-engine/src/read.rs). | S exact `(artifact_class, write_cursor)` joined to recorded revision ID where present, full edge body/endpoints/properties, attributes and bounded chunks. Never substitute logical-ID point lookup for an exact revision. Missing registry row retains session-local physical locator and explicitly absent revision evidence. |
| Operational modes / 20 | Public `read_collection`, `read_mutations`, `read_operational_state_page`; [write_commit.rs](../../../src/rust/crates/fathomdb-engine/src/write_commit.rs). | S collection+key current locator versus collection+entry cursor log locator. Latest replacement does not retain prior body; no inferred cross-collection history. |
| Projection association/readiness / 20,30,40 | Public `read_projections`, `read_projection_generation_status`, `read_mutation_projection_status`, `read_embedding_readiness`; [registry](../../../src/rust/crates/fathomdb-engine/src/projection_registry.rs), [generation](../../../src/rust/crates/fathomdb-engine/src/projection_generation.rs). | S recorded attribute/vector-row/FTS cursor mappings plus Q returned generation statuses. Registry declaration ≠ materialization ≠ query runtime. Inspect row-specific state; no database-wide ready proof for one origin. |
| Lexical serving / 30 | Public `search_text_only_view_with_limit`, `search_projected_text_with_limit`, doc-hidden but callable `bm25f_search`; [search_api.rs](../../../src/rust/crates/fathomdb-engine/src/search_api.rs), [search.rs](../../../src/rust/crates/fathomdb-engine/src/search.rs). | Slice30 implements Q body/edge lexical route and declared property-FTS route; explicit BM25F substrate operation if chosen by user, not ordinary serving search. Engine owns compilation/tokenization/ranking including temporary FTS scratch. S exposes actual indexed fields/text. BM25F cursor-only output is hydrated as a later scoped observation, never attributed an absent snapshot/evidence carrier. |
| Lexical evidence / 30 | Body/edge/property FTS5 tables and effective `sqlite_master` tokenizer declarations. Selected source exposes public filtered/view/frozen/reranked snippet routes; projected text and BM25F expose no snippet option. | Prefer actual returned same-read evidence; qualify route-specific evidence in 30. S bounded `highlight`/`snippet` on the selected FTS row with the same MATCH expression is a separately labeled index-match observation when actually executed, with its own context. Otherwise show indexed text and evidence unavailable. Do not infer spans using a second home-grown tokenizer. Required known-match fixture uses actual FTS5 evidence. |
| Hybrid/ranking evidence / 30,40,60 | Public `search_explained_with_limit`, `search_with_evidence`, `search_frozen`, frozen/evidence resolvers and typed SearchResult; [evidence.rs](../../../src/rust/crates/fathomdb-engine/src/evidence.rs), [search_types.rs](../../../src/rust/crates/fathomdb-engine/src/search_types.rs). | Q preserves actual options/results/sidecar, representative arm, scores/ranks and structural state. Use matching frozen APIs for point resolver claims. Use only source-qualified snippet/evidence contracts; absent degradation metadata remains unknown. Missing evidence is unknown, never an inferred omission cause. |
| Stored vectors/neighborhood / 40 | Private [vector_storage.rs](../../../src/rust/crates/fathomdb-engine/src/vector_storage.rs) defines vec0 f32+binary storage and row sidecar; unchanged sqlite-vec functions are callable on owned connection. Public normal search gives fused scores, not distances. Current producers/admission enforce vector rowid=source cursor, window 0/start 0; recorded byte_end/body_length may leave coverage unknown. S9-010 qualifies this supported single head window per revision and defers within-source multiple-window positive. | S separate exact L2 operation over an explicitly bounded, policy-visible stored population (below), same f32 representation and lower-is-nearer L2 as Engine phase 2. This is not an Engine serving vector-only query, global ANN or hybrid rank. No private candidate/eligibility engine is copied. |
| Cache-only CPU query model / 20,40 | Production Default/None; unchanged loader accepts pinned symlinks and current public frozen search returns actual evidence. |40 uses sealed-memfd immutable asset view plus LandlockABI3/seccomp in the existing owned child; protected SQLite authorizer forbids persisted dense/probe byte writes. Existing45-probe baseline/materialization is required. Actual CPU dense positive and missing/corrupt/replaced/raced cases are qualified in [40 status](../plans/0.2.0/slice-40/status.md). No Caller, separate provider helper, fetch/repair or unsafe Default fallback. |
| Optional reranker / 40 | `CandleTinyBertReranker::try_load` may fetch; no selected public cache-only constructor. | Do not compile `default-reranker`; report unsupported optional CE, not a failed required vector operation. If later enabled, a separately reviewed no-fetch route is prerequisite; no default loader preprobe TOCTOU workaround. |
| Graph / 50 | Public `graph_neighbors`/graph expand and graph evidence; [graph_api.rs](../../../src/rust/crates/fathomdb-engine/src/graph_api.rs), canonical_edges. | S complete stored directed adjacency/edge detail with caps; Q for serving traversal evidence where requested. Stored adjacency explicitly includes retained inactive history unless hidden. It never claims Engine serving eligibility by membership alone. |
| Provenance/history/closure / 50 | Public `trace_dependency`, `read_dependency_closure`; artifact/source/dependency/closure schema; [dependency_trace.rs](../../../src/rust/crates/fathomdb-engine/src/dependency_trace.rs), [dependency_closure.rs](../../../src/rust/crates/fathomdb-engine/src/dependency_closure.rs). | Q bounded trace where compatible; S exact retained revision/source links with zero/one/many resolution and class-specific lifecycle facts. No source-body guarantee from source_id; no overwritten operational payload reconstruction. |
| Disclosure/telemetry/export / 20,50 | Stored closure/provenance and operational bookkeeping; [erasure.rs](../../../src/rust/crates/fathomdb-engine/src/erasure.rs). Pending telemetry redaction contains identifiers, not retained canonical body proof. | P classifies from actual authoritative links and closure state; S content exists check; narrow bounded stream to selected local destination. Independent gates compose, including on stale observations and alternate representations. Unknown link/classification refuses affected content. Autonomous deletion has no retained body after removal. Current-format ExplicitMaintenance queued requests retain an exact cursor cut without logical removal: S reads that membership and P applies the E gate, preserving independent inactive/telemetry gates. No policy override, maintenance command or erasure delay is introduced. Real Engine tests cover queued display/export as well as atomic absence; typed contracts cover bounded stream failures. |
| Controlled fixtures / 20–60 | Existing `test-hooks`, projection transaction pause, generation transition hooks and erasure hooks; [test_hooks.rs](../../../src/rust/crates/fathomdb-engine/src/test_hooks.rs). | Test harness only, never shipped feature. Use ordinary writes first; deterministic disposable fixtures may seed legitimate otherwise transient states with supported test build/control. Positive production read paths stay unchanged. Named seams and limits below; test hook existence is not production support. |
| Native views/protocol/install / 20–60 | Original S16 required new Inspector-owned U/K/R components; record-browser/session/protocol code now exists in the imported Slice 20 implementation. | Current source covers record20, native lexical30, local vector/cached CPU40 and graph/history50; installed/all-view technical qualification completed70/75; historical human study removed. Payload correctness does not establish interaction, accessibility, resource or installed pass. |

Build features: shipping Inspector explicitly selects `cached-cpu`, forwarding
Engine `default-embedder` with Engine defaults off. Bare Cargo development builds
remain model-free and never probe cached assets. The shipping artifact excludes
operator/test-hooks/migration-test-hooks/TC5/CUDA/Metal/ONNX/default-reranker.
Fixed EngineConfig uses one scheduler runtime thread, one embedder worker and
25,000ms provider timeout; unchanged remaining defaults. Diagnostics SQLite
initializes in the cleared child before its first connection. The one owned
session uses production Default or deliberately selected None, preserving stored
producer/runtime distinctions and actual failure rather than retrying None.

40's source-qualified route copies only existing pinned bytes into bounded sealed
memfds, hashes after confirmed seals and exposes a private cache-shaped view to
the unchanged loader. LandlockABI3 mutation/truncation confinement and denied new
sockets precede Engine threads. A connection authorizer denies persisted vector/
mean/probe bytes and rebuilds, with only exact kind-enrollment INSERT metadata
permitted for Engine's idempotent boot graft. Existing45-probe references and
materialization checks avoid known startup embedding debt. No mutable-cache
preprobe alone, model download/repair, new persisted embedding or copied ranker.
Missing/unqualified assets/prerequisites leave independent browsing/text usable.
Actual Default compatibility failure never retries None. Cached mode needs ABI3+;
70 still qualifies Ubuntu22-compatible packaging/kernel and installed behavior.

### Process environment is part of the selected composition

Before dependency initialization, spawn the absolute Inspector executable with
`Command::env_clear`. Parent resolves only an absolute cache candidate string,
not SQLite or assets. The child validates its owned canonical work/view siblings,
then performs canonical DB preflight, stored prerequisites and bounded cache
probe. Launch policy/build/IPC are typed owned process inputs, never shell or an
environment dump. Assets are hashed after sealing; ambient hints never become
loader download configuration.

Session policy `cleared-env-v2` has exactly six keys: work `TMPDIR`,
`LANG`/`LC_ALL=C.UTF-8`, `FATHOMDB_EMBED_DEVICE=cpu`, `RAYON_NUM_THREADS=1`,
and private-view `XDG_CACHE_HOME`. CPU settings are inert for None admission.
HF/proxy/LD/performance/GPU/test/telemetry overrides are absent. Browser launch
retains its separate three-key base plus validated GUI allowlist. Build identity
names the composition without recording parent values or secrets. A None choice
before probing has no source-cache accesses; failed bounded asset probes read
cache but perform no create/repair.

This boundary is necessary on the unchanged production source:
`connection_runtime.rs` applies environment-selected writer and reader PRAGMAs
(`apply_perf_experiment_writer_pragmas` / `apply_perf_experiment_reader_pragmas`),
`open.rs` invokes writer overrides before admission work, and `search.rs` reads
`FATHOMDB_PERF_SEARCH_LIMIT` for hybrid lexical candidates. Cargo feature choices
and EngineConfig alone do not disable them. Inspector must prevent inherited
experiments from changing header/effect or query-population semantics.

Launch the browser separately using its resolved absolute executable and a
private session profile, not the worker environment. Clear its parent environment,
then allow only validated local GUI connection variables `DISPLAY`,
`WAYLAND_DISPLAY`, `XDG_RUNTIME_DIR`, `XAUTHORITY`, `DBUS_SESSION_BUS_ADDRESS`
when present, plus the same fixed locale and owned scratch directory. Do not
pass Engine/model overrides, proxies, loader paths or credentials. Use explicit
profile/bootstrap paths and command arguments; no inherited browser extensions,
persistent profile or shell expansion. Browser-owned GUI settings never flow back
to Engine/helper processes. Missing GUI prerequisites give an actionable launch
error; they do not trigger a broader environment fallback.

F-05/F-07 hostile-parent tests set experimental writer/reader PRAGMAs and a tiny
hybrid search cap before launch. Independent header sentinels must stay unchanged,
and actual lexical/hybrid candidate facts must match ordinary selected config
under identical fixtures/options. Assert effective worker allowlist, CPU identity,
no cache writes/network attempts or parent-value/log leakage. Feature 20 owns
spawn/admission enforcement; 30/40 query/model checks; 70 installed execution.
These are planned implementation tests, not a runtime security/performance pass.

### MCP and sibling reuse decision

The [source comparison](compatibility-0.8.28-0.8.29.md) pins release/0.8.28
`19704c6a760f32404d6cafad52fbce070a078b46` and dev/0.8.29
`8aec60ecea84ddc044e25c5247da787a61c438b6`; its code differences and
[architecture research](../research/compatibility-architectures.md) are inputs.
Adopt existing Engine query compilation, FTS, fusion, evidence and bounded
public trace. Reject MCP transport/agent response formatting and a forced shared
session abstraction: selected public snippet/evidence routes are reusable only
under their actual contracts, while MCP transport/envelopes and agent response
budgets are not Inspector's aggregate lifecycle contract. The route-specific
reuse decision must use current source, not historical absence claims. Do not cherry-pick/import sibling code or depend
on its planned follow-ups. The selected child/runtime boundary supplies local
ownership; useful future shared abstractions need demonstrated benefit and
separate scope. Current private vector-stage helpers and benchmark-only
`tc5_vector_stage` are rejected as production APIs; the exact stored-neighborhood
operation below is Inspector-owned and does not promise serving-stage parity.

## Admission, Engine lifecycle and current layout

Show the selected Engine mode and possible startup/worker/shutdown changes
before Attach. A bounded preflight (owned connection/process, 30 s deadline)
rejects missing/empty/nonregular file, missing/held product-lock prerequisite,
noncurrent schema, initial nonempty WAL/journal, malformed header/layout or
incompatible profile. Refusals preserve database, sidecars and cache. Use
read-only SQLite URI with immutable preflight only while external writers are
stopped and sidecars absent; close it before Engine open. It is not the live
reader. Acquire/release an existing lock only as a nonmutating preflight probe;
Engine then acquires its lock. Revalidate file identity/header immediately before
open; external-writer stoppage remains a prerequisite, not a race-proof promise.

Configure the Engine SQLite runtime through public `configure_runtime` in the
session child **before its first rusqlite connection**, including preflight.
A separate preflight process has separate SQLite global state. Otherwise a
preflight connection can make Engine initialization fail with `TooLate`.

Current `user_version` is necessary but insufficient. S validates required
current table/column/encoding shapes and singleton/generation metadata before
open; malformed or absent metadata refuses, never bootstraps. Require existing
current vector partition shape, profile, generation-current records and boot
completion markers `_fathomdb_open_state.search_index_tokenizer_reproject_complete`
and `tc33_edge_vector_prune_complete`, both value `1`. Check existing
BGE mean: if absent and vector count can reach the pin threshold, refuse before
open rather than permit startup mean recovery. Count only to threshold+1 under
budget. No Inspector recompute call is exposed. No full integrity scan claim
follows. Engine may still reject a file after these bounded checks.

Permitted unchanged open effects are lock metadata, WAL/SHM, profile/generation
validation, rederivation of already declared property projections, reconciliation
of declared filter/vector attributes, ordinary projection/closure workers and
shutdown. Engine functions named `bootstrap` also validate an existing complete
generation; allow that no-initialization branch only. Missing generation/markers,
shape repair, migration, mean pin recovery or schema creation are preflight
refusals. This is an explicit guarded subset of current-schema attachment,
not a promise every malformed/current file opens. F-05 compares intended
caller-content invariants and allowed event classes by phase, then equivalent
Engine runs for attribution. A startup failure after open warns that allowed
changes may have occurred; it does not claim preflight preservation.

Live S connections use ordinary read-only WAL-aware SQLite, query_only, busy
limits, same bundled sqlite-vec and no extension loading. Each operation owns
its connection/read transaction; schema and file fingerprint are checked again.
Never `immutable=1` on an Engine-changing database. Reads release snapshots
between chunks/requests; do not keep a snapshot solely to preserve an erasure
pending export. Normal Engine generations change observation, not database ID.
Replacement (device/inode/attachment nonce), incompatible schema or ownership
loss expires session references and stops admission; no path silently reopens.

F-04 setup: ordinary writes seed incomplete provenance, retained superseded and
inactive nodes, expired edges, declarations and valid origins. Existing
`transition_projection_generation_for_test`, `set_projection_scheduler_frozen_for_test`,
`force_next_projection_storage_failure_for_test` and projection transaction pause
establish pending/failed generation cases in the test child (see existing
`slice40_projection_completion.rs` and `slice40_mutation_projection_status.rs`);
debug-only controls require a debug fixture build, never production availability.
A test-only wrapper
arms existing hooks before operations, not a shipped UI endpoint. Missing cache
is a ready/runtime-unavailable case. Other test-child barriers can keep supported projection test states while
production reads run unchanged. This does **not** establish retained erasure.
Physical closure deletes matching canonical/projection content transactionally
before durable pending state; a hand-seeded closure claiming zero residue while
retaining the body is invalid even if open checks only the proof encoding.
No supported hook/fixture route establishes retained-erasure content in this
Engine, and none is required by the 2026-10-31 ruling. Real integration verifies
atomic deletion/absence, flag behavior and no reconstruction; pure typed-content
policy/export tests cover conditional positives independently. Never hold an
external writer or erasure-delaying snapshot to create an export target.

## Typed references, observation and operation contracts

All locators are opaque server-minted handles on the wire; their internal
identity tuple is typed. No bare number/string is accepted across classes.
Canonical reference: database instance nonce + attachment nonce + node/edge
class + exact cursor + recorded revision ID if present, with logical identity
separate. Operational current reference: collection+record key+observed write
cursor; log reference adds retained entry cursor. Projection reference: declared
representation/profile+physical entry key+verified origin set+generation evidence.
Result reference: request observation ID+ordinal+Engine IdSpace(namespace,value),
not a canonical locator. Large integers/cursors travel as exact decimal strings.

Resolution returns `unique(exact_ref)`, `choices(refs, cardinality)`, `unresolved`,
`unavailable`, `refused` or `expired`; zero and many never coerce to one. Equal
bodies never establish equality. Passage IDs stay passage observations. Missing
artifact revision metadata does not justify manufacturing a durable revision ID.
Exact requests require the original class/cursor/revision match and retained
bytes; current requests explicitly ask for a fresh logical-ID observation.
Worker progress never silently retargets exact links.

Observation includes operation ID, session/build pair, actual backend route,
resolved time/view when supplied, read start/end, available Engine write/
projection/dependency generations and scope. An S transaction can attest one
snapshot within that request. A Q Engine result keeps its supplied context;
if an API omits a generation, it is unknown. Later S hydration/match/eligibility
checks get separate contexts; they are not claimed atomic with the search.
Frozen evidence resolvers are used only with their matching Engine authority.
Stored producer/provenance meaning and current serving-query meaning remain
separate. Absence from top-k proves neither nonexistence nor a specific cause.

Every response carries `complete`, `partial(reason, continuation)`, `cancelled`,
`budget_exhausted`, `busy`, `unsupported`, `unavailable`, `refused`, `expired` or
`error`, plus examined/returned counts and byte/work scope where measured.
Unknown evidence is a field-level tagged state, never zero/false/empty success.
No lossy universal row DTO: payload variants retain their operation's meaning.

| Operation family | Request and result contract |
| --- | --- |
| Catalog/page/count/sample | Explicit category, allowed kind/role/collection/source/field constraints, ordering and cursor; result declares revision/logical/current/log/projection counting unit, filter/policy scope and examined cap. Observed fields are a scoped sample. Unsupported predicate refuses before execution. |
| Exact detail/chunk/copy | Typed exact reference, field path and byte range; complete metadata separate from text/structured JSON tree, missing/null/empty distinguished. Read exact underlying UTF-8/bytes with bounded offsets, boundary handling and continuation. Preserve numeric literal spelling and oversize scalars; do not parse all numbers through JavaScript Number or preload the 10 MiB value. Explicit copy streams authorized current bytes to clipboard. |
| Lexical | Body/edge, named property or BM25F substrate route; query/filter/view/limit retained. Ordered hits preserve actual score family/direction, typed ID, cursor and originating evidence; indexed text is separate from canonical body. FTS5 effective tokenizer comes from selected schema/build, not recorded per-attribute override. BM25 negation/fusion score is labeled rather than relabeled raw bm25. No raw BM25F rank is compared numerically to fused rank. |
| Hybrid | Q exact public options and limits, observed result body/IdSpace/representative arm, actual explanations/sidecars only. No hidden graph/CE enablement; unsupported CE is disabled. Policy removes withheld payload/links without inventing new Engine ranks or backfilling more hits. State displayed subset may contain gaps. |
| Vector neighborhood | Select a stored record vector or query embedding with matching runtime identity/dimension. Declare profile, stored uncentered f32, lower-is-nearer L2, source-type/declared constraints, exact read snapshot and bounded population cursor range. Examine at most 100,000 stored rows and return nearest within that examined population, never global nearest if truncated. Stream one vector at a time into a bounded top-k heap using sqlite-vec L2; do not copy serving rank/eligibility predicates. Stored population may include retained ineligible rows under D-01; show their lifecycle facts separately and clearly label stored-neighborhood scope. Query-time hybrid separately uses actual Engine eligibility and binary shortlist→f32 L2 ranking. Binary signs/mean centering apply to serving shortlist, not an unexplained transform of stored f32 L2. |
| Graph/trace/history | Seed exact node/revision, direction in/out/both, edge kind, depth and continuation. Return full typed nodes/edges, endpoint refs, distinct link kinds, retained revision chain and missing targets. Preserve parallel edges/self loops/cycles; deduplicate by identity only. Traversal scope distinguishes stored adjacency from Q serving graph and dependency trace. |
| Association/readiness/check | Declaration, materialization entry and verified origins with zero/one/many cardinality; runtime availability and observed serving evidence are separate axes. Node state/supersession/half-open validity and edge t_invalid/dependency facts use class-correct fields. No invented edge node state or t_valid filter. Later checks state time/source and unknown cause of absence. |
| Narrow export | Only selected still-retained erasure-pending content with `--show-erased`, all other gates and explicit local destination. Stream exact value plus disclosable identity/revision/context; distinguish complete/partial/cancel/error, never write to product/cache or network. This is not a dump/general export route. |

### Disclosure at every boundary

Adopted [D-01](../requirements.md#adopted-operator-disclosure-policy--d-01-2026-10-30)
is fixed policy, not a pending decision. For inactive/history I, retained
owed-erasure E and telemetry payload T, P enforces `(not I or not N) and
(not E or Eflag) and (not T or Tflag)`, where N=`--no-show-inactive`,
Eflag=`--show-erased`, Tflag=`--show-payloads`. Normally accessible data is
shown; append-log age is not a lifecycle classification.

| N | Eflag | Tflag | History/inactive | Retained owed erasure | Telemetry payload |
| --- | --- | --- | --- | --- | --- |
| 0 | 0 | 0 | allowed | withheld | withheld |
| 0 | 0 | 1 | allowed | withheld | allowed |
| 0 | 1 | 0 | allowed | allowed | withheld |
| 0 | 1 | 1 | allowed | allowed | allowed |
| 1 | 0 | 0 | hidden | withheld | withheld |
| 1 | 0 | 1 | hidden | withheld | allowed |
| 1 | 1 | 0 | hidden | allowed | withheld |
| 1 | 1 | 1 | hidden | allowed | allowed |

Columns are independent gates: an I+E+T object requires all three permissions,
not whichever column allows it. Structure/counts for telemetry remain available
with policy-filtered scope; payload/erased identifiers stay out. Physically
absent content never reconstructs from indexes, vectors, telemetry or old results.
P establishes closure/dependency membership and retained bytes before emission;
unknown membership withholds affected content rather than assume permission.
It rechecks every new detail/chunk/evidence/link/copy/export, including cached
observations, using current authoritative context. Links/results are policy
filtered before references are minted. Browser copy/export always asks the
service for revalidation; it cannot elevate policy using old local data.
On detected classification change, invalidate retained payload caches and notify
the UI to clear affected content; previous user-observed bytes cannot be erased
retroactively. Race contracts attest the read instant, not indefinite future
retention. No long-lived snapshot delays owed erasure.

## Actual native views and accessibility

U exposes a FathomDB structure navigator, not a catalog of SQL table names.
Kinds, structural roles, operational modes, canonical source/derived records,
projection declarations and result observations have distinct groups and labels.
Readable text uses inert prewrapped content with paged chunks; JSON uses a
lazy token tree with expand/collapse and exact scalar copy. System metadata is
separate. Neither a grid nor a raw JSON response substitutes for this detail UI.

The lexical view shows selected index/fields/effective tokenizer, indexed text,
query controls, ordered hit cards, score family and verified origin navigation.
Verified match spans are highlighted as data; unavailable spans are labeled.
The vector view shows model/profile/dimension/readiness and scoped distance bars
with numeric L2 labels (zero at best, lower nearer), selection and links; no
fabricated 2D embedding geometry. Graph uses interactive directed SVG with
arrowheads, edge kinds, full edge detail, select/expand/collapse, bounded layout
and synchronized relationship list. Parallel edges and self loops stay distinct.
Layout coordinates never mean stored strength or vector similarity.

One O selection handle coordinates views/back stack and original result context.
Ten observations/20 MiB caps include graph expansions and cached values; collapse
releases hidden graph state. Eviction is announced, preserves safe locator/
context and never silently reexecutes a query. Each visual has equivalent labeled
text/numeric facts and keyboard actions: roving focus, enter to inspect, explicit
expand/collapse, back, visible focus and predictable focus return after eviction.
SVG selection has a synchronized keyboard list, not a pointer-only canvas.
Stored URLs/HTML/scripts render inert; explicit ordinary external links require
user action and never contain service privilege or trigger automatic fetch.

## Execution, budgets and finite ownership

R enforces the canonical operating envelope rather than earlier draft budgets:
200 items/2 MiB response, 256 KiB chunks, 100,000 examined rows/30 s aggregate
inventory, 500 graph objects/1,000 examined links/depth 3, one executing costly
request/eight waiting, ten observations/20 MiB, 30 s attach/request/teardown,
250 ms cancel acknowledgement; RSS growth 1 GiB plus separately measured model
residency ≤2 GiB, browser growth ≤512 MiB. F-06 qualifies, not this document.

Queue entries have cancellation tokens/deadlines before execution; excess is
busy. One request consumes one shared work/byte/time budget across all category
reads/stages, never a fresh budget per table. S uses keyset cursors and bounded
fetch/chunk reads, statement progress/interrupt handles on its own connection,
busy timeout, and an explicit examined counter. SQLite VM work/time limits cover
sort/join/count work that output limits miss. Exact counts become partial at
cap; graph scans cannot hide high-degree work behind output limits. Top-k vector
heap and streaming JSON tokenizer avoid corpus/payload preload. Response framing
counts serialized bytes before delivery. Limit before allocation where possible.

Q Engine pool calls have no general interrupt handle or examined-row API. Run
only one costly request; cancellation acknowledges immediately and drops delivery,
but R retains its execution slot until quiescence. Cancel starts an absolute
30 s teardown deadline immediately, as do detach/close or request timeout; it
is never request-deadline plus another 30 s. Stop admission and close the whole
session child: give graceful Engine close at most 5 s within that deadline, then
terminate/kill and reap the owned process group within the remaining time.
This expires the session and all references, with truthful cancelled/budget state;
it does not pretend the query returned empty or Engine gracefully closed. Kernel
termination of a normal local process is the finite ownership mechanism for
native calls, not Rust thread cancellation. Uninterruptible OS/device failure is
outside the declared healthy local-SSD envelope; installation/fault tests must
exercise real kill/reap/lock release. Never delete WAL/lock files to force success.
A forced termination can leave WAL: next launch follows refusal guidance.

CPU model inference runs inside the existing owned Engine child using unchanged
production Default, not an Inspector Embedder wrapper or a second Engine. Sealed
asset descriptors stay alive through Engine teardown. Fixed one-worker/25s
provider bounds and shared30s request/teardown ownership apply to real native
work; cancellation expires delivery and reaps the owned process group. Count
model residency separately from request growth. No successful stored neighborhood
or lexical fallback counts as dense serving. [40 status](../plans/0.2.0/slice-40/status.md)
records actual native control/RSS outcomes and limits. Positive F06 work must
meet the declared budget; killing every positive request is not compliance.
No cached model held by a dead session survives detach. Scratch files are per
session and removed only by their owner after handles/processes finish.

## Local service and protocol pairing

K binds 127.0.0.1 on an ephemeral port, never wildcard; IPv6 loopback is a separate
explicit binding if added/tested. Generate a high-entropy launch credential,
supply through a private local launch handoff, never a URL/query parameter/log.
Use an HttpOnly SameSite=Strict local session cookie. The launcher writes a
mode-0600 temporary local bootstrap HTML form and separate local script in its
private session directory, opens that file in the selected browser and posts a
one-use random bootstrap secret in the body to the exact loopback endpoint.
Only this bootstrap route accepts `Origin: null`; it validates the one-use
secret and Host, consumes it, sets the cookie and redirects to the public shell.
Delete bootstrap files after use/timeout; no secret is in the file URL. Remote
origins cannot guess this secret or bootstrap a cookie. Main application POST
requests require the cookie and exact Origin/Host; no `null` exception applies
to data routes. Bootstrap CSP permits only its local script and exact form
action; main-shell CSP remains stricter. Test denied/missing/replayed bootstrap
and redirect/cookie behavior in the actual selected browser before any DB read. No CORS,
no unauthenticated database read, no service-worker persistence. A cross-site
request cannot bootstrap a privileged cookie. Static public shell contains no
database data; credentials are not embedded in public static assets. Slice 20 owns this concrete one-time handoff and all data-route rejection tests.

Every handshake/request checks exact bundled asset/service build digest,
protocol contract revision, launch nonce and attachment nonce before DB access.
Stale tabs reject and offer reload/reattach, never reinterpret an old handle.
Additive optional fields may be ignored only if their absence does not change
meaning; unknown discriminated operation/reference/state/metric variants refuse
the affected operation rather than default-coerce. Incompatible semantics change
the paired build/contract and expire sessions. Independent client/service releases
are not 0.2.0, and no negotiation/plugin framework is introduced.

Use CSP with no inline scripts/eval/remote sources, render via text nodes, reject
HTML injection and script-bearing URLs, do not auto-fetch stored attachments.
No payload/query/credential/erased identifier in URLs, access logs or errors.
Structured safe error codes are separate from local path details. Body sizes,
header counts, request framing, IPC and export destinations have bounded parsing;
reject ambiguous framing and unexpected content types before operations. Export
streams only selected authorized content to a local destination and reports
partial files truthfully, without replacing product/cache files.

## Requirements to components and verification

This mapping allocates architecture to **every** shared row; canonical criteria,
fixtures/oracles/layers remain in the matrix. Feature status records identify executed checks; later feature and integrated qualification remain planned.

| Matrix rows | Architectural component / contract | First owner and handoff |
| --- | --- | --- |
| M01–M03 | S catalog/concepts/scoped counts; U navigator and actual detail; O | 20, extensions 30/40/50; 60 transfer |
| M04 | A/K guarded current-schema admission, phase effects and install | 20; 60 installed refusals |
| M05 | S collection/log/state identities | 20; 60 operational flows |
| M06–M07 | S bounded allowed constraints/chunks; U document/tree; P | 20, lexical 30; 60 scale |
| M08 | S exact edge/endpoints; U directed graph/detail | 50; 60 |
| M09–M10 | O exact typed handles/back/eviction; U | 20, each view owner; 60 |
| M11–M13 | S/Q association/materialization/runtime axes and operation controls | 20 core; 30 lexical, 40 vector, 50 graph |
| M14 | U four factual axes, Q truthful unknown | 60 functional T-05;70 installed |
| M15 | O request context/frozen authority/later reads; A expiry | 20 foundation; each Q owner; 60 |
| M16 | Q lexical/fused families; S exact L2 scope; U labels | 30/40; 60 |
| M17 | O zero/one/many exact origin resolution; S links | 20 references, 30/40 origins, 50 history |
| M18–M19 | S node/edge class-correct lifecycle, provenance/history; Q trace | 20 node; 50 edge/trace/history |
| M20 | P full D-01 truth table; conditional retained content, real atomic-delete absence | 20 common gate/30–50 extensions/60 installed; typed content contracts distinguish Engine scope |
| M21–M22 | Q actual result/sidecar; S separate FTS observation; U evidence | 30, extensions 40/50; 60 integrated |
| M23 | S scoped exact f32 L2, Q staged serving attribution | 40; 60 |
| M24–M25 | U real document/tree and lexical interactions | 20/30 respectively; 60 |
| M26–M28 | U metric bars/interactive directed SVG/coordination, O | 40/50 local; 60 complete transfer |
| M29–M30 | A/R Engine effects and cache-only model resource ownership | 20 lifecycle; 40 positive models; each feature/60 |
| M31–M32 | R aggregate accounting/child termination; S exact chunks/O | 20 infrastructure; each feature resource cases; 60 aggregate |
| M33–M35 | K/P/U origin/auth/build pairing/inert values/keyboard/install | 20 shell, each view owner; 60 installed all views |
| M36 | Q unchanged reuse decision, R request/scratch/native ownership | 16 source design; 20/30/40 runtime evidence |
| M37–M38 | U/O all views, Q evidence, S trace; independent fixture/browser facts | 50 High functional; 60 person workflow + High investigation;70 installed |
| M39 | P/S/R conditional exact retained-erasure stream; no target is unavailable/not-applicable | 20 typed contract/export path + real absence/flags; 50 overlap; 60 qualified artifact scope |

Feature 20 implements shared A/S/P/O/R/K and text/tree vertically, including
conditional export contract, real absent-target/flag integration and parser/help tests. 30 adds Q lexical plus indexed-content
and actual match UI; 40 cached CPU ownership and stored-neighborhood/serving distinction;
50 complete graph/history/provenance/closure UI; 60 installed aggregate, unseen
functional investigation and support statement. Each owns local errors/cancel/accessibility
and required positive fixtures; 60 cannot absorb unfinished basic behavior.

## Retained-erasure scope — B16-01 resolved

On 2026-10-31 the owner chose option 1, **“only if it exists”**, and explicitly
rejected changing the Engine to avoid physical deletion. Physical deletion is
correct behavior. This resolves B16-01 by correcting the over-specified mandatory
positive Engine fixture; D-01 flags/defaults and independent gates remain adopted.
No retention/delay, erasure-intent queue, Engine/API/schema change, forensic
reconstruction or fabricated pending closure is authorized or needed.

Source-grounded chain (selected source `728884bfe`):

- `erasure.rs` purge removes projections/canonical content at lines 835–861,
  measures the closure at 872 and commits at 874. Source excision deletes at
  1436–1443, measures at 1488 and commits at 1490.
- `dependency_closure.rs` `record_physical_closure` records a zero-residue proof;
  `measure_physical_closures` rejects any nonzero remaining canonical/projection/
  dependency/receipt rows at 520–529. These operations share the deletion
  transaction: residue failure rolls back both content deletion and obligation.
- Existing real-database regression `slice30_dependency_closure.rs` lines
  438–553 explicitly tests physical-residue rollback. Its fixture is evidence
  against manufacturing retained payload + a zero proof, not a missing hook.
- Postcommit incomplete paths concern telemetry redaction/WAL checkpoint
  (`erasure.rs` 998–1005). `telemetry.rs` records identifiers/arms/query lengths,
  not retained canonical/query bodies. These identifiers do not establish a
  readable content target. Old WAL bytes are not a supported record read;
  initial nonempty WAL is refused and forensic reconstruction is out of scope.

S/P first check for legitimate readable content and authoritative classification.
On the original schema-34 composition no retained-erasure target was exposed.
The current schema-37 composition also supports already-queued ExplicitMaintenance
requests through their exact frozen cursor scope; these are real retained targets,
not completed logical removal. Autonomous post-delete content remains unavailable.
Never substitute soft-deleted history, telemetry identifiers or old WAL bytes.
All ordinary/history/telemetry policy paths retain their real positive cases.

Verification has two distinct layers: pure typed policy/stream contracts use
supplied content and independently specified intent to test exact output,
independent overlap gates, cancellation/partial/error and disappearance without
mocking the database; real unchanged Engine integration verifies atomic deletion,
absence, flags and no reconstruction. A supplied-content unit positive is never
reported as Engine end-to-end qualification. If a later qualified supported
Engine legitimately exposes readable retained-erasure content, positive Engine
display/export becomes required for that composition before claiming support.
Installed qualification reports the actual qualified policy/state combinations.
Other positive requirements, including every native view, remain mandatory.

The prior pre-deletion intent/read proposal is rejected for this release; do not
add it to implement Inspector. Source facts remain useful, but no longer block
Slice16 under the owner's explicit clarified scope.

## Evolution through FathomDB 0.9 and 1.x

This architecture supports deliberate changes, not automatic future file support.
The [research stress test](../research/compatibility-evolution-stress-test.md)
identifies real semantic break points; capability booleans cannot resolve them.

| Scenario / affected boundary | Preserved invariant and required change | Compatibility response, qualification and decision trigger |
| --- | --- | --- |
| New storage layout/encoding/schema, same concepts / S,A | Keep typed class/identity and exact retained value semantics; replace named reader/shape checks, not UI labels if meaning truly unchanged. | Refuse until one selected reader+query build qualifies intended fixtures; no historical fallback. Supporting an older format requires explicit roadmap promotion. |
| Same tables, changed eligibility/tokenizer/fusion/score / Q,S observation | Serving semantics belong to current query build, historical producer behavior/provenance stays stored fact. Update route/metric/evidence payload and labels where meaning changes. | Schema equality is insufficient; semantic conformance fixtures compare actual operations and independent facts. Unknown producer meaning stays unknown; incompatible meaning needs new paired contract/build. |
| Multiple models/representations/metrics/ANN / S,Q,U | Embedder-owned identity, per-representation dimensions/transform and real distance direction/population survive. Add genuine metric/result variants and renderer scales, no L2 default coercion. | New corpus for each supported representation/selection path; decide model/cache/residency budgets and support. ANN shortlist is not exact neighborhood. |
| Passage results, zero/one/many origins / O,U,Q | Preserve IdSpace and origin cardinality, never map passage ordinal to canonical ID. Add exact passage bounds/provenance evidence when available. | Existing unresolved/choice UI remains honest; positive passage navigation needs real contract/fixtures, not a synthetic stored passage. |
| New identity scope/lifecycle/data structure / domain,O,U | Class-specific typed reference and evidence remain; new variants may need new interaction, field semantics and policy classifications. | Old pair rejects unknown variant; redesign domain/protocol/view together. Generic row DTO cannot hide a meaningful new structure. |
| Changed native/async runtime / R,Q | One owner, bounded admission/execution/teardown, no abandoned work. Replace helper/child mechanism only with measured finite native ownership. | Fault/kill/late-result/resource fixtures requalify; a cancellation Future alone does not prove native work stopped. |
| UI/service restart or independent release / K,O | Session expiry, exact build pairing, no silent retargeting. Independent release would require explicit protocol/support windows. | The carried contract rejects stale pair. Before independent releases, decide additive/incompatible policy and test oldest/newest declared pair; no speculative negotiator now. |
| Live/WAL/encrypted/remote/non-SQLite / A,S,R | Truthful consistency, preservation/disclosure and resource ownership. Different admission/transaction/decryption/remote trust and cancellation contracts are real new design. | Remain roadmap candidates; do not bypass lock or assume SQLite immutable reads. Promote only with scope, threat/consistency contract and real fixtures. |
| Stable public surface/support windows through 1.x / K,domain | Public Inspector launch/help, documented behavior and eventual transport commitments have an owner. Internals/opaque session refs are not durable cross-release IDs. Persisted meaning is distinct from serving version. | Before 1.0 decide minimum producer/reader/query pairs, duration of support/deprecation and supported public transport. Grow fixture corpus only for actual commitments. Breaking pre-1.0 contracts change in place per accepted ADR, no parallel API generations/shims. |

The failure limit is fundamental: indistinguishable stored bytes with different
undocumented meanings cannot be disambiguated by flags or producer strings.
Reuse authoritative compiled schema, actual layout and recorded format facts.
The Inspector static build description attests its own code/model constants;
it cannot invent database producer metadata. A new Engine manifest/queryable
catalog/durable semantic marker requires separately scoped dependency authority.

Establish now: honest references, typed operation meaning, lifecycle effects,
qualified reader/query composition, finite ownership and bundled protocol pair.
Later genuine variants may require contract/view revisions. Before 1.0, support
windows and public transport/deprecation are policy decisions, not promised
compatibility with all 1.x databases. Optional expansion stays in
[ROADMAP](../ROADMAP.md) with promotion triggers; no plugin framework, copied
private engine, historical shim or parallel pre-1.0 API is commissioned.

## Slice 20 implementation refinements — 2026-10-31

The default package excludes Inspector-only `fixture-control`. A distinctly
identified feature build accepts private typed internal control jobs from the
example driver, never HTTP mutation requests. The same cleared child/actual
Engine owns fixture lifecycle changes. Acknowledged one-shot and pre-init
wrapper pauses qualify ownership, queue, cancellation and late delivery; they
do not establish interruption of native work. Source/feature/profile/target/
compiler/flags identify the artifact. The independent Sol design reviewer
accepted these boundaries before affected implementation.

Observed-field samples use the same browse transaction and one aggregate
candidate/time/content budget. First ten matching retained records, at most
64 KiB inspected content each and twenty top-level names, are deterministic
content observations; skipped/refused/oversize/unsupported and truncation remain
explicit. Exact key ranges supply names rather than tree preview placeholders.
Sample facts never prove registered schema or field absence in the population.

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

## Observed Slice 20 development platform — 2026-10-31

The actual executor is Ubuntu 24.04.5 x86_64, glibc 2.39, Ryzen Threadripper PRO
5945WX, Node 26.8.2, Chromium 156.0.8078.4 and Playwright 1.62.1. The engineering
envelope permits Ubuntu 22.04 or newer; this does not meet or expand the selected
Ubuntu 22.04-compatible distribution target. The tested development release
candidate requires GLIBC2.39 and cannot run on stock Ubuntu 22.04. A compatible
build and installed runtime qualification remain pending for 60. Slice 20
package/help/native-launch evidence is scoped to this actual development
executor. No published artifact or broader support statement is claimed.

## Coordinated record workspace — 2026-11-19

The [Slice20 design](../plans/0.2.0/slice-20/design.md) reuses opaque exact
class/cursor/revision handles, current-layout read transactions and the existing
ten-observation/20MiB cache. Record detail exposes independently scoped storage,
inspection eligibility, representation-read readiness, unknown retrieval facts
and None-session runtime evidence. Scalar detail carries an actionable canonical
origin reference; following it is a separate read, not the original snapshot.

Back retains the original panel/tree/chunk/association contexts and initiating
focus after bounded validation of all restored owners. Payload and structural
metadata checks compose D-01 appropriately. One request accepts at most201
references and counts at most100,000 locator/dependency-closure rows with one
25s work deadline. This counter does not count SQL VM steps or the separately
bounded existing queue metadata decode (16 scopes ×16,384 cursors,8MiB, once per
read transaction); the connection retains its shared100M VM-step/30s progress
limit. Eviction clears active and detached presentation bytes. Navigation/owner
acceptance precedes shared response effects, including failed old validations.
No unbounded public status call, new Engine opener or synthetic origin is added.

### Slice30 native lexical implementation

The reviewed [Slice30 design](../plans/0.2.0/slice-30/design.md) owns actual indexed
text, query/filter/view limits, source/effective tokenization, native evidence,
score families, later origin context and UI/Back contracts. Actual property
SearchHit values negate SQLite BM25 despite its method summary; higher is better.
BM25F applies active/current/dependency eligibility but ignores own-node validity.
It has no filter/view/native limit or snapshot payload; output truncation/hydration
are explicit. Source-size and conservative historical alias scratch admission,
final serialized2MiB cap, fresh D01 and inherited owned-child deadlines apply.

### Slice50 stored graph and investigation implementation

Inspector-local `graph.rs` provides indexed directed incident-edge reads and
class-qualified retained history/recorded source/dependency reads under fresh
D01 transactions. Endpoint logical references resolve to bounded retained revision
choices, never a latest alias or an asserted historical endpoint. One100k-row/
16MiB-key scan bounds history resolution in format37's unindexed historical
logical-ID space; exhaustion stays unknown. Objects/candidate relations/bytes
are separately capped before frontend layout. Exact edge content and properties
reuse detail/chunk/copy/narrow-export contracts.

Directed SVG and equivalent keyboard relationships preserve class+cursor identity
through parallel/self/cyclic edges, selection, expansion ownership, filters and
Back. Native `graph_expand` uses a separately minted default-eligible frozen
context and its public exact target/terminal-edge sidecar. Pre-call length-only
admission charges repeated canonical-source hydration and target/edge bodies;
later policy/origin hydration is separately labeled. No private serving predicate,
ranker, similarity edge or unsupported absence cause is synthesized. Full
source/provenance may legitimately refuse on ordinary legacy writes while stored
inspection remains available. The [slice design](../plans/0.2.0/slice-50/design.md)
owns precise limits and the [source guide](../../../src/inspector/README.md)
owns executable feature commands.

## Two actual observation comparison — Slice60

Browser A/B slots reference the existing observation registry by ID and version;
they do not copy payloads or rerun requests. Registry admission retains the actual
submitted request beside the response. Each side exposes original request/seed,
filters/options/limits, population or unknown, status, actual rank and its own
metric/direction, original query context and separate later policy-read context.
Native producing arms are evidence, not identity. Typed database/attachment,
class/IdSpace/value/cursor, physical vector row/profile/window, and exact canonical
class/cursor/revision remain distinct from recorded origin correlations. Frozen
graph paths retain target and terminal-edge identities; graph sets invent no rank.

A50-entry union/intersection page preserves duplicate entries and missing-side
unknowns; it makes no omission-cause, common-score, common-snapshot or database
absence claim. Original native views and exact origin actions remain accessible
with keyboard focus and Back. The original indexed/vector source can independently
evict while its still-admitted result stays usable, explicitly labeling the loss.

The same ten-observation/20MiB registry charges original values, requests, pin
descriptors and conservative transient comparison index/DOM/control reservations.
No extra pin budget or comparison worker operation exists. FIFO/object caps can
evict pins; unpin/replace/evict/refuse/invalidate/expire/detach/cancel scrub derived
DOM and controls. Back releases discarded forward observations only when neither
pins nor other current/history owners retain them. Original source request tokens,
pin versions and navigation/cache epochs reject late actions. Fresh combined
content/metadata checks<=201 validate every displayed owner, including standalone
frozen graph evidence after unpin and wall-clock lifecycle expiry. Policy changes
retain their original strict global invalidation behavior.

## Installed0.2.0 composition — Slice70

Inspector's own manifest/root-lock package entry, help and build-description
version are0.2.0; Engine/shared package versions and pins remain unchanged.
Shipping explicitly selects cached-cpu, fixture-control false. The compatible
artifact is built with pinned Rust1.95 in an isolated Ubuntu22.04/glibc2.35 target,
never reusing host-compiled objects. The resulting ELF requires at mostGLIBC2.34.
Actual qualification uses Ubuntu22.04.5 userland, existing7.0.0-43-generic kernel
(LandlockABI8), Chromium156.0.8078.4 and normal Chromium sandbox. This is an exact
userland/kernel/browser pairing, not stock Ubuntu22/kernel5.15 or all-newer support.

Installed probes start the extracted production executable and its own worker,
consume its private native bootstrap, drive all views, and close its actual owned
window with X11 Alt+F4. No debug driver stands in for native launch/close. Node,
Playwright, Python and X11 tools measure behavior outside the product; the packaged
application requires none of those developer runtimes. Committed tools and their
parameters are indexed in the [source tool guide](../../../src/inspector/tools/README.md).
The [Slice70 status](../plans/0.2.0/slice-70/status.md) records exact tested artifact,
checks and measurement limits; the human study is unperformed/removed. No publication is
implied. The fixed study key uses separate ordinary-write familiarization/unseen
fixtures; the five-participant activity is historical/unperformed and removed
by INS-020-LOCAL-RELEASE.
