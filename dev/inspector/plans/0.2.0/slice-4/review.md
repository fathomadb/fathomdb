---
title: Inspector 0.2.0 architecture and code alignment review
date: 2026-11-19
status: REVIEW
---

# Source agreement and feasibility

Current disposition: owner S9-010, applied in [Slice 10](../slice-10/status.md),
qualifies supported single head windows in [0.2.0 scope](../scope.md) and defers
within-source multiple-window positive. Earlier findings/draft proposals below
retain their historical meaning; cache-only CPU Engine serving remains mandatory
and unqualified in 40.

Actual review source `aef5b2eac19dbcf5850713081ccd427896909eea`, unchanged Engine
ancestry `6edc1155f`, packages 0.8.28/format 37. Read plan/design/filled prompt,
0–3 outputs, relevant Engine interfaces/accepted vector, visibility, erasure and
frozen-read decisions. Keep A/S/Q/P/O/R/U/K, scope and current owner map; adjust
old draft SHA/executor. No source delta implements a previously planned view.
The table below is source evidence, not executed qualification.

| Contract/component | Actual source and boundary | Result / proposed owner |
| --- | --- | --- |
| A/R record session | `worker.rs::Request/dispatch` permits Catalog, Check, Page, Sample, Detail, Chunk, Tree, Associations, Export, Close; `session.rs::attach` opens None with explicit small runtime bounds. | Existing foundation only. New lexical/vector/graph/compare dispatch and UI remain 30/40/50/60. 20 revalidates carried foundation, not a rewrite. |
| R/K owned cancellation/security | `process.rs::worker_command` env_clear and CPU model env; OwnedWorker process-group kill/reap; `server.rs` epoch/deadline/queue handling. | Keep isolation and late-result guard. UI cancel/session kill is not per-native-call interruption; each query feature owns close/cancel/aggregate proof. |
| A/S guarded reads | `admission.rs` checks shape/current format/initial sidecars/coalescing; Session opens request read transaction, checks identity/schema/coalescing. | Preserve current-only admission and per-request context. Engine-owned progress is not database replacement or a common snapshot. |
| P/S queued policy | `session.rs::ReadConnection::queued_erasure_contains` OnceCell belongs to one read transaction; bounds 16 scopes x16,384 cursors/8 MiB. `policy.rs::permits` independently composes inactive/erasure/telemetry. | Keep freshness at next transaction after withdraw/requeue. All new snippet/vector/graph/pinned deliveries reclassify. D-01 and correct physical deletion unchanged. 20 common, each view extension. |
| S/O exact content | `storage.rs::Reader`, opaque handles and `window.rs::Column` incremental BLOB/scalar reads; node/edge/operational locators remain distinct. | Extend typed identities/origins instead of text matching or retargeting to latest. 20 foundation, 30/40/50 origin detail. |
| Q lexical | `engine/src/search_api.rs` public filtered/view/frozen/reranked snippet methods, evidence/resolvers; ordinary projected-text methods; doc-hidden public `bm25f_search` takes tunable Bm25fQueryPlan and returns cursor/score tuples. | Stale no-snippet HLD assertion disproved. BM25F is callable Rust, not general SDK API; direct connection mutex path and no snippet option differ from reader-worker snippet routes. 30 qualifies each actual route/options/index/tokenizer/evidence/cancellation. |
| Q MCP reuse | MCP manifest/tool bridge is a separate stdio server over SDK with its own byte/queue/transport contract. | Prefer unchanged Engine calls inside existing worker. Reject copying MCP transport/envelope without fit; reject absence-based rationale, not whole reuse assessment. V01-S02/M36 stays required. |
| S/Q vector storage | `projection_commit.rs` ordinary vector commit and `vector_storage.rs` helper both insert vector_rowid=write_cursor, window_number=0, byte_start=0; `vector_spans.rs::validate` rejects different row/cursor, nonzero window/start and mismatched body length. Called from actual readiness/admission paths. | S4-WINDOW: layout has window metadata but current producer/admission enforce head-only. Mandatory F-07 supported multiple-window positive conflicts with unchanged selected substrate. No supported ordinary producer was identified. 9 must explicitly rule dependency/scope; 40 cannot fabricate rows or count refusal as success. |
| Q/R cached semantic serving | Production `open.rs::EmbedderChoice` has Default/None; injected Caller is separate TestEmbedderChoice behind test-hooks/tc5-benchmark. Default invokes `load_pinned_default_embedder`; `loader.rs` creates cache dir and can populate from HF or fetch/rename under lock. Cache-only local loader is pub(crate), tc5-gated. Public LoadedWeights/Candle CPU construction is standalone. | S4-CACHE: HLD helper-to-Caller route invalid. Standalone weights cannot attach to production Engine; None cannot acquire provider by direct query-vector injection in examined public API. Valid warm Default cache may avoid fetch, but it is not a no-fetch/no-repair contract on missing/corrupt/racing assets. 40 must establish a supported contained Default composition or external prerequisite before semantic/hybrid positive. |
| S/Q graph/history | `graph_api.rs::graph_neighbors` returns Vec<NodeRecord>, depth 1–3 and 50-result serving cap/eligibility. Canonical edges retain body/kind/endpoints/lifecycle; existing exact detail readers are available. | S4-GRAPH: public neighbor nodes do not supply complete stored historical/parallel edge facts. Add bounded Inspector-owned named adjacency/edge/revision reads and independent conformance under 50; frozen serving evidence retains its own meaning. No Engine API change implied. |
| O/U comparison | `app.js::observe/invalidate/evict` bounds ten observations/20 MiB and clears cached values on context changes. No two-pin comparison. | S4-OBS: preserve two actual observation contexts/typed sets while withholding expired payloads; no rerun, fabricated rank/absence cause/common snapshot. 60 and foundation 20; no generic experiment store. |
| K installed support | Default manifest and bundled assets are real; current package README ABI is 24.04/glibc 2.39. | S0-ABI/7 aliases, 70 final supported artifact. No all-view runtime qualification. |

## Feasibility decisions and minimal proposed changes

S4-CACHE is **unresolved**, not proof every composition is impossible. Explored
alternatives: production Caller rejected by accepted admission; test/benchmark
opener rejected for product; standalone cache weights rejected as proof of
Engine admission; normal Default rejected as unconditional cache safety promise;
None plus private vector-stage/ranker rejected as supported semantic serving.
A future Inspector-local warm-cache validation/contained Default design must
specify asset TOCTOU, CPU resolution, unavailable branches without attempted
fetch/repair, model residency and actual Engine serving/admission. Static
verification here cannot qualify it. Do not add Engine runtime/cache policy
changes under this commission. Observable acceptance: real valid-cache CPU
semantic/hybrid results with independent identities, denied network, unchanged
cache; missing/partial/corrupt inputs stay unavailable with no fetch/repair.

S4-WINDOW is a **source-established requirement/dependency contradiction**.
Format-35 metadata alone cannot satisfy mandatory multiple windows. Preserve the
criterion until an explicit human ruling changes scope or commissions a separate
Engine prerequisite. The valid head-window case and exact span inspection remain
useful but insufficient. No test-only writer/synthetic fixture makes it supported.

Propose updating maintained HLD factual snippet/Caller/window statements under
D2-D6-HLD; detailed route and view contracts belong to 30/40/50/60. Keep existing
responsibility boundaries. Reader currently couples storage/policy/reference
minting and app.js couples observations/history/rendering; extract a concrete
query adapter or typed observation/view module when an actual new caller needs
it. No prework generic DI framework, plugin registry or broad rewrite justified.

## Handoff and checks

S4-CACHE/WINDOW go to 9 as required prerequisites; S4-GRAPH/OBS/lexical route
work is ordinary adopted feature delivery, not a new Slice 10 implementation
bundle. 5 owns test adequacy; 6 exact stale-doc text; 7 source/build route impact.
Relevant requirements remain all INS/V01/X/F/T/High and M01–M40. Future tests use
real DBs, independent authored lexical/adjacency/L2 oracles, properties and
bounded cancel/resource/keyboard cases; no synthetic positive or unavailable
waiver. No build/test campaign or canonical/source/config edit ran. Independent draft verification and scoped-check limits are in [the shared record](../slice-9/prework-verification.md).
