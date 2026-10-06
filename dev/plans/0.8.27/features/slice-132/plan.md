---
title: FathomDB 0.8.27 Slice 132 — dedicated Rust SDK parity
status: PLANNED
target_release: 0.8.27
planning_baseline: 2f217f573
---

# Slice 132 — dedicated Rust SDK parity

This slice follows Slice 130 and precedes Slice 135. Build a dedicated Rust
application SDK with the same governed public surface as the Python and
TypeScript SDKs. The proposed package is `fathomdb-sdk` (`fathomdb_sdk` in
Rust imports). Its root `Engine`, `EngineConfig`, errors, types, standalone
utilities, and `read`, `graph`, and `admin` namespaces must map one-for-one to
the other SDKs. Rust syntax, borrowing, `Result`, and conventional snake-case
names are allowed translations; operation membership, argument meaning,
defaults, validation, result fields, error categories, and lifecycle behavior
are not. The SDK wraps the engine core through an explicit allowlist rather
than re-exporting `fathomdb_engine::Engine` wholesale. The existing `fathomdb`
crate remains a separately named lower-level crate until its disposition is
decided and documented; it is not the Slice 132 SDK deliverable.

This draft records the 2026-10-06 user direction, which supersedes the
2026-10-03 one-crate direction for Slice 132. The accepted
`ADR-0.8.0-supersede-five-verb-surface-cap.md`, `dev/interfaces/rust.md`, and
`docs/positions/sdk-parity.md` still describe a deliberately different Rust
consumer contract. Implementation must first ratify a successor ADR and
update all three interface documents. The [draft design](design.md) specifies
the intended boundary and the evidence needed to make that change reviewable.

## Entry inventory

The current Python/TypeScript checker validates 69 signed members mapped to 44
live canonical operations. Its map omits the documented standalone
`embed_batch_cls` / `embedBatchCls` operation, so 44 is an entry count, not a
complete target. It has no Rust input. Rust's own `governed_surface.rs`
compiles a 37-type positive allowlist and checks the shared five-name recovery
denylist. Those guards cannot prove SDK parity.

The difference matrix below is the entry review artifact. Its first rows cover
all 44 current Python/TypeScript canonical operations by family; the remaining
rows cover the types, configuration, extension, and boundary differences that
operation names alone cannot reveal. “Equivalent” is an entry observation, not
proof that argument defaults, errors, and result shapes match; the frozen
three-way inventory must expand every family into individual rows.

| Canonical capability and current operation IDs | Python / TypeScript state | Rust state at entry | Proposal or HITL question | Evidence and contract impact |
| --- | --- | --- | --- |
| Core: `engine.open`, `admin.configure`, `engine.write`, `engine.close` | Equivalent application capability, with Python/TS binding configuration and async differences. | Core has `Engine::open`, `write`, `close` and process-level `admin::configure_runtime`; signatures and administration semantics need mapping. | Add explicit SDK wrappers for the same root and namespace operations. Resolve every unmatched configuration field and error before accepting the contract. | `src/conformance/governed-operation-parity.json`; `dev/interfaces/{python,typescript,rust}.md`; successor must map defaults, range validation, and error precedence. |
| Dependencies and lifecycle: `engine.actuate`, `engine.register_source_dependency`, `engine.dependencies_for_source`, `engine.dependency_for_derived`, `engine.read_dependency_closure`, `engine.transition`, `engine.purge`, `engine.erase_source` | Equivalent governed commands in Python/TS. | Engine methods and re-exported receipt/domain types exist. | Preserve if the exact request, result, and error behavior maps; **OPEN HITL** for any unmatched Rust-only lifecycle/helper method discovered in the audit. | Shared operation map; `dev/interfaces/rust.md` lifecycle/erasure sections; recovery and typed-boundary ADRs remain binding. |
| Search and graph: `engine.search`, `engine.search_text_only`, `engine.search_projected_text`, `graph.expand`, `graph.neighbors`, `graph.search_expand` | Equivalent governed commands and binding-specific namespaces. | Engine search and graph methods exist, with additional Rust overloads/view forms. | Expose the governed graph calls under `graph::*`; inventory and disposition any extra core overload before exporting it through the SDK. | `src/rust/crates/fathomdb-engine/src/{search_api,graph_api}.rs`; interfaces’ search/graph sections; result/error shape audit required. |
| Frozen/evidence: `engine.freeze_read_context`, `engine.search_frozen`, `engine.search_expand_frozen`, `engine.search_with_evidence`, `engine.resolve_evidence`, `engine.resolve_graph_evidence`, `engine.trace_dependency` | Equivalent governed commands. | Matching Engine methods and evidence types exist. | Preserve only exact authority, expiry, and result behavior; **OPEN HITL** for a missing type or extra callable route. | Shared map; `search_api.rs`; `dev/interfaces/rust.md` evidence sections. |
| Reads: `read.get`, `read.get_many`, `read.collection`, `read.mutations`, `read.list`, `read.crossed_boundary_since`, `read.projections`, `read.projection_status`, `read.embedding_readiness`, `read.projection_generation_status`, `read.mutation_projection_status`, `read.canonical_page`, `read.operational_state`, `read.operational_state_page` | Equivalent read namespace commands. | Matching `read_*` / `crossed_boundary_since` Engine methods exist. | Expose the governed calls under `read::*`; inventory must prove parameter/result/error equivalence. | Shared map; `read_api.rs`, `projection_{registry,generation}.rs`; Rust interface read sections. |
| Search-hit identities and public result types | Python/TS expose `SearchHit` and `IdSpace` with a governed space discriminator. | `SearchResult` is exported, but Rust facade omits `SearchHit`, `IdSpace`, and the core `IdSpaceKind` discriminator. | Export SDK equivalents of the accepted identity/result types and discriminator behavior. | `dev/interfaces/rust.md` “SearchHit.id”; facade `src/rust/crates/fathomdb/src/lib.rs`; a facade re-export is a governed-surface delta. |
| Standalone rerank: `rerank` | Python/TS package-level operation has typed passages/results, validation, identity path, defaults, and feature refusal. | Internal `rerank_passages` exists; no documented facade entry or public passage/result surface. | Add a root `rerank` with the same request/result/error contract. | `dev/interfaces/{python,typescript}.md` standalone rerank; `fathomdb-engine/src/rerank.rs`; requires shared behavior fixtures. |
| CLS batch embedding: `embed_batch_cls` / `embedBatchCls` | Python/TS package-level pinned-BGE CLS operation, including empty input and feature refusal. | `Engine::embed_text` is mean pooled; no matching facade operation. | Add a root `embed_batch_cls` with the same input/output and refusal behavior; put it in the parity manifest. | Python/TS interface CLS sections; `embedding.rs`; pooling, cardinality, and normalization are contract data. |
| Direct embedding: `engine.embed` | Python/TS expose engine-attached mean embedding. | `Engine::embed_text` provides the likely Rust equivalent. | Preserve if exact default-provider, refusal, vector, and error semantics match. | Shared map; `embedding.rs`; public Rust spelling is an idiom, not a capability exception. |
| Custom provider injection | Python/TS deliberately support default or no embedder only. | Whole-Engine re-export leaves `open_with_choice[_and_config]` and `EmbedderChoice::{Caller, CallerWithDeviceResolution}` reachable through the engine core; `fathomdb` does not itself re-export the choice type. | Exclude from `fathomdb-sdk`; separately resolve the existing `fathomdb`/engine/plugin crate posture. Do not add it to Python/TS. | `fathomdb-engine/src/open.rs`; `fathomdb-embedder-api` README; successor ADR must distinguish SDK from extension reachability. |
| CLI operator/recovery and test hooks | Python/TS SDKs exclude recovery; test hooks do not ship. | `operator` feature is the CLI seam; engine also has test-only/provider injection routes. | Excluded, retain only as CLI/internal boundaries. Do not include merely to equal raw symbol counts. | `fathomdb/src/lib.rs`, `fathomdb-cli`, operator gate and recovery denylist contracts. |
| `fathomdb`, `fathomdb-engine`, and `fathomdb-embedder-api` external posture | Bindings do not expose engine core or an embedder plugin trait. | Existing `fathomdb` reexports the engine; engine and embedder API are published. | **OPEN HITL:** audit real external use and choose their post-SDK compatibility, documentation, and extension posture. This does not add an SDK operation. | Cargo manifests and README files; successor ADR must state package positioning and migration posture. |

The execution inventory may add rows but may not silently close a gap. A Rust
facade method or re-export cannot be declared out of scope merely because the
existing Rust test does not enumerate it.

## Appendix A — per-operation entry map

At entry, `fathomdb::Engine` is a wholesale public re-export of
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
| `rerank` | **MISSING** | New SDK root operation required | Python/TS interfaces; engine `rerank.rs` is not a facade operation |
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

The shared map currently omits `embed_batch_cls` / `embedBatchCls`. It is a
documented Python/TypeScript capability and must become a signed row in the
three-SDK parity map.

## Appendix B — provisional Rust-only inventory

This is a source-grounded, non-exhaustive intake list. It gives HITL concrete
items to classify now and directs the implementation inventory to discover
more; it does not claim that every item is an extra capability until the
installed three-SDK comparison establishes behavior.

| Rust item or family | Initial classification | Provisional recommendation | Evidence / reason |
| --- | --- | --- | --- |
| `Engine::open_with_choice`, `Engine::open_with_choice_and_config`, `EmbedderChoice::{Caller, CallerWithDeviceResolution}` | Rust-only provider reachability | **OUT** of `fathomdb-sdk`; separately decide existing-crate disposition after external-need audit | `open.rs`; same capability is deliberately absent from Python/TS. |
| Existing `fathomdb`, `fathomdb-embedder-api::Embedder`, and published engine/plugin crates | External extension contract, not an SDK operation | **OPEN HITL** retain/deprecate/retire after real-user audit | Cargo manifests and plugin README promise semver stability. |
| `search_with_limit`, `*_with_limit`, `search_filtered`, `search_filter`, `search_reranked`, `search_explained`, and `*_view` variants | Potential idiomatic overloads or extra option combinations | **UNCERTAIN** until matched against Python/TS parameters and signed view contract | `search_api.rs`; Python/TS expose options on some searches, while interface view sections are marked proposed. |
| `Engine::drain`, runtime counters, subscriber/control methods, `runtime_configuration`, and slow-threshold controls | Operational capability outside the 44-operation map | **UNCERTAIN**: map to equivalent binding controls or present individual feature-in/out decisions | `runtime_lifecycle.rs`, `open.rs`, facade re-exports; not evidence of equal behavior. |
| `write_node_importance`, `node_importance`, `bm25f_search`, and other public non-test Engine methods outside this appendix's map | Possible Rust-only application behavior | **OPEN**: establish whether public/advertised, equivalent elsewhere, or remove/gate | `write.rs`, `read_api.rs`, `search_api.rs`; no claim of binding absence until full audit. |
| `operator`-gated recovery/integrity methods and report types | CLI operator seam | **OUT** of SDK parity | `fathomdb` `operator` feature and recovery-denylist tests. |
| `*_for_test`, `write_vector_for_test`, test hooks, and private benchmark routes | Test-only or feature-gated harness seam | **OUT** | engine source and test-hook feature declarations; no shipped SDK feature. |

## Decisions and contract gate

The 2026-10-06 direction settles the product goal: a dedicated Rust SDK whose
public operation, type, error, configuration, and namespace surface matches
Python and TypeScript. The [draft design](design.md) proposes a separate
`fathomdb-sdk` crate so the existing wholesale engine re-export cannot leak
core-only methods into the SDK. The follow-up ADR must ratify that packaging,
supersede the accepted `BIND-RUST` exception, and update the interface docs
before public code changes. It must enumerate the exact allowed Rust syntax
translations and the complete signed inventory, including standalone rerank
and CLS embedding.

The existing `fathomdb` crate and the published engine/embedder extension
contracts need a separate disposition based on real external use. Their
post-SDK status does not dilute the dedicated SDK surface requirement. Keep
custom provider injection, CLI recovery, and raw SQL out of `fathomdb-sdk`.

## Requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-132A | Ratify one three-SDK public contract. | AC27-132A: a successor ADR, all three interface documents, and `docs/positions/sdk-parity.md` define the same namespace, operation, type, error, and configuration set, plus exact Rust syntax translations and excluded operator/extension boundaries. No public SDK change precedes this gate. |
| R27-132B | Ship an independently importable Rust SDK. | AC27-132B: an installed consumer imports `fathomdb_sdk::{Engine, EngineConfig, read, graph, admin, rerank, embed_batch_cls}` without importing `fathomdb_engine`; a compile-fail fixture cannot reach an unapproved engine, provider, raw-SQL, or recovery member through the SDK. |
| R27-132C | Every signed command and data contract matches. | AC27-132C: a three-SDK manifest includes all current governed operations plus documented CLS embedding, and every public DTO, config option, error category, default, and result field. Fail-closed checks reject missing, extra, renamed, or incompatible SDK members. Consumer fixtures prove `SearchHit`, `IdSpace`, and the Rust representation of its space discriminator. |
| R27-132D | Shared behavior matches through installed artifacts. | AC27-132D: human-authored cross-SDK fixtures cover open/configure/close, read and graph namespaces, search and evidence, rerank validation/defaults/ordering/refusal, CLS cardinality/order/normalization/empty input/refusal, and error precedence. |
| R27-132E | Core-only capability is contained. | AC27-132E: the SDK exports no custom provider injection, CLI recovery, or raw SQL; the separate existing-crate disposition is recorded. An adversarial test inserts one core-only public method and proves it cannot become an SDK operation without an explicit export and parity-manifest change. |
| R27-132F | The reviewed candidate is release-ready. | AC27-132F: package imports, Rust consumer compilation, Python stubs, TypeScript declarations, installed crate/wheel/npm tests, existing security guards, and the strict full repository gate pass on one candidate SHA. Docs and release notes describe the new SDK and any intentional break. |

`dev/acceptance.md` remains locked; these IDs are release-local.

## Execution order

1. **Freeze source and installed surfaces.** Capture the Python runtime/stub,
   TypeScript runtime/declaration, current Rust facade/core, and exact installed
   artifact exports at the post-Slice-130 SHA. Expand the 44-row map with CLS
   embedding; enumerate every public DTO, error, config field, default, and
   behavior. Resolve existing Python/TypeScript differences before asserting a
   common contract.
2. **Ratify the contract.** Review the three-way inventory and the existing
   crate/plugin consumer audit. Adopt a successor ADR and update the three
   interfaces and parity position. Record exact SDK membership, Rust
   translations, existing-crate disposition, and exclusions. Slice 135 remains
   blocked until Slice 132 closes.
3. **RED.** Add human-authored consumer compile/runtime and cross-SDK behavior
   fixtures that fail on the current Rust surface. Add fail-closed manifest
   checks, including adversarial missing, extra, and shape-drift cases. Stage
   or commit the failing tests before implementation; do not edit them to make
   implementation pass.
4. **GREEN.** Add the `fathomdb-sdk` crate with a private engine field, explicit
   operation adapters, governed namespace modules, typed DTO/error mapping,
   standalone rerank and CLS embedding, and package docs/examples. Reuse core
   semantics when exact; implement missing translation explicitly. Do not
   expose a core type wholesale through a public signature.
5. **REFACTOR and prove.** Run the three SDK contract suites, installed consumer
   tests, security guards, and strict `./scripts/agent-verify.sh`; review the
   code and contract against one candidate SHA. Record the final matrix,
   exceptions, and artifacts, then close Slice 132 before Slice 135.

## Boundaries

- This slice is not a mechanical decomposition and cannot claim the normal
  behavior-preserving cadence for Slices 40–130.
- It does not add a raw-SQL route, an operator recovery SDK route, or a
  compatibility shim.
- It does not change the CLI's operator boundary merely to make raw symbol
  counts match.
- It does not expose custom-provider injection or a companion trait through the
  SDK. Existing crate and plugin compatibility is separately decided.
- Tags, publication, registry writes, and Pages deployment remain separately
  authorized.
