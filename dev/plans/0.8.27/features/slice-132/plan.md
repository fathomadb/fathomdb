---
title: FathomDB 0.8.27 Slice 132 — Rust SDK canonical-surface parity
status: PLANNED
target_release: 0.8.27
planning_baseline: a66c67d89
---

# Slice 132 — Rust SDK canonical-surface parity

This slice follows Slice 130 and precedes Slice 135. It makes `fathomdb` the
one first-party Rust application SDK, equal in canonical product capability to
the Python and TypeScript SDKs: no missing accepted operation, result type,
error shape, or supported configuration capability, and no Rust-only
application capability. It does not require identical spelling, method
placement, ownership model, synchronous versus Promise form, or input
container types where those choices preserve the same contract. A second Rust
SDK crate is out of scope: it would create competing public contracts rather
than resolving the existing facade's gaps.

This is a planning gate, not authorization to change a public API. The current
contract intentionally differs: `docs/positions/sdk-parity.md` and
`dev/interfaces/rust.md` define Rust as a distinct consumer contract with
parity-in-intent, rather than Python/TypeScript membership equality. The
accepted `ADR-0.8.0-supersede-five-verb-surface-cap.md` records the deliberate
HITL `BIND-RUST` exception and the resulting Rust allowlist pin. At that time,
Python and TypeScript were compared as command namespaces while Rust was
treated as an Engine-method and re-exported-type facade. That historical shape
explains parity-in-intent; it does not establish a different product purpose
for `fathomdb`. The successor changes the measure from raw member equality to
one canonical capability map, while retaining idiomatic surface form. The Rust
interface says the facade remains as it is until that successor expands it.
Slice 132 must obtain and implement the successor before changing the facade,
the binding allowlist, or any public documentation.

## Entry inventory

The current Python/TypeScript checker validates 69 signed members mapped to 44
live canonical operations. It deliberately has no Rust input. Rust's own
`governed_surface.rs` instead compiles a 37-type positive allowlist and checks
the shared five-name recovery denylist. Those guards prove governance, but they
cannot prove canonical product parity.

The difference matrix below is the entry review artifact. Its first rows cover
all 44 current Python/TypeScript canonical operations by family; the remaining
rows cover the types, configuration, extension, and boundary differences that
operation names alone cannot reveal. “Equivalent” is an entry observation, not
proof that argument defaults, errors, and result shapes match; the frozen
three-way inventory must expand every family into individual rows.

| Canonical capability and current operation IDs | Python / TypeScript state | Rust state at entry | Proposal or HITL question | Evidence and contract impact |
| --- | --- | --- | --- |
| Core: `engine.open`, `admin.configure`, `engine.write`, `engine.close` | Equivalent application capability, with Python/TS binding configuration and async differences. | Equivalent `Engine::open`, `admin::configure_runtime`, `write`, `close`, plus Rust-specific `EngineConfig` entry points. | Preserve only behaviorally equivalent configuration and error contracts; **OPEN HITL** if any config knob lacks a matching SDK effect. | `src/conformance/governed-operation-parity.json`; `dev/interfaces/{python,typescript,rust}.md`; successor must map defaults, range validation, and error precedence. |
| Dependencies and lifecycle: `engine.actuate`, `engine.register_source_dependency`, `engine.dependencies_for_source`, `engine.dependency_for_derived`, `engine.read_dependency_closure`, `engine.transition`, `engine.purge`, `engine.erase_source` | Equivalent governed commands in Python/TS. | Engine methods and re-exported receipt/domain types exist. | Preserve if the exact request, result, and error behavior maps; **OPEN HITL** for any unmatched Rust-only lifecycle/helper method discovered in the audit. | Shared operation map; `dev/interfaces/rust.md` lifecycle/erasure sections; recovery and typed-boundary ADRs remain binding. |
| Search and graph: `engine.search`, `engine.search_text_only`, `engine.search_projected_text`, `graph.expand`, `graph.neighbors`, `graph.search_expand` | Equivalent governed commands and binding-specific namespaces. | Engine search and graph methods exist, with additional Rust overloads/view forms. | Treat overloads as idioms only when they express an existing canonical option; **OPEN HITL** for a Rust-only behavior. | `src/rust/crates/fathomdb-engine/src/{search_api,graph_api}.rs`; interfaces’ search/graph sections; result/error shape audit required. |
| Frozen/evidence: `engine.freeze_read_context`, `engine.search_frozen`, `engine.search_expand_frozen`, `engine.search_with_evidence`, `engine.resolve_evidence`, `engine.resolve_graph_evidence`, `engine.trace_dependency` | Equivalent governed commands. | Matching Engine methods and evidence types exist. | Preserve only exact authority, expiry, and result behavior; **OPEN HITL** for a missing type or extra callable route. | Shared map; `search_api.rs`; `dev/interfaces/rust.md` evidence sections. |
| Reads: `read.get`, `read.get_many`, `read.collection`, `read.mutations`, `read.list`, `read.crossed_boundary_since`, `read.projections`, `read.projection_status`, `read.embedding_readiness`, `read.projection_generation_status`, `read.mutation_projection_status`, `read.canonical_page`, `read.operational_state`, `read.operational_state_page` | Equivalent read namespace commands. | Matching `read_*` / `crossed_boundary_since` Engine methods exist. | Rust method spelling is allowed; inventory must prove parameter/result/error equivalence. | Shared map; `read_api.rs`, `projection_{registry,generation}.rs`; Rust interface read sections. |
| Search-hit identities and public result types | Python/TS expose `SearchHit`, `IdSpace`, and `IdSpaceKind`. | `SearchResult` is exported, but those three types are not. Callers cannot name hits, match `IdSpaceKind`, or call `IdSpace::to_prefixed`. | **OPEN HITL:** retain the shared typed-hit feature and add Rust names, or approve removal from all SDKs. | `dev/interfaces/rust.md` “SearchHit.id”; facade `src/rust/crates/fathomdb/src/lib.rs`; a facade re-export is a governed-surface delta. |
| Standalone rerank: `rerank` | Python/TS package-level operation has typed passages/results, validation, identity path, defaults, and feature refusal. | Internal `rerank_passages` exists; no documented facade entry or public passage/result surface. | **OPEN HITL:** retain and add equivalent Rust entry/types, or approve removal from Python/TS. | `dev/interfaces/{python,typescript}.md` standalone rerank; `fathomdb-engine/src/rerank.rs`; requires shared behavior fixtures. |
| CLS batch embedding: `embed_batch_cls` / `embedBatchCls` | Python/TS package-level pinned-BGE CLS operation, including empty input and feature refusal. | `Engine::embed_text` is mean pooled; no matching facade operation. | **OPEN HITL:** retain and add equivalent Rust entry/types, or approve removal from Python/TS. | Python/TS interface CLS sections; `embedding.rs`; pooling, cardinality, and normalization are contract data. |
| Direct embedding: `engine.embed` | Python/TS expose engine-attached mean embedding. | `Engine::embed_text` provides the likely Rust equivalent. | Preserve if exact default-provider, refusal, vector, and error semantics match. | Shared map; `embedding.rs`; public Rust spelling is an idiom, not a capability exception. |
| Custom provider injection | Python/TS deliberately support default or no embedder only. | Whole-Engine re-export leaves `open_with_choice[_and_config]` and `EmbedderChoice::{Caller, CallerWithDeviceResolution}` reachable through the engine core; `fathomdb` does not itself re-export the choice type. | **OUT by owner direction** for the default `fathomdb` SDK unless the audit produces a concrete external application need and HITL accepts a named exception. Do not add it to Python/TS. | `fathomdb-engine/src/open.rs`; `fathomdb-embedder-api` README; successor ADR must distinguish test/internal injection from SDK reachability. |
| CLI operator/recovery and test hooks | Python/TS SDKs exclude recovery; test hooks do not ship. | `operator` feature is the CLI seam; engine also has test-only/provider injection routes. | Excluded, retain only as CLI/internal boundaries. Do not include merely to equal raw symbol counts. | `fathomdb/src/lib.rs`, `fathomdb-cli`, operator gate and recovery denylist contracts. |
| `fathomdb-engine` and `fathomdb-embedder-api` external posture | Bindings do not expose engine core or an embedder plugin trait. | Both are published; embedder API claims an independently versioned, semver-stable plugin contract. | **OPEN HITL:** audit real external use and choose retain-as-extension, deprecate/retire, or another scoped disposition. This is not a default SDK parity exception. | `Cargo.toml`; `fathomdb-embedder-api/{Cargo.toml,README.md}`; successor ADR must state compatibility and migration posture. |

The execution inventory may add rows but may not silently close a gap. A Rust
facade method or re-export cannot be declared out of scope merely because the
existing Rust test does not enumerate it.

## Appendix A — per-operation entry map

`fathomdb::Engine` is a wholesale public re-export of
`fathomdb_engine::Engine`, so each method below is reachable through the
facade even where the facade README does not enumerate it. “Provisional
equivalent” means the entry point exists; Slice 132 must still compare request
shape, defaults, errors, result types, and installed-artifact behavior. This
appendix is deliberately per-operation so HITL can decide feature-in/out from
individual rows rather than a family claim.

| Canonical ID | Actual Rust facade path at entry | Status | Entry evidence |
| --- | --- | --- | --- |
| `engine.open` | `fathomdb::Engine::open` | Provisional equivalent | `fathomdb/src/lib.rs`; `fathomdb-engine/src/open.rs` |
| `admin.configure` | `fathomdb::admin::configure_runtime` | Provisional equivalent; idiomatic spelling differs | `fathomdb/src/lib.rs` `admin` module |
| `engine.write` | `fathomdb::Engine::write` | Provisional equivalent | `write.rs` |
| `engine.actuate` | `fathomdb::Engine::actuate` | Provisional equivalent | `actuation.rs` |
| `engine.register_source_dependency` | `fathomdb::Engine::register_source_dependency` | Provisional equivalent | `dependency.rs` |
| `engine.dependencies_for_source` | `fathomdb::Engine::dependencies_for_source` | Provisional equivalent | `dependency.rs` |
| `engine.dependency_for_derived` | `fathomdb::Engine::dependency_for_derived` | Provisional equivalent | `dependency.rs` |
| `engine.read_dependency_closure` | `fathomdb::Engine::read_dependency_closure` | Provisional equivalent | `dependency_closure.rs` |
| `engine.transition` | `fathomdb::Engine::transition` | Provisional equivalent | `record_lifecycle.rs` |
| `engine.purge` | `fathomdb::Engine::purge` | Provisional equivalent | `erasure.rs` |
| `engine.erase_source` | `fathomdb::Engine::erase_source` | Provisional equivalent | `erasure.rs`; Rust interface lifecycle section |
| `engine.search` | `fathomdb::Engine::search` | Provisional equivalent | `search_api.rs` |
| `engine.freeze_read_context` | `fathomdb::Engine::freeze_read_context` | Provisional equivalent | `search_api.rs` |
| `engine.search_frozen` | `fathomdb::Engine::search_frozen` | Provisional equivalent | `search_api.rs` |
| `engine.search_expand_frozen` | `fathomdb::Engine::search_expand_frozen` | Provisional equivalent | `search_api.rs` |
| `engine.search_text_only` | `fathomdb::Engine::search_text_only` | Provisional equivalent | `search_api.rs` |
| `engine.search_projected_text` | `fathomdb::Engine::search_projected_text` | Provisional equivalent | `search_api.rs` |
| `engine.search_with_evidence` | `fathomdb::Engine::search_with_evidence` | Provisional equivalent | `search_api.rs` |
| `engine.resolve_evidence` | `fathomdb::Engine::resolve_evidence` | Provisional equivalent | `search_api.rs` |
| `engine.resolve_graph_evidence` | `fathomdb::Engine::resolve_graph_evidence` | Provisional equivalent | `search_api.rs` |
| `engine.trace_dependency` | `fathomdb::Engine::trace_dependency` | Provisional equivalent | `dependency_trace.rs` |
| `engine.close` | `fathomdb::Engine::close` | Provisional equivalent | `runtime_lifecycle.rs` |
| `read.get` | `fathomdb::Engine::read_get` | Provisional equivalent; Rust has no `read` namespace | `read_api.rs` |
| `read.get_many` | `fathomdb::Engine::read_get_many` | Provisional equivalent; Rust has no `read` namespace | `read_api.rs` |
| `read.collection` | `fathomdb::Engine::read_collection` | Provisional equivalent; Rust has no `read` namespace | `read_api.rs` |
| `read.mutations` | `fathomdb::Engine::read_mutations` | Provisional equivalent; Rust has no `read` namespace | `read_api.rs` |
| `read.list` | `fathomdb::Engine::read_list` | Provisional equivalent; Rust has no `read` namespace | `read_api.rs` |
| `engine.ingest_with_extractor` | `fathomdb::Engine::ingest_with_extractor` | Provisional equivalent | `ingest.rs` |
| `engine.consolidate_with_provider` | `fathomdb::Engine::consolidate_with_provider` | Provisional equivalent | `consolidation.rs` |
| `graph.expand` | `fathomdb::Engine::graph_expand` | Provisional equivalent; Rust has no `graph` namespace | `graph_api.rs` |
| `graph.neighbors` | `fathomdb::Engine::graph_neighbors` | Provisional equivalent; Rust has no `graph` namespace | `graph_api.rs` |
| `graph.search_expand` | `fathomdb::Engine::search_expand` | Provisional equivalent; Rust has no `graph` namespace | `search_api.rs` |
| `rerank` | **MISSING** | Gap | Python/TS interfaces; engine `rerank.rs` is not a facade operation |
| `engine.embed` | `fathomdb::Engine::embed_text` | Provisional equivalent; idiomatic spelling differs | `embedding.rs` |
| `read.crossed_boundary_since` | `fathomdb::Engine::crossed_boundary_since` | Provisional equivalent; Rust has no `read` namespace | `graph_api.rs` |
| `engine.configure_projections` | `fathomdb::Engine::configure_projections` | Provisional equivalent | `projection_registry.rs` |
| `read.projections` | `fathomdb::Engine::read_projections` | Provisional equivalent; Rust has no `read` namespace | `projection_registry.rs` |
| `read.projection_status` | `fathomdb::Engine::read_projection_status` | Provisional equivalent; Rust has no `read` namespace | `projection_generation.rs` |
| `read.embedding_readiness` | `fathomdb::Engine::read_embedding_readiness` | Provisional equivalent; Rust has no `read` namespace | `projection_generation.rs` |
| `read.projection_generation_status` | `fathomdb::Engine::read_projection_generation_status` | Provisional equivalent; Rust has no `read` namespace | `projection_generation.rs` |
| `read.mutation_projection_status` | `fathomdb::Engine::read_mutation_projection_status` | Provisional equivalent; Rust has no `read` namespace | `projection_generation.rs` |
| `read.canonical_page` | `fathomdb::Engine::read_canonical_page` | Provisional equivalent; Rust has no `read` namespace | `read_api.rs` |
| `read.operational_state` | `fathomdb::Engine::read_operational_state` | Provisional equivalent; Rust has no `read` namespace | `read_api.rs` |
| `read.operational_state_page` | `fathomdb::Engine::read_operational_state_page` | Provisional equivalent; Rust has no `read` namespace | `read_api.rs` |

The shared map currently omits `embed_batch_cls` / `embedBatchCls`; Appendix A
does not downgrade that documented Python/TypeScript capability. It remains an
explicit difference-matrix row and must receive an individual HITL feature
decision.

## Appendix B — provisional Rust-only inventory

This is a source-grounded, non-exhaustive intake list. It gives HITL concrete
items to classify now and directs the implementation inventory to discover
more; it does not claim that every item is an extra capability until the
installed three-SDK comparison establishes behavior.

| Rust item or family | Initial classification | Provisional recommendation | Evidence / reason |
| --- | --- | --- | --- |
| `Engine::open_with_choice`, `Engine::open_with_choice_and_config`, `EmbedderChoice::{Caller, CallerWithDeviceResolution}` | Rust-only provider reachability | **OUT** of default `fathomdb`; specific HITL exception only after concrete external-need audit | `open.rs`; same capability is deliberately absent from Python/TS. |
| `fathomdb-embedder-api::Embedder` and published engine/plugin crates | External extension contract, not a default facade operation | **OPEN HITL** retain/deprecate/retire after real-user audit | independent-version Cargo manifest and plugin README promise semver stability. |
| `search_with_limit`, `*_with_limit`, `search_filtered`, `search_filter`, `search_reranked`, `search_explained`, and `*_view` variants | Potential idiomatic overloads or extra option combinations | **UNCERTAIN** until matched against Python/TS parameters and signed view contract | `search_api.rs`; Python/TS expose options on some searches, while interface view sections are marked proposed. |
| `Engine::drain`, runtime counters, subscriber/control methods, `runtime_configuration`, and slow-threshold controls | Operational capability outside the 44-operation map | **UNCERTAIN**: map to equivalent binding controls or present individual feature-in/out decisions | `runtime_lifecycle.rs`, `open.rs`, facade re-exports; not evidence of equal behavior. |
| `write_node_importance`, `node_importance`, `bm25f_search`, and other public non-test Engine methods outside this appendix's map | Possible Rust-only application behavior | **OPEN**: establish whether public/advertised, equivalent elsewhere, or remove/gate | `write.rs`, `read_api.rs`, `search_api.rs`; no claim of binding absence until full audit. |
| `operator`-gated recovery/integrity methods and report types | CLI operator seam | **OUT** of SDK parity | `fathomdb` `operator` feature and recovery-denylist tests. |
| `*_for_test`, `write_vector_for_test`, test hooks, and private benchmark routes | Test-only or feature-gated harness seam | **OUT** | engine source and test-hook feature declarations; no shipped SDK feature. |

## HITL decisions required before implementation

1. **Parity boundary.** Accept a successor ADR that defines the canonical
   cross-SDK set: operations, request/result/error types, lifecycle and
   configuration capability, and supported extension hooks. It must explicitly
   state which language-level differences are allowed and retain the typed
   boundary and recovery denylist.
2. **External provider disposition.** Confirm removal of custom-provider
   reachability from the default `fathomdb` SDK unless the audit shows a
   concrete external need. If it does, decide the narrowly scoped exception;
   do not add custom embedders to Python or TypeScript. Separately decide the
   published `fathomdb-engine` and `fathomdb-embedder-api` compatibility and
   migration posture, including whether they remain a supported extension
   boundary outside SDK parity.
3. **Shared standalone utilities.** Choose whether standalone reranking and
   CLS batch embedding are part of the canonical SDK set. If they are, Rust
   gains equivalent entries and public data types. If they are not, remove them
   from Python and TypeScript through an explicitly approved breaking contract.

No decision is inferred from an implementation convenience, an internal engine
function, or the current parity checker. A successor ADR and matching interface
updates are the entry gate for code.

## Requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-132A | The cross-SDK parity contract is explicit and HITL-approved. | AC27-132A: a successor ADR, all three interface documents, and `docs/positions/sdk-parity.md` define one canonical set and its expressly excluded operator/companion boundaries. The decision accounts for each entry-inventory row; no public change precedes it. |
| R27-132B | The Rust SDK has no accepted canonical capability gap. | AC27-132B: generated/live inventories for Rust, Python, and TypeScript map every canonical operation and its typed input, result, and error vocabulary to each surface. The checker fails for a missing, extra, renamed, or shape-incompatible canonical item; language idioms are mapped rather than compared as raw names. |
| R27-132C | Rust application callers can name every accepted result and identity type. | AC27-132C: consumer compile fixtures import and use each required Rust type at the facade boundary, including `SearchHit`, `IdSpace`, and `IdSpaceKind` if retained in the approved set; Python and TypeScript type/declaration fixtures prove the same capabilities. |
| R27-132D | Standalone accepted utilities have equal behavior in all SDKs. | AC27-132D: shared, human-authored fixtures prove rerank validation, identity behavior, defaults, ordering, scores, feature refusal, CLS batch cardinality/order/normalization, empty-input behavior, and error mapping through each public SDK. An internal engine-only function is insufficient. |
| R27-132E | No unsupported Rust-only application capability remains. | AC27-132E: the provider audit and successor disposition are complete; the default Rust facade contains neither custom-provider reachability nor a CLI recovery route, unless HITL accepts a named concrete exception. Internal/built-in provider support and any approved external extension boundary remain outside default SDK parity. |
| R27-132F | The parity change is released as an intentional public contract change. | AC27-132F: root/package imports, Rust consumer compilation, Python stubs, TypeScript declarations, installed Rust crate/wheel/npm-package tests, existing recovery/no-raw-SQL guards, and the full repository gate pass on one reviewed candidate SHA. Public docs and release notes state any breaking change. |

`dev/acceptance.md` remains locked; these IDs are release-local.

## Execution order

1. **Freeze the actual surfaces.** At the exact post-Slice-130 SHA, capture
   Rust default and `operator` feature APIs, companion-crate exports, Python
   runtime/stub exports, and TypeScript runtime/declaration exports. Derive the
   44 current canonical Python/TypeScript operations from the shared companion
   file and inventory their input/result/error types and behavior. Capture
   installed artifacts as well as source declarations.
2. **Hold the HITL scope review.** Present a three-way matrix that classifies
   every row as canonical, language idiom, CLI-only operator capability,
   companion protocol, test-only hook, or ungoverned leak. Audit actual external
   users of the published engine/plugin crates and prove which internal tests
   and built-in providers require injection. Present the three decisions above,
   then write the successor ADR and update the active
   interfaces only after approval. A rejected or deferred decision leaves this
   slice planned and blocks Slice 135.
3. **Write failing contract tests.** For each approved gap, add a minimal
   human-authored cross-SDK fixture or Rust consumer compile test that fails on
   the entry candidate. Keep the existing Python/TypeScript checker intact
   until its successor can prove all three SDKs. Mutation-test the new checker
   with a missing Rust export, a Rust-only operation, and a result/error shape
   drift.
4. **Implement the smallest approved delta.** Prefer facade re-exports and
   thin calls to existing engine behavior when they satisfy the approved
   contract. Preserve the default operator gate, no-raw-SQL boundary, recovery
   denylist, error precedence, and default-embedder behavior. Do not use a
   public wrapper merely to expose an internal function whose behavior has not
   been approved.
5. **Prove and review.** Run the three SDK surface/type/behavior suites, Rust
   default and `operator` feature checks, installed artifact consumer tests,
   the surface comparator, and `./scripts/agent-verify.sh`. Independently
   review the successor contract and implementation against the candidate SHA.
   Record the final matrix, evidence, exceptions, and exact artifacts before
   advancing to Slice 135.

## Boundaries

- This slice is not a mechanical decomposition and cannot claim the normal
  behavior-preserving cadence for Slices 40–130.
- It does not add a raw-SQL route, a recovery/admin SDK route, a second API
  generation, or a compatibility shim.
- It does not change the CLI's operator boundary merely to make raw symbol
  counts match.
- It does not preserve custom-provider reachability, remove a binding feature,
  or expose a companion trait without the HITL successor.
- Tags, publication, registry writes, and Pages deployment remain separately
  authorized.
