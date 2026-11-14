# Slice 30 M continuation — proposed real stdio measurements

Status: **PROPOSED**, with independent Astra design PASS at reviewed
ca76af9f5c7c710b1aca7f8ee79f2c90549a4874. This document
refines held M only; it grants no source, setup or runtime GO. Accepted P and
executed G preregistrations/freezes remain unchanged. No qualified selected G
store exists at this design entry. The first G0 attempt lacked its required
own-PID memory plus GPU-utilization sample; the separately reviewed polling
correction and running G source gate do not qualify a store for M.

## Inherited scope and predecessor fit

[Requirements](requirements.md) R30-1..9 and [acceptance](acceptance.md) A30-1..7
remain unchanged. Inherited M dimensions are real stdio c1/4/8/16/32, honest
queue/all-call denominators, errors/cancel/timeouts, first-process/open/idle/
alternating-store costs and logging controls. The exact counts/modes/formats
below are **new preregistration proposals**, not already accepted gates.

Current parent973b2e692f54b2944d809cec2367ff53b8c152d0 includes accepted20 A3/A5
f803a964ad8c8154a481e6f9385599bef5c0e054, accepted60 subset
5271c1bf022f0ea22f95f7433e226feb2315043e and complete70
e875f36042f89c1d84756f78135db0e8926c1ed4. The [G/D fit](continuation-gd.md)
records their exact limits. A3/A5 satisfies that named M prerequisite;
whole20/WindowsA6 is independent.60 supplies compact-default/structured formats,
filters and budgets, but F11 remains open.70 supplies current provider absence,
overload, timeout, Closing and CUDA-kind outcomes; M must preserve these words
and remedies rather than collapse them to a generic error.

| Existing obligation | M refinement and retained boundary |
| --- | --- |
| R30-1; A30-1 | Build against actual20/60/70/current schema37; replace obsolete example assumptions, not production algorithms. |
| R30-2/3; A30-2/3 | Reuse qualified read-mostly100k input and query order. M is not a new write/overlap proof or AC083 result. |
| R30-4/5; A30-4 | Attribute only client clocks, returned outcomes and actually logged waits; exclusive service/open/render costs remain unavailable. |
| R30-6/7; A30-5/6 | Small real-process/real-DB tool TDD, committed/indexed commands, bounded streams/attempts/resources and honest incomplete cells. |
| R30-8/9; A30-7 | P/G/D/M/F remain separate. No M timing closes frozen TC5 fidelity, F11, Windows, whole30, Block1 or downstream gates. |

[Release requirements](../../../../requirements.md#performance-req-009req-018)
and [canonical acceptance](../../../../acceptance.md) retain REQ-010/AC012/076
text, REQ-011/AC013/072 vector, REQ-017/AC019/073 stress and AC075 fidelity
protocols. MCP latency is none of those substitute protocols. REQ-013/014
canonical/FTS freshness and REQ-015 projection freshness remain independent;
REQ-018 is not proved by client concurrency alone. AC083 remains tracked,
nongated100k LME direct-text80/300ms. No canonical acceptance edit, new numerical
pass threshold, calibrated verdict, cache/ranking/default change, schema change,
migration or backward-compatibility shim is proposed. Preserve built hints and
stored embedding spans. Apply the existing quality-roadmap real-boundary,
independent-oracle and resource-release recommendations, without broad fuzzing.

## Current code reuse and tooling cut

Reuse framing/IDs and closed-loop concepts from the current
[ac013_mcp example](../../../../../src/rust/crates/fathomdb-mcp/examples/ac013_mcp.rs),
not its synthetic corpus/CPU seeding, inherited performance knobs, debug default,
structured-only answer decoder, post-flush start clock or unmatched-wait service
subtraction. Reuse accepted converter/statistics,16-record checkpoints, strict
UTF8 prefix retention, fresh attempts/resume and owned-child lifecycle concepts
from the [measurement tools](../../../../../dev/experiments/0.8.29-slice30-measurements/README.md).
No wholesale prototype merge or new production observer is needed.

Proposed future tools belong in that existing experiment directory: a separate
M cell inventory and client/controller, with bounded tests and commands indexed
in its README. They do not exist or execute by virtue of this proposal. Keep P/G
entry points and freezes unchanged. Current production launch is exactly
`fathomdb-mcp --config <path.json>`; `--check-config` validates without opening.
Normal M0 uses a locally built no-feature binary, explicit closed tiny stores,
`use_default_embedder:false`, `search_mode:text`, and each result format in turn.
There is no provider-disable environment flag. Production M-R/M-C/M-L use an
owned locked release `fathomdb-mcp` binary built with `embed-cuda`, existing
`FATHOMDB_EMBED_DEVICE=cuda:N` and `FATHOMDB_MCP_LOG=warn` or `debug` as registered.
Ordinary timing excludes `test-seams`, test hooks, observer and telemetry.

The [MCP contract](../../../../interfaces/mcp.md) owns exact formats/defaults.
Discover the existing seven tools: list_stores/use_store/orient/search/fetch/
expand/feedback. Do not modify discovery. Compact success/errors must be decoded
from the text block; absent structuredContent is not an empty successful result.
MCP search always requests match-centred snippets and retains hints/budgets;
G's snippet-free SDK route is a different workload. Their difference cannot be
called exclusive adapter or query-embedding cost. G embed-only remains separate,
not a quantity subtracted from MCP hybrid latency.

## Proposed inventory and fixed schedule

Each table row is held. IDs below define the future explicit inventory;
`r=1,2`, `c=1,4,8,16,32`, `f=text,structured`. Text here names result format;
search mode is separately stated. The natural input is the qualified100k G
read-mostly subject, unchanged500 queries, default current view and limit10;
no filters, highlights, explain, rerank, graph option or writes are added.

| IDs | Cells | Exact proposal |
| --- | --- | --- |
| M0-f |2| Provider-free normal-CLI tiny real-DB smoke; initialize, discover seven tools, literal search/fetch/orient, typed error and normal stdin-close/exit. No natural/model workload. |
| M-R-r-c-log |20| Compact hybrid, five concurrency levels, two repeats, each warn/debug. Fresh server per cell; separate first call,500 sequential warmups,1000 timed calls. |
| M-C-r-modeformat |4| Warn c1 controls: compact direct text and structured hybrid, two repeats each; same first/warmup/timed counts and queries. |
| M-L-r-case |8| Compact/warn hybrid lifecycle: lazy, eager, idle and alternating A/B/A, two repeats each; exact calls below, no percentile claims from tiny counts. |
| M-E-f-case |12| Tiny private-helper stdio diagnostics: queue_full, wait_deadline, queued_cancel, running_cancel, run_limit, open_deadline, each format. No ordinary latency claim. |

M-R/M-C are24cells and36024 search submissions including first calls/warmups;
timed samples are24000, with1000 per cell. Every1000-call timed pass submits
query IDs0..499 twice in that order. Warmups use0..499 once; the separate first
call uses0. Repeat1 runs c ascending, warn then debug; repeat2 reverses c and
log order. Controls run compact-text then structured-hybrid in repeat1, reversed
in repeat2. No best-repeat pooling or claim of1000 independent query identities.

For cN, write the first min(N,total) requests before reading their responses;
then refill one slot per terminal response until exactly1000 submissions.
Record actual client outstanding high-water and submission/completion order.
Client c32 is not32 running readers and does not itself test the33rd waiter:
with cap4 the diagnostic queue_full case needs4 held calls+32waiters+1excess.
Drain every phase before proceeding. No natural cancellation is injected in
M-R/M-C. Preserve all unexpected errors/outliers; they leave qualification
incomplete rather than becoming successful samples or being retried away.

Warn/debug cells are matched workload/logging perturbation controls. The
restricted text/structured controls above are a review choice, not a full
mode×format×concurrency matrix. They do not replace Slice60/F11 qualification
or release MCP conformance under both formats. Review must accept this explicit
coverage or revise it before source/freeze; no dimension is silently waived.

## Calls, clocks, queue and response accounting

Record attempt/PID/RPC-ID/phase/submission-index/query-index/store/mode/format,
raw request/response, bytes and actual terminal outcome; retain returned
store/epoch/response_id/degradation/budget/hints. response_id is not an invocation
ID. Serialize the request before timing, capture monotonic send-start before
write/flush and flush-end, then complete-line arrival before response parsing.
Client response latency includes transport/admission/open/work/shaping and
delivery; it does not split them. Record parse and persistence costs separately.
Use accepted nearest-rank ns statistics, separate success/error distributions,
and explicit submitted/completed/success throughput denominators. A cancelled
request may receive no response: cancellation-requested is a separate flag;
retain its actual response or bounded unresolved disposition, never wait forever.

[Registry admission](../../../../../src/rust/crates/fathomdb-mcp/src/registry.rs)
logs only admissions after a wait, with store and integer-millisecond waited_ms,
but no RPC-ID. Keep raw stderr through EOF with bounded output/checkpoints.
Report observed waited-admission events and the **whole-process** submitted
denominator, separating first/warmup/timed/control counts. Conditional wait
percentiles are quantized and cover logged admitted waits only; errors/cancels
and open waits are not thereby measured. Warn has unavailable queue data,
not zero wait. Missing logs are not proof of no queueing. Do not assign events
to calls or timed windows using stderr reader receipt time. No unmatched mean/
percentile subtraction or inclusive-duration addition supplies service time.
Per-call queue, exclusive service/open/render and engine-finish times remain
unavailable; no production span/correlation observer is commissioned.

[Bridge](../../../../../src/rust/crates/fathomdb-mcp/src/bridge.rs) cancellation
and run-limit replies abandon work; they do not interrupt it or release its
permit before the blocking closure returns. Distinguish busy/admission deadline,
queue_full, not_ready opening/reopening and timeout/deadline/cancelled. Preserve
protocol, transport and actual70 provider outcomes independently. Debug changes
are descriptive logging overhead, never a correction subtracted from warn.

## Closed subjects and actual lifecycle measurements

Runtime input parameters must name the selected **qualified closed100k G**
subject, qualification/source/native/model/device identities, unchanged corpus/
query files, readiness/current generation, physical vector counts and spans,
and successful normal close. No placeholder path qualifies it. Copy only an
owner-released closed SQLite subject with its valid closed-file packaging into
M-owned storage; never copy a live DB or omit required WAL companions. Do not
force checkpoints, invalidate equivalence markers or alter projection state.
Each natural cell receives a fresh owned copy; no sibling live engine/cache/
venv/target/native artifact is borrowed. No live Memex, substitute corpus,
model/dataset fetch, CPU fallback, K620 or driver/primary-CUDA change.

M-L lazy/eager each uses one fresh process: initialize, list_stores, first search
and one immediate same-query search, then normal close/exit. Eager differs only
by `eager:true`; lazy uses false. Record spawn→handshake, first-call→response,
spawn→first-result, subsequent call and exit clocks. Eager opening overlaps
initialization; none is exclusive model/open duration or cold-disk latency.

M-L idle: first search, observe ready, wait for actual closed via list_stores,
then the same search must return a new epoch; one immediate repeat follows.
Retain default60s timeout and1s sweeper; polling at250ms is outside call timers,
included in whole-cell time and recorded as perturbation. list_stores does not
acquire/touch the engine. A call after a sleep alone is not reopen proof.
If closure cannot be observed within the remaining cell cap, retain
NO_OBSERVED_EVICTION/INCOMPLETE; do not shorten the default or mint extra time.

M-L alternating: two distinct owned closed copies A/B, one process, explicit
store on search A→B→A; after each result list_stores must show the other closed
and target ready. Final A must have its next epoch. Record switch-call walls,
states and physical file identities. Identical copies cannot prove data routing
from hit content alone; the tiny TDD fixture uses distinct A/B literal IDs.
Persistent cached equivalence acceptance can survive process restart/idle/A/B/A;
do not claim fresh verification or remove its marker to manufacture open cost.

## Small implementation and genuine runtime TDD units

No tests execute under this design-only authority. After reviewed source GO,
select exact commands/names and register new bounded tests in the maintained
runner. Each missing behavior gets a compiling scaffold, committed runtime
assertion failure, then separate GREEN with unchanged oracle. Existing passing
product behavior is reuse, not manufactured RED; no new golden/tamper suite.

| Unit | Small falsifying oracle before its GREEN |
| --- | --- |
| M-T1 compact wire; then structured wire if missing | New recorder runs normal CLI on literal alpha/beta real DB; missing decoded l:alpha, exact fetch body and typed unknown-store error fails. Keep formats separate if their changes are independent. |
| M-T2 invocation ledger | Real child concurrent calls, including repeated identical queries, must retain distinct RPC IDs and exact query/result associations, timestamps and dispositions. Missing new records fail explicit count/identity assertions. |
| M-T3 schedule | Tiny33-call real-process passes at each c assert literal cyclic query order, exact count, initial outstanding count and fresh repeat/process paths; no timing-based claim of reader concurrency. |
| M-T4 accounting | Literal samples[1,2,3,100]ns give p50=2,p99=100,mean=26.5ns; hand table4submitted/2success/1error/1unresolved retains all four plus1logged wait, without claiming a per-call wait/service. Real helper events verify actual decoder/count routes. |
| M-T5 lifecycle recorder, one behavior at a time | Tiny real A/B stores require literal distinct IDs, observed closed/ready and incremented epochs; missing lazy/idle/switch records fail independently. Normal exit/released handles are observed, not inferred from sleep. |
| M-T6 outcome recorder, one case at a time | Private-helper stdio runs actual Adapter+real DB with existing Seams; missing queue_full/deadline/cancel/run-limit/open-deadline disposition fails the respective literal assertion. Running-abandoned work must remain held until release. |
| M-T7 registration/remaining allocation | Actual harmless children prove explicit table/order, refusal before copy/start when forecast cannot fit, distinct incomplete attempts and retained not-run rows. Reuse passing checkpoint/prefix/resume controls unchanged. |

M-E/M-T6 require a separate **nondefault test-only helper build** serving the
existing Adapter over stdio. Control existing block_open/block_calls/held_calls/
release_all Seams through one private bounded Unix channel under the owned task
root; no extra MCP tool, production environment/config flag, delay hook or new
observer. Stage acknowledgements come from actual held/state observations, not
sleep-only ordering. Always release/join on failure and bound child lifetime.
Keep default cap4/queue32/wait10s/run30s, including diagnostics; never treat the
private helper's timings as normal CLI data or assume production CLI can arm
Seams. Canonical existing A3/admission tests remain preservation evidence.

## Entry stages, fixed caps and unresolved choices

Design PASS and explicit source GO precede isolated tool implementation/scoped
TDD; source qualification/review precede runtime GO. All M runtime, including
M0, stays held until selected qualified closed G input and exact tooling review.
Candidate-owned production MCP/cache/source proof and a reserved3090 with actual
MCP-PID memory increase plus nonzero GPU utilization qualify the GPU stages;
G's old PID observation or an environment string cannot substitute. M0 remains
provider-free and does not establish this GPU proof. Only then stage M-R/M-C,
M-L and separately identified helper diagnostics. No concurrent compilation or
other owned heavy campaign; AGENT_LONG/manual selection gates natural runs.

There is **no renewed4h or16GiB allocation**. Before any M preparation, identify
the explicit remaining reviewed campaign allowance and retained G/D active
DB+WAL+raw bytes. Forecast the complete table from actual lower-stage wall/byte
costs with conservative2x bounds, including copy/open/first/warmups/calls/logging/
polling/idle/close/diagnostics and failed attempts. Forecasting controls dispatch,
not measured latency or a new pass gate. Missing bounds or unclear remaining
allocation keeps rows HELD. Each whole cell remains<=900s; accumulated timing
cannot exceed the reviewed14400s; retained active bytes remain<=16GiB across
prior G/D and M, not per cell. No deletion/reset, automatic expansion or claim
that lower cells satisfy unstarted obligations. If it cannot fit, retain all
required rows and seek separately reviewed explicit budget disposition.

Retain start floors root32GiB/task250GiB/MemAvailable16GiB; stop only owned
groups at root<20GiB/task<50GiB/MemAvailable<8GiB/owned>=200GiB or swap>=64MiB/s
for3 consecutive30s samples. All mutable caches/inputs/tools/DB/WAL/raw/log/tmp
stay under owned/mnt/data; one native phase/jobs2 and canonical verification
settings. Keep load<=2 entry/noisy>2.5, raw errors/outliers/partial prefixes and
original16-record checkpoint limits; no retry-until-green. Default60s idle,
cap4/queue32/wait10s/run30s never shrink to fit. Polling/output costs remain in
whole-cell/campaign accounting, not subtracted from latency.

Material review choices are the proposed limited mode/format matrix, private
helper/channel fit and complete-table feasibility under the actual remaining
allocation. Material runtime evidence remains the qualified closed G subject
and candidate-owned MCP/cache/GPU proof. Source phases later require scoped
checks, one required final clean source gate and fresh independent review;
this docs-only change uses scoped Markdown/link/view checks, not a full gate.
No implementation, source/runtime GO, release-state edit or program closure is
implied by approval of this proposed design.
