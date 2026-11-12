# Slice30 G/D concrete extension design

Status: DESIGN PROPOSAL ONLY. Baseline349e9fa07d6c624affab69709735f35c18c0abae
contains accepted P at0d8be5bb75516955ebf99b94913b9b54921bb3a4. Parent integrated
P/state at2381de882 and the all-/mnt/data storage checkpoint at349e9fa07.
Independent Astra design PASS and explicit parent source/runtime GO remain
prerequisites. No G/D source, model load, DB, native build or GPU workload ran.

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
| projection_worker.rs candidate_page_sql/scheduler_scan_sql and runtime_configuration.rs | Current per-arm scan bound256, commit batch64, default orchestration2/provider5. Existing configured concurrency is observed, not retuned. active_jobs counts rows, not workers. |
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
public context reads never masquerade as its snapshot. No cache-key sufficiency
or future-time safety proof follows from these observations.

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
establishes observed worker concurrency, while a real overlap query must also
intersect an actual governed writer interval and positive pending work; commit intervals may serialize normally.
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
