---
title: 0.8.30 Slice 40 CPU study and hints qualification design
status: ACTIVE
slice: 40
target_release: 0.8.30
---

# Slice 40 study design

## Inputs and route qualification

Use kernel-scheduled ordinary shared-host CPU execution. Record load, affinity,
CPU model, OS, environment selections and free space at boundaries. Use current
locked release builds of `fathomdb-sdk --features default-embedder --example
slice30_measure`, with `slice30-observer,slice50-measurement` added only to a separately retained
diagnostic and fidelity executable. Leave the Slice 50 eager selector unset.
This supplies actual per-hit cursor, rank bits, provenance, snippets and route
witnesses outside the ordinary timing profile. Set `FATHOMDB_EMBED_DEVICE=cpu` and
`AGENT_LONG=1` for the adapter, with CPU-only Cargo features and no perf
environment limits, forced sort, graph or reranker overrides. Retain cargo
feature closure, Rust version, source/lock, SQLite build and executable digests.
No cache flush, scheduler or CPU pinning overrides. OS cache is uncontrolled.
Take the model only from the prior G cache, copying the three source-pinned BGE
assets to an owned `XDG_CACHE_HOME` before opening. The current loader expects
BGE revision `5c38ec7c405ec4b44b94cc5a9bb96e735b38267a`. Verify the three
assets against the constants in the loader before dispatch. This study makes
no change to the embedded model, chosen precision, default device, or pins.

Use the released closed `g-frozen-seed100k-v4` 715,223,040-byte main SQLite file
from [prior G](../../../0.8.29/features/slice-30/continuation-gd.md#closed-copy-custody-and-independence).
Confirm its retained seed result, quiescence, closure and zero WAL bytes before
taking owned matching copies. Preserve the archived files. Bind the actual
bytes in the run. Use its original ordered JSONL queries, SHA-256
`0eeb5582fc93bee7e2a494054a55d26bb0940d771083b726f999a9682e74dc16`, corpus
`cc5930e525a2af2954353f92dacf720cc4b79f40dbb7965c44bb5c985b5cb481`, and
license. Query index is zero-based. Confirm format 37, 100,000 active `doc`
records, 100,000 logical vectors, spans and physical vector rows, declared
profile/embedding identity, ready generation and zero pending/failed work.
Compare closed pre-arm fixture identity and options. SDK `hybrid` must show
an effective CPU device, completed embedding, vector and text retrieval where
the query has matches, and a vector-bearing result population. SDK `text`
uses the same initialized store/options with direct text API and no embedding
for that request. This holds dataset and provider constant. Record actual source,
kind, validity and dependency evidence. Check that a second SDK process can
reopen the first closed subject of each route and reproduce the first six
ordered reads and projection authority. Record source close result, reopen
report, probe outputs and first-call scope.

## Fixed matrix

All scheduled searches are sequential closed-loop, one SDK caller. Submit the
next call after the preceding call and recorder work, with no added pacing.
Ordinary shared-host traffic and G-W's interleaved governed writes are the
contention here. Report offered attempts as realized arrivals and the schedule,
without extrapolating to an open-loop or multi-caller service. Each SDK call
uses existing adapter options, with default unfiltered view and limit 10
on both hybrid and direct text.
Use G-R/G-W ordinary artifacts for performance and G-O for request stages.
Each ordinary read arm has Q0 as the first API call after open/readiness, Q0–19
as warmups and Q0–119 as measured searches. Each ordinary write arm uses the
same warmups, Q0–79 as measured searches and G-W b1/s10. The existing scheduler
commits one natural record replacement per cohort, probes point/FTS/readiness,
then performs ten searches. No projection drain is inserted between cohorts.
Record positive writes and read progress. This schedule does not assert a
projection worker overlapped a specific read.

| Passage | Populations and order |
| --- | --- |
| Pre-timing qualification | One separate owned copy for untimed fixture/provider and four dense/text diagnostic query witnesses; fail closed before timing if selected prerequisites are absent. |
| Ordinary baseline G-R | Four nearby two-route blocks, order hybrid/text, text/hybrid, text/hybrid, hybrid/text. Both arms use the exact ordinary current source artifact and fresh equivalent copies. Record the scheduled lifecycle reopen after the first block outside timing. Each route has 4 first calls, 80 warmups and 480 measured calls. |
| Diagnostics G-O | One separate copy per route, Q0–11 with no warmups or first-call cohort, current observer artifact. Record one batch for each request and validate operation joins and loss counters. Run paired direct-text and hybrid; time values are stage attribution only. |
| Natural stress G-W | One hybrid ordinary b1/s10 cell plus one separate observer b1/s10 proof cell with Q0–19 measured and Q0–3 warmups. Reopen the written observer subject and verify current marker cursors. Establish natural committed state before any selected change. |
| Selected treatment, conditional | Eight G-R hybrid baseline/candidate blocks, order AB, BA, BA, AB, BA, AB, AB, BA, preserving calls and fresh copies. A is pre-treatment source. Use a G-O Q0–11 pair for exact fields, and one separate matched observer G-W A/B pair with the same natural proof and reopen for both written subjects. Four ordinary G-W hybrid pairs AB, BA, BA, AB. Confirm direct-text guard scope. If affected, use four adjacent ordinary direct-text G-R pairs AB, BA, BA, AB and one corresponding G-O pair. |

Record all attempts and use adjacent arms with unique owned process/copy
paths. Confirm no leftover actor before proceeding to the other arm. If
otherwise inert file preparation takes time, prepare both copies before the
block. Validate counts, declared input identities and repeated query order.
Keep data, binary and build options identical within a treatment block except
for the candidate source delta. Track peaks without changing the workload.
Set a 20-minute watchdog per cell and a 5-hour wall stop for this study
excluding builds and breaks for review. If an input or fixed setup fails
before call dispatch, fix the setup and record both attempts. A started cell
is final evidence. Retain all partial data and classify missing counts/errors
and stopped cases. Finish independent qualified cells where possible. No
replacements for workload outcomes on the basis of measured latency or host
load. A missing required arm or unmatched pair precludes promotion.

Use existing G JSONL rows and SDK call clocks. Record successful-return
nearest-rank p50/p95/p99 from each arm and pooled by route/schedule, no dropped
success or latency outlier. Include separate first/warmup populations and
the denominators. Count raw attempts by operation and split success, typed
refusal, timeout and other errors from actual result types. Missing completion
is missing evidence and is not a typed timeout. Useful completions per second
use the measured loop wall including recorder work and, on G-W, write and
probe time. Also report end-to-end process elapsed separately. Use
process user+system CPU seconds, peak RSS, available I/O counters and sampled
WAL file bytes, with counter units and sample interval documented. JSONL
recording times and bytes are harness overhead, including checkpoint flush.
Rust SDK return has no product wire encoding. The canonical G rows are
comparison payloads, not SDK response or wire byte counts.

## Attribution, candidate and decision

Map `operation_id` to completed observer request/span events. Attribute API
compile, query embedding dispatch, vector retrieval/SQL/hydration, dependency
and direct eligibility, node/edge FTS, fusion, deferred hydration and final
response as present. `reader_wait` includes response wait. A distinct queue
delay needs reader admission and worker start clocks for the same request.
Do not subtract overlapping or incomplete spans to infer one. Report reader
transaction as its measured scope and leave full reader-service jitter
unobserved where no corresponding span exists. State that configured depth
zero bypasses cross-encoder rerank. Label harness output recording separately
from final API shaping. The diagnostic fixture and projection observation are
separate from the actual search snapshot unless the request records a reader
capture. When the node-only route applies eligibility on matched rows and
skips the global proof, the opt-in observer records one untimed
`all_nodes_directly_eligible` event with status `not_run`, reason
`match_local_eligibility`, and no executed SQL or eligibility result.
A request that needs the legacy proof reports its actual SQL span.

For `has_source_dependencies` and `all_nodes_directly_eligible`, retain
current query plan and SQLite `FULLSCAN_STEP` and `VM_STEP` from the exact
SQL and same bundled SQLite build on the owned closed subject and small
all-eligible, early-ineligible and mixed real stores. Capture counts from
each statement, not from a parsed plan estimate. Capture frequency and
elapsed cost from actual G-O request spans. Gather actual prefilter/defer
route and text/vector evidence, distinguish why the guards skip, and count
affected queries. Use existing trace/test selectors plus a small source-bound
statement-count probe if required. Plan text is diagnostic, unordered with
respect to equal ranks, and not a permanent assertion format.

Consider exactly one treatment: remove a demonstrated whole-store
eligibility proof from hybrid node FTS by applying the existing ownerless
existence and validity SQL on the existing canonical match join before
ranking on the node-only winner-hydration branch. Select this branch only
under all its existing guards other than the replaced scan. Eager deferred
identity, mixed-edge and vector-stage-only paths keep the old global proof
and branch selection. Consider implementation only when all 12 scheduled
hybrid G-O attempts have a joined valid observer result, the winner route
and guard each run on at least 6 of 12, the guard median is at least 5 ms
and 10% of the same requests' median caller time, and the all-eligible
100k baseline query has at least 90,000 full-scan steps or 500,000 VM steps.
Retain skipped and missing calls, with all 12 scheduled as the denominator.
Missing required diagnostic results preclude selection. Keep per-snapshot
dependency check, remaining guards, rank arithmetic and identity. Retain
reference typed errors for malformed losing matches and existing behavior
when valid predicates change candidate membership. If that cannot be shown,
record no justified treatment. Compare original/reference and candidate
on exact order, IDs, cursors, score bits, bodies/snippets, source/provenance and
projection authority, with fixed view/time for boundary checks. Canonical
ownerless rows still use their FTS rowid presence/ownership rules. The
default-view predicate must keep future start and half-open expiry semantics,
and run before candidates are selected. Retain reader snapshot and visibility
publication. Do not touch graph, provider selection, persisted state or caches.
The work regression must fail on the actual path in RED. Reuse
`slice71_search_statement_trace`, `slice35_eligibility_pretruncation` and
existing .29 lightweight controls. Where an optimized route can only differ
on work count, assert work count and add real-store direct expected results;
do not regenerate an oracle or give moving runs cross-snapshot equality.

For every diagnostic G-W arm, join each measured read and marker query to
its reader snapshot capture by operation ID. Compare returned projection
cursor with observed snapshot projection cursor and observe monotone read
visibility. Check hit cursors against the pre-study canonical maximum plus
that read's already committed governed receipts. Projection may lag the
canonical maximum. Use initial and committed point/FTS cursors for the
changed IDs and establish that any returned changed ID has its current
cursor and the old cursor is excluded. After ordinary close, a fresh SDK
process opens each same written subject and repeats the marker-token reads
and a natural hybrid call. Confirm persisted current cursor/body, absence
of the superseded cursors, and public projection authority/progress via
reopen. Give a limited SDK probe time for pending work to reach readiness
if necessary and retain the initial reopen result. A missing required
snapshot join, marker or reopen result leaves adoption ineligible. Judge
each arm by its own authority during moving-state proof.

Primary statistic is the median paired relative warm p50 improvement
`(A - B) / A` across eight G-R blocks. Use the min and max paired values as
the conservative exact distribution-free 99.22% sign interval for an
independent-block median, noting any same-host serial dependence. Require
median reduction at least 10% and at least 5 ms, and positive lower observed
bound. Adoption also requires no new errors/refusals/timeouts, exact
stationary parity and positive natural-write progress. For each affected
ordinary and G-W schedule require pooled p95 and p99 no more than 5% higher,
median paired G-W p50 no more than 5% higher, median paired useful throughput
at least 97% of baseline, median paired CPU seconds per success no more than
105%, and median paired peak RSS no more than 110%. Require no structural
increase in touched persistent state or measured WAL/response representation.
Report actual WAL and I/O samples and any measurement limits. First-call
descriptive guard: flag any median paired increase over 15% and keep
promotion open if unexplained. Anything failing benefit or guardrails is
neutral, negative or inconclusive and the candidate source stays unpromoted.
Final consumed Slice 60 pin changes require affected proof before promotion.

Method references: [SQLite query plans](https://www.sqlite.org/eqp.html),
[statement counter semantics](https://sqlite.org/c3ref/c_stmtstatus_counter.html),
[disabled optimization tests](https://www.sqlite.org/testing.html#disabled_optimization_tests)
and [NIST randomized blocks](https://www.itl.nist.gov/div898/handbook/pri/section3/pri332.htm).

## Inherited hints diagnosis

Use the existing devloop profile and fixture in `hints_overhead.rs`: 10 and
100 hits, alternating 204/4,098 body bytes, one render per call, 31 trials of
15 off/on repetitions. Execute the unchanged existing test exactly once on
current inputs, retaining artifact/build, exit, load and listed p50/p99.
One bounded same-process diagnostic uses the existing MCP test seams to
measure paired `plan_search_hints` on/off, preplanned
`shape_planned_search_page` on/off, and combined `shape_search_page`.
Use 31 trials of 15 paired calls per stage and limit, matched pages and
balanced alternation of off/on order over the full observations. Observe
elapsed per call, aggregate process CPU and independent output bytes/kept
count/iterations. Report stage medians as separate estimates, without
subtracting separately measured medians to invent attribution. Search-plan
construction and later emission plus rendering are distinct boundaries.
Keep the stage evidence local to this synthetic MCP fixture. This test
seam includes an extra harness output string encoding. Preserve its scope
rather than attributing all of the measured delta to production rendering.

Prior 289.9, 366.8, 345.3 and 374.6 µs values remain failing under the 200 µs
limit. A current incidental pass does not establish cause or erase failures.
If the bounded split does not establish the cost source and demonstrate a
qualified resolution of the inherited failure, carry the qualification open
for its owner. No second product optimization is commissioned here.
