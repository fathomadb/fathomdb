# Slice 30 product architecture reconciliation

## Actual integrated paths

[Engine direct text](../../../../../src/rust/crates/fathomdb-engine/src/search_api.rs)
`search_text_only_filtered_with_snippets` compiles text and dispatches
`SearchReaderWork::text_only`; it never calls the query embedder or MCP registry.
The explicit direct-text marker carries its distinct candidate behavior. The
[Rust SDK](../../../../../src/rust/crates/fathomdb-sdk/src/engine.rs)
`search_text_only` delegates to it. Calling hybrid `search` with an absent
provider is a distinct route and is not a valid direct-text substitute.

[Search](../../../../../src/rust/crates/fathomdb-engine/src/search.rs) reads
`has_source_dependencies` once per reader transaction. The vector fallback
predicate is conditional on vector availability/dependencies. The surviving
`all_nodes_directly_eligible` scan is behind all deferred-text guards, including
vector availability, no dependencies/filter/history/recency/importance/rerank/
graph/explain/full-sort override. Direct text has an additional bounded rank
route when eligible. Therefore100k direct-text latency cannot by itself estimate
hybrid eligibility-cache benefit. Preserve each actual branch in attribution.

[Projection worker](../../../../../src/rust/crates/fathomdb-engine/src/projection_worker.rs)
`candidate_page_sql` and `scheduler_scan_sql` already bound each node/edge arm.
[Format37](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md)
already coalesces committed visibility publication.
[Projection commit](../../../../../src/rust/crates/fathomdb-engine/src/projection_commit.rs)
already shares generation/time/initial mean work per batch. Re-measure current
residual work; do not restore or count historical prototype fixes as new savings.

[SQLite profiling](../../../../../src/rust/crates/fathomdb-engine/src/connection_runtime.rs)
uses a per-engine context at statement completion. Public profile records expose
integer milliseconds and placeholder zero step/cache counters; they cannot prove
sub-millisecond phase work, SQL-specific exclusive attribution or absent work.
Keep observer overhead separate from untraced latency. Existing caches and
snapshot authority remain as mapped in [40 design](../slice-40/design.md).

[MCP registry](../../../../../src/rust/crates/fathomdb-mcp/src/registry.rs)
adds lifecycle/admission/epoch/store behavior before the
[bridge](../../../../../src/rust/crates/fathomdb-mcp/src/bridge.rs) reaches SDK.
Its existing wait log observes only calls that waited.20 A3 covers that lifecycle
gap; the two qualified20 hook repairs concern engine test seams.70 finalizes
provider outcomes; it does not close the MCP race proof or20 Windows evidence.

## Proposed maintained architecture clarification

Parent may add this paragraph under the current integrated profile in
[architecture v2](../../../../design/fathomdb-data-plane-architecture-v2.md):

> Release0.8.29 measurement attribution distinguishes direct text, hybrid
> retrieval, query embedding and MCP admission/lifecycle routes. Direct text
> dispatches without a query embedder or MCP registry; hybrid eligibility
> branches depend on actual vector/dependency/view state. Projection scans and
> visibility coalescing already include their accepted bounded/current-format
> implementations. Slice30 measures current residual work and read-mostly versus
> committed write→search schedules; its diagnostics create no cache, ranking or
> new numerical contract. Partial Linux measurements do not qualify held MCP,
> Windows or frozen TC5 fidelity obligations. See the Slice30 design.

No public API or product architecture change is needed to time the existing
routes. Any narrowly necessary feature-gated diagnostic observer remains a test
surface with a same-change Rust interface note, bounded owner and no new
production environment switch. The source-grounded distinctions above replace
stale Slice30 prototype/current-baseline equivalence claims.

## Implemented P tool boundary

The committed private SDK example wraps existing direct APIs, with no production
observer or environment switch. Natural API and governed-write clocks stop before
JSONL serialization/checkpoints. Loop throughput includes those costs and P2
writes/marker proofs; raw compiled-FTS matches stay separate from returned hits.
Snapshot observations remain unavailable. Recorder/resume/strict UTF8 framing
belong to measurement tooling, not database recovery or a new product contract.
[Results](results-p.md) describe actual current residual cost; filtered P2 versus
unfiltered P1 is not a controlled write-only treatment comparison. Authoritative
architecture paragraph above remains a parent integration proposal.

## Proposed G/D architecture delta

The [concrete extension](gd-extension.md) proposes one non-default engine-owned
bounded measurement feature, independent from broad test-hooks. Its collector
attaches to existing per-engine attribution ownership; query identity travels
to the same reader transaction. Normal builds omit instrumentation. Hidden
feature-only Engine accessors get a same-source-change Rust interface note;
SDK production/test surfaces remain unchanged. Current provider admission,
mean/CLS distinction, dispatch/default concurrency and worker commit ownership
are preserved. D synthetic providers stay in existing test-hooks attachment.
No global statement tracer, public production knob, cache or ranking change.
At349 this was a design proposal. The engine collector and bounded source
fixtures were implemented through46ee7cf6e with accepted prefix reviews.
The current source implements the private GN/GO host, G1/G2/GG and D routes, bounded owned
controller, catalog and paired result adapters. Assembled source qualification
and production lifecycle checks passed; natural campaign qualification remains pending.

## Generic G artifact ownership

The generic normal and diagnostic G callers use a SEPARATE feature-gated binary
in the same private host
in existing publish=false fathomdb-tc5-benchmark, as specified by
[the fit amendment](gd-extension.md#generic-g-ownership-and-route-fit-amendment-planning-only).
That package is only a private host: existing TC5 main/protocol/features remain
unchanged and disabled for G. Hosting is not TC5 fidelity evidence. Private
host features select normal SDK embed-cuda versus additional engine-owned
slice30-measurement directly; no SDK public feature/accessor forwarding.
Shared private Core/SDK callers and timers keep GN/GO comparisons source-matched;
Core observer collection stays engine-owned, outside caller API timers. D stays
in explicit test-hooks/measurement artifacts. Earlier SDK/engine examples and
all original proofs remain honest tiny-fixture identities, not relabelled as
this new host. No product ABI/ranking/cache/provider policy changes follow.
Controller-owned absolute clocks bound native phases/probes/cleanup
without making synchronous Engine APIs cancellable. The11e09 fit received exact
design PASS/source GO; GN is implemented with accepted bounded G1 source prefixes
through8e5823. The candidate now includes G2/GO/GG, supplied populations,
phase attribution, bounded controller/resource/device supervision and raw result
comparisons. Final source/native/lifecycle and natural-campaign qualification
remain open; earlier planning-freeze anchors retain their historical meaning.

## Proposed D worker ownership clarification

The [D11 separate-stream fit](gd-extension.md#d11-separate-worker-transport-and-shared-clock-fit-proposal)
keeps completed query collection and accepted GO transport intact. Per-engine
projection batch records have their own bounded buffer/drop count, actual
projection worker IDs and captured typed monotonic intervals. The private D tool
normalizes those intervals against its one writer/query/readiness attempt origin;
provider workers and configured concurrency remain distinct observations. No SDK
surface, scheduler notification/pause, product counter/policy or default-build
instrumentation is introduced. The exact c5/7c design received PASS and source
continuation; bounded D11 source transport now has a genuine RED/GREEN pair.
Held and ordinary writer/query/readiness adapters are implemented. Traced
attribution uses strict common overlap with a fully contained successful positive
readiness invocation; controls remain explicitly unavailable. Natural overlap,
full-source and campaign qualification remain unproven.
