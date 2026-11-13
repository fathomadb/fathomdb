# Slice30 G/D concrete extension design

Current execution: reviewed design PASS and parent source GO at5968884b.
The reviewed G/D routes are implemented in the source candidate; see [source status](execution-gd.md).
Historical design-stage statements below describe the349/b83 review subject,
not current runtime authorization or completed qualification. No G/D natural
campaign or whole-slice acceptance is claimed.

Historical design baseline349e9fa07 contains accepted P at0d8be5bb;
parent integrated P/state at2381de882 and all-/mnt/data storage at349e9fa07.
Subsequent design reviews and source GO authorized the implemented G/D routes.
Final source/native/lifecycle qualification and natural campaigns remain open.

## Scope and accepted P boundary

P measured unfiltered100k direct text at27.184–27.509ms p50 and
106.911–107.682ms p99. Filtered P2 p50 was108.745–110.402ms. Different filter/
route/schedule means that gap cannot be called write cost. P's zero-vector
stores cannot quantify hybrid eligibility scans, query embedding, projection
concurrency or drain. Preserve P raw/freeze/old environments unchanged; no rerun
or historical prototype substitutes for current G/D evidence.

G measures the pinned built-in BGE384 on the same current-format store through
existing core and SDK hybrid/direct-text/mean-embed APIs. Separate read-mostly
and committed write→drain→proof→query schedules. A filtered read-only control
shares G-write's kind filter and marker population. No inference from subtracting
hybrid/text/embed percentiles; report every population and paired raw-call
comparison with its actual route/observer identity.

D measures deterministic768-d projection fixtures at10k/100k, held versus actual
writer/worker/query overlap, pending work, individual worker intervals, WAL and
drain.300k is conditional on measured lower-scale time/storage feasibility.
Synthetic arithmetic providers are test fixtures, not BGE semantic/GPU stand-ins.
No ranking/cache/ANN/candidate/default/CS64/overflow/format/migration/fork change.
W0.8.29/E0.8.0 and accepted numerical contracts remain fixed.

M still needs actual20 A3/A5 policy/proof/disposition. Windows20 A6 and pre90
Block1 remain held. F needs the exact frozen TC5 pack/current qualified proof;
LME, generated synthetic bodies and convenience10k are not substitutes. Whole30,
40/50 source and final160 exits remain open. G/D have no new numerical gates.

## Current source fit and net-new work

| Existing source | Actual fit and limit |
| --- | --- |
| SDK engine.rs Engine::search/search_text_only/embed; options.rs | Core delegation and mean-pooled embed already exist. Standalone embed_batch_cls is CLS and is excluded. No new SDK API/test accessor. |
| engine embedding.rs embed_text | Engine-owned dispatch, current provider deadlines/capacity. Embed-only includes dispatch/API cost; it is not a pure tensor-kernel timer. |
| engine open.rs and accepted vector-compatibility ADR | Production Default/None only. G uses Default with own pinned cache; D uses existing hidden open_with_embedder_for_test under test-hooks. No production Caller restoration. |
| search.rs read_search_in_tx at349 | has_source_dependencies once/snapshot; vector fallback short-circuits on history/vector/dependencies; all_nodes_directly_eligible behind all current guards. P direct text does not execute that hybrid eligibility scan. |
| dependency_closure.rs predicate functions | Exact existing predicates. No-dependency fallback returns without the dependency SQL; all-nodes scan is not a blanket per-search cost. |
| connection_runtime.rs profile callback; search.rs Slice71 trace | Public milliseconds/zero placeholders and global label-only trace cannot supply exact per-query predicate timing/snapshot attribution. Do not extend/use that global trace. |
| projection_worker.rs candidate_page_sql/scheduler_scan_sql and runtime_configuration.rs | Current per-arm scan bound256, commit batch64, default orchestration2/provider5. Existing configured concurrency is observed, not retuned. active_jobs counts ProjectionJob entries, not workers; D node jobs correspond to cursors, not a general physical-row count. |
| projection_runtime.rs freeze/readiness; runtime_lifecycle.rs drain | Existing held fixture, passive drain and durable pending predicates. Terminal/drain alone is not vector coverage. Held and true overlap are different cells. |
| tests/exp_ac013_page_cache.rs; dev/experiments/ac013-page-cache | Selectively reuse deterministic body/vector generator and read-only page/WAL observation. Its30minute drain, old knobs, full-sort/PRAGMA/libsqlite flags and stale digest assumptions are excluded. No wholesale prototype merge. |

Authoritative constraints: accepted vector compatibility/open policy, engine-owned
runtime topology/default dispatch capacity, dual runtime/CUDA policy, format35
vector span/CS64 and format37 transactional visibility ADRs. The current quality
roadmap supports real DBs, branch boundaries, bounded resources and lifecycle
cleanup. No general fault/fuzz/mutation/MC/DC program or new release gate.

Net-new scope is committed G/D tools, feature-contained engine measurements,
independent measurement tests and maintained suite registration. Existing
product correctness coverage remains unchanged and is not relabelled new RED.
See [test map](gd-test-map.md) and [prospective cells](gd-preregistration.md).

## Minimal engine-owned diagnostic observer

Propose non-default engine feature slice30-measurement, independent of test-hooks;
no facade/binding forwarding or production environment switch. Normal G build
is SDK embed-cuda only. Diagnostic G adds engine/slice30-measurement, not the
broader test-hooks feature. D's existing synthetic opener/held controls require
test-hooks; its fixture artifact is labelled accordingly, never production BGE.

A private module owns a bounded32768-record collector per Engine, using the
existing engine-owned WalAttributionCollector Arc already carried to readers and
projection workers. Add only a feature-gated collector field there; do not alter
WAL attribution policy or expose raw SQL/user bodies. No process-global tracer.
An internal call ID travels in cfg-gated SearchReaderWork metadata to the same
reader transaction. Hidden feature-only Engine arm/drain methods expose records
to core diagnostics; update dev/interfaces/rust.md with the exact test surface
in the source change. SDK remains unmodified: SDK calls supply ordinary totals;
engine-direct diagnostics alone supply internal branch attribution.

Record per-engine ID/call ID/worker, actual FrozenView effective_at, projection cursor and
read-visibility generation loaded on that SAME reader transaction. The extra
diagnostic generation read is timed/labeled observer work; adjacent writer or
public context reads never masquerade as its snapshot. The initial snapshot-stamp RED includes the bounded post-pin/concurrent-commit
race in [test map](gd-test-map.md), not just a quiescent adjacent-generation
comparison. No cache-key sufficiency or future-time safety proof follows.

Bracket actual calls/blocks with monotonic ns: dependency-presence predicate,
vector-fallback predicate/its SQL branch, guarded all-nodes predicate, vector
arm and text arm. Record invoked/boolean-or-typed-error outcome/fine duration versus not_run(reason)/null,
including vectorless/history/filter/dependency/time controls. Record actual
vector-arm degradation; a sparse/fallback result is not labelled complete dense BGE hybrid. Preserve existing
short-circuit order and errors; never evaluate a skipped predicate for reporting.
Nested spans are inclusive and not summed as exclusive shares. Query embedding
is a separate actual API population; no percentile subtraction or inferred
SQL/kernel/queue partition. Any unsupported attribution stays unavailable.

For D, collect per-worker batch start/finish and row count around the existing
worker batch lifecycle, plus actual queued/active/in_flight state snapshots.
Do not equate active_jobs with worker concurrency. Worker-ID interval overlap
establishes observed worker concurrency. Real triple-overlap requires the strict
COMMON intersection max(query_start,writer_start,worker_start) <
min(query_end,writer_end,worker_end), not pairwise query intersections. A positive
pending-read witness must lie wholly inside that common interval as specified in
[preregistration](gd-preregistration.md). Boundary-only contact does not qualify; commit intervals may serialize normally.
Read-only counts/page/WAL observations include the live WAL (no immutable=1).
Exact pending membership and per-kind vector/spans are independent completion
oracles, not just a cursor or terminal flag. No scheduler notification/control is
added by the observer; this is not20 A5's held proof.

Arm/drain outside API timers. Consume records after each query or bounded writer
cohort; overflow count is explicit and missing attribution makes that diagnostic
incomplete. Disabled/compiled-out records are unavailable, not zero elapsed.
Feature/mode-off/mode-on artifacts/cells have separate identities and paired
raw-call observer comparisons. The whole feature difference and recording/
checkpoint cost remain visible; no adjustment to untraced latency.

## One source phase, bounded campaigns

Use a new indexed G/D controller/cells and exact SDK example next to existing P
tools; keep P files/cells/raw freeze unchanged. Reuse tested converter/metrics/
checkpoint/prefix/resume primitives without rewriting them. New templates,
route populations, dense verification, observer records and D schedule behavior
get their own genuine runtime RED before separate GREEN. Commands/counts/
features and platform/manual selectors must be maintained, not globally ignored.

One coherent source phase is justified because G/D share the same per-engine
collector, checkpoint/framing/maths and owned resource machinery. Scoped checks
while implementing; regenerate applicable inventories before one final clean
full agent-verify with canonical jobs1. Run feature observer suites/actual owned
native controls as maintained affected scopes, retain every failure/skip/ignore,
then freeze actual source/binary/model/template/cells before campaigns. Build
outside timing. No source edits during gate or campaign; failed affected scopes
alone resume unless a broad change justifies more.

G and D serialize, with explicit separate retained-output budgets and one shared
4hour campaign clock including preparation/waits. This proposal requests at
most16GiB G plus16GiB D DB/WAL/raw retention (32GiB combined), within unchanged
200GiB aggregate owned/resource floors including retained P task/cache/target,
old P checkout, new G/D checkout/setup and shared data-backed Git once. The
design-entry receipt counts only the new root and is not that future aggregate. It is an explicit prospective retention proposal,
not a silent reuse of P's16GiB combined cap or runtime permission. Separate
families preserve fresh DBs/partial outcomes without deleting them to hide cost.
Every preparation/retrieval/drain cell retains900s cap. Parent/Astra must accept
these exact bounds before source/runtime GO; otherwise narrow the matrix.

No model fetch, new dataset, driver/toolkit/primary CUDA change, paid session,
fault/reset/VM/Memex/CI/publish operation. Actual missing inputs/feasibility are
reported precisely; no generic cold-drive request or unrelated Windows prerequisite.

## Generic G ownership and route fit amendment (planning only)

The heading preserves the11e09 planning-freeze anchor. That exact fit received
independent design PASS/source GO. The candidate implements GN/G2/GO/GG,
D held/ordinary routes, owned bounded execution, catalog and result adapters.
Bounded source tests do not establish natural-campaign or final qualification. The following artifact/identity/method constraints remain in force.

Accepted source prefixes through46ee7cf6e cover the observer, actual API/tiny
closed-template tools and bounded SDK-text actuation; they are not generic
campaign routes. The accepted host is existing publish=false fathomdb-tc5-benchmark:
new separate bin slice30-g-measurement at src/slice30_g.rs, required-features
slice30-g. Its default=[] and existing TC5 features/bin/main/protocol remain
unchanged. Building G must NOT enable tc5-benchmark, tc5-benchmark-cuda or
engine test-hooks. No TC5 assets/protocol/evidence or fidelity claim is reused.

Private host feature slice30-g enables optional workspace SDK/query/embedding
API/embedder edges, SDK's EXISTING embed-cuda, rusqlite0.41(bundled/fallible_uint)
and libc0.2 for Linux CLOCK_MONOTONIC. All resolve existing locked versions
(rusqlite0.41.1/libc0.2.186); no root/production dependency or SDK feature change.
Private slice30-g-observer adds only slice30-g plus engine/slice30-measurement.
GN uses the first feature; GO-off/on use the SAME second-feature executable and
runtime observer flag. Existing engine-example self-dev test-hooks cannot supply
GO; those examples remain D/test fixtures with honest identities. SDK accessors
and observer forwarding remain absent. Existing private TC5 host ownership does
not authorize modifying its TC5 executor or enabling its benchmark features.

Factor private API callers/Recorder into shared tool modules, preserving legacy
fixture arguments/oracles. Both new host builds use those same Core/SDK public
callers and API clocks. Observer cfg references live only in the private host;
Core-only GO arms/drains existing Engine records OUTSIDE API timers. No SDK
public test accessor is added. GO-off still pays its actual compiled/control
cost; GN is the distinct uninstrumented build. Mean embed has no reader query
record: mark query observer not_applicable, never invent a SnapshotStamp or
predicate zeros. Search success expects correlated engine/call records; missing,
error-route unavailable, drops and unpaired records remain explicit.

Generic G1 consumes qualified closed copies and arbitrary ordered query records,
not hardcoded alpha/beta/one-query fixture assertions. Retrieval limit10 applies
to hybrid/text; mean remains Engine::embed_text/SDK::embed,384finite/nonzero.
Actual argument query fingerprint is computed beside the call from the SAME
query slice (not from a separately planned ID); this plus literal text IDs tests
routing without learned-vector goldens. Real counts, profile/span/readiness,
copy/close and reserved CUDA controls precede each intended tool RED.

GG uses Core/public ProvenancedNodeV1, WriteProvenanceV1 and dependency
registration, then transition(source,Deleted,None), as current predicate tests
demonstrate. Time copy uses explicit ReadView/public view search: baseline100, then one
valid90..110 row at before80/inside100/after120, without supersession side effects. Literal direct-text checks establish
expected visible IDs; timed hybrid stages report actual guards/degradation, not
learned rankings. Source-valid→invalid and no-dependency/time-window copies are
separate; actual100k+control-row totals and generations are recorded. No test
recency/importance/full-sort override enters a campaign.

Native deadline admission uses Linux CLOCK_MONOTONIC, shared with controller
monotonic_ns; optional libc is tool-only, not a product cancellation API. Check
before every open/copy/write/warmup/API/drain/proof/verification phase and pass
floor(remaining work ns/1e6) to drain; refuse zero, never refresh a5s/30s budget.
Engine::close has no timeout and can wait for SQLite owners; only the owned
controller bounds that phase/process group. Blocking du/GPU probes also need
remaining-budget timeouts; existing unbounded measure_root/resource_sample are
not a hard-deadline campaign implementation. Keep source guard defaults and
already accepted cleanup/resource semantics unchanged unless a separately
staged bounded-tool unit requires an optional campaign-deadline path.

## D11 separate worker transport and shared-clock fit (proposal)

Source subject26cb528230e6137c299b0118da9a066e95420683. In
projection_worker::projection_worker_loop, worker_idx is the actual projection
thread. The popped Vec<ProjectionJob> forms a batch; queued_jobs/active_jobs
count jobs, in_flight contains cursors. Provider embed_dispatch::core worker
indices/live_workers are a DIFFERENT population; defaults2/5 describe configured
projection/provider topology, never observed concurrency. Existing catch_unwind,
commit failure reporting and mandatory state cleanup retain their order/results.

Keep take_slice30_measurements_for_test and its query-only capacity/drop contract
unchanged. Propose TWO hidden Engine methods under slice30-measurement only:
slice30_observe_projection_workers_for_test(bool) and
 take_slice30_projection_measurements_for_test(origin: std::time::Instant)
 -> Result<serde_json::Value, EngineError>. The setter returns Result<(),
EngineError>. No SDK forwarding, environment switch or capacity knob. Add the
exact Rust interface note with implementation, not a claim of an existing API.
Both use normal admission and return Closing after close; draining a poisoned
worker buffer returns Storage, without altering query or product error paths.

The existing per-engine attribution owner carries a SEPARATE worker buffer,
fixed32768 completed batches and independent cumulative dropped counter; query
records/counters are not mixed or reset. The drain report has records/capacity/
dropped, removes completed records in completion order, and reads its counter
under the same worker-buffer lock. Drops accumulate for the Engine lifetime;
arm/disarm/drain never reset them. The two streams retain at most65536 records
combined, each with its own32768 bound; no new resource allowance. Starts sample
worker enablement; disabling
retains records and permits already observed batches to finish. Normal compiled-
out behavior and GO's exactly-one-query-record contract remain unchanged.

Capture typed Instant start/end with engine_id, separate monotonic batch_id,
actual worker_idx, job_count and captured cursor list. Bound each list by existing
PROJECTION_COMMIT_BATCH64; no bodies/SQL are collected. Begin follows removal/
active accounting and precedes run_projection_jobs. Explicit finish follows the
existing catch/recovery, error reporting and mandatory state cleanup. Its inclusive
interval is NOT provider kernel time or SQLite transaction duration. Retain actual
commit Ok/Err and caught-unwind disposition separately; Ok does not prove every
job projected successfully. Per-job physical/pending/failure controls remain.

The ended-record disposition is finished only after explicit post-cleanup finish;
a guard abandoned by early error/unwind emits aborted at Drop, with cleanup
unconfirmed and unknown commit outcome null. A poisoned/unavailable cleanup state
is explicit, never successful cleanup. Observations cannot swallow/replace the
original error/unwind or run product cleanup themselves. Sample actual queued_jobs,
active_jobs and in_flight membership count after entry accounting and after cleanup,
with sample timestamps; they may include other workers' jobs and are not atomic
batch-start facts or physical-vector counts. Serialize outside the state lock.
No new scheduler notification, rendezvous or runtime configuration follows.

The tool creates ONE attempt-local Instant before any writes/queries/readiness
and passes it to the worker drain. Typed worker timestamps convert relative to
that SAME origin outside API clocks; driver writer/query/readiness intervals use
origin.elapsed directly. Do not independently zero intervals or use reader-worker
IDs as projection IDs. An origin after an event produces null offsets plus an
explicit unavailable issue, not saturated zero/qualifying overlap. This is an
in-process clock, not cross-process epoch arithmetic. No query schema change is
needed. Actual D controls share this load; no observer-controlled scheduler pause.

A completed-buffer drain cannot preserve an interrupted in-flight batch. Report
missing completion/partial output honestly; no every-start durability claim.
Worker/driver phase persistence and absolute deadline integration remain later
units. Separate stream bounds add no budget: same13roots/original69280403456,
96GiB setup/G16/D16/200GiB aggregate and existing floors. Source fixtures remain
180/150/30; no large population, artificial timed delay or campaign authority.
