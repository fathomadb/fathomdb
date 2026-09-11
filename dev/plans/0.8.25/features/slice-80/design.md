---
title: Slice 80 — absolute read-performance acceptance design
status: DRAFT
---

# Slice 80 — absolute read-performance acceptance design

## Authority and purpose

Owner ruling **seq-277** approves the scope implemented by [the plan](plan.md).
The old ratio penalizes sequential improvements even when both execution
times improve. Replace that acceptance assertion with independent absolute
budgets; preserve historical failures and separately prove reader independence.
This design is pending independent review, not a claim of implemented behavior.

## Local requirements

| Requirement | Contract and proof |
| --- | --- |
| R80-1 | Retire AC-020 and register its successor consistently in acceptance, ADR, test-plan, selectors and live release manifests. |
| R80-2 | Sequential total <=500 ms; warning >=200 ms. Concurrent total <=100 ms; warning >=80 ms. Boundary unit tests use full precision. |
| R80-3 | Both arms execute the original 1,600-search workload with unchanged result checks and eight concurrent readers. Positive-count evidence is mandatory. |
| R80-4 | Ratio remains descriptive; warnings are non-blocking and prominently visible in structured and human summaries. |
| R80-5 | REQ-018 independent reader progress remains tested against a real database, independently of performance timings. |
| R80-6 | Seven valid fresh-process performance-mode observations are required; missing, skipped, malformed and environment-invalid evidence cannot pass. |
| R80-7 | Valid AC-072 evidence and applicable protected 71B write evidence remain required; other read/freshness contracts are unchanged. |
| R80-8 | Exact candidate/input identities, independent reviews and a Slice 85 applicability map support closure; no broad round here. |

## Fixture and timing invariants

The existing Rust gate in
`src/rust/crates/fathomdb-engine/tests/perf_gates.rs` seeds four records using
an eight-dimensional RoutedEmbedder and drains before timing. The sequential
arm runs eight mixes; each mix has 50 rounds of four queries. The concurrent
arm runs the same mix on each of eight caller threads: 1,600 searches total,
not 1,600 per reader. Preserve query bodies and nonempty-result assertions.

Preserve current ordering: sequential first, concurrent second. Do not add
warmup, move setup into/out of timing or relabel this as independently warmed
arms. The concurrent timer starts before releasing the barrier and includes
completion/join; thread creation remains outside it. Record the asymmetry and
cache state in the protocol rather than changing them to improve results.

Use the actual shipping performance-mode configuration with statement reuse.
No profiler, diagnostic counters, environment override or allocator preload
belongs in timed artifacts. Startup witnesses may run separately. This tiny
fixture is an overhead/aggregate-time gate, not representative production QPS
or per-query p50/p99. Absolute budgets apply on the registered performance
executor, not indiscriminately to all cross-platform packaging jobs.

## Oracle and result model

Implement a small test-harness-local pure evaluator taking the two measured
durations. Keep it outside the product/public API and avoid a new framework.

```text
sequential_warning = sequential >= 200 ms
concurrent_warning = concurrent >= 80 ms
sequential_failure = sequential > 500 ms
concurrent_failure = concurrent > 100 ms
numeric_pass = not sequential_failure and not concurrent_failure
```

Compare Duration values or exact integer nanoseconds, never truncated integer
milliseconds. Exactly 500/100 ms passes numerically with a warning. A run
that exceeds a hard limit is FAIL even if it also raises warnings. Retain
each arm's flags, so one failing arm does not hide the other's status.

Records include successor identifier, source/build/binary identity, process
label, mode, fixture identity, operation/thread counts, exact durations,
thresholds, per-arm warnings/failures, exit status and environment qualification.
Keep raw logs. Ratio may be calculated for display only; zero/missing duration
must not cause division errors or become a vacuous acceptance pass.

Human summary distinguishes PASS, PASS WITH WARNING, FAIL, ENVIRONMENT_INVALID
and INCOMPLETE. Environmental invalidity and a numerical failure may coexist:
retain both facts. Warn in the final campaign/release summary and structured
output, not solely in test stdout that a successful runner may suppress.
No new product slow-statement events: these warnings concern a benchmark batch,
not AC-007's per-statement instrumentation.

## REQ-018 architecture coverage

First inspect `tests/reader_pool.rs` and existing test-hook witnesses. Reuse
adequate proof rather than adding duplicate tests. If a gap remains, arrange
two real reader connections with a narrowly scoped synchronization hook: hold
one reader after acquisition and demonstrate that a second reader completes
before releasing the first. Assert distinct connection/worker identities.
Use a bounded timeout only to detect deadlock, not as a performance ratio.
Always release/join during failure cleanup; avoid a hanging test process.
Do not mock SQLite or alter shipping dispatch to make the witness pass.

## Focused RED/GREEN cases

- For each warning and hard limit: just below, exactly equal and just above
  (including a one-nanosecond violation). Test all-clear, one/both warnings,
  one/both failures and mixed warning/failure outcomes.
- A poor ratio with both absolute limits satisfied must pass numerically;
  a good ratio with either absolute limit violated must fail.
- Missing arm, zero search count, wrong thread/total count, skipped fixture,
  nonzero test exit, malformed record, missing identity and invalid environment
  cannot yield applicable acceptance. Inspect existing campaign tooling before
  adding the smallest necessary parser/validation support.
- Prove warning visibility in the human campaign summary and structured record;
  warnings do not change successful exit status, hard failures do.
- Run the mapped or added reader-independence witness separately from timing.
  Stage/commit RED before implementation; no editing unrelated golden oracles.

## Bounded evidence protocol

Before READY, enumerate exact executable commands and selectors from code,
expected positive counts, source/toolchain/features, timeout and environment
sampling in a retained protocol/manifest. Do not invent commands or suite counts.
Preserve applicable historical controls, but do not inherit a ratio acceptance
rule. Explicitly verify the existing AGENT_LONG guard actually executes tests.

Run seven fresh processes, each measuring both arms once in performance mode.
Both hard limits must pass in every valid run. Summaries retain individual
measurements, warnings, medians, spread and all invalid/failed attempts. Run
without builds/competing performance work. No repeat-until-pass; the plan permits
one documented environment correction and replacement series for an affected
gate, after which unresolved environment trouble blocks closure.

Run the exact Slice 71 AC-072 candidate campaign: three 10k/384d/1,000-query
repetitions with unchanged 80/300 ms p50/p99 limits and environment policy.
Nonzero machine-wide swap activity invalidates evidence, without proving
FathomDB caused it. Obtain a quiet environment; do not change OS policy or
terminate unrelated processes without approval. Record contemporaneous controls
for the successor too; swap-invalid evidence is not applicable there either.

Six Slice 79 write receipts can carry forward when relevant product/build
inputs are unchanged. A test-oracle/documentation change alone does not demand
historical write reruns. Explain applicability precisely; product/runtime
changes require the retained two-workload candidate-only guards as appropriate.

## Contract migration and historical integrity

Allocate the successor AC identifier only after checking acceptance and ADR
registries; no draft identifier is assigned here. The owner has authorized
retirement, but canonical registration and executable migration remain work.
Keep AC-020's original assertion/results under a retired/superseded label.
Update P-PARALLEL-TOL's active mapping, REQ-018 traceability, test-plan commands,
ADR index and live release validators so no hidden old ratio blocks closure.
Do not alter historical Slice 75/76/77/79 receipt bytes or claim they passed
the original gate. An explicit successor mapping is preferable to rewriting
historical manifest semantics. Review the existing AGENT_LONG/CI routing for
positive execution rather than accepting a green skipped test.

AC-072/076 measure per-query larger-corpus latency; AC-073 measures real-corpus
stress tails; AC-015/016 measure freshness/per-call latency. All remain binding
under their own protocols. No additional 20% regression criterion, hybrid bound
or diagnostics-mode acceptance gate is introduced.

## Slice 85 handoff and non-goals

Independent design, code and evidence review must pass. Handoff names every
affected source/test/fixture/artifact and each reused versus invalidated receipt.
Slice 85 uses successor acceptance, not the retired ratio, and preserves the
rest of the inherited release inventory. Its final broad round and packaging
remain separate; no need to allocate or move other slices.

No product optimization, SQLite fork/isolation, lookaside/page-cache treatment,
new runtime mode, public API, schema change, publishing or threshold tuning.
Future optimization opportunities remain parked in ROADMAP.md. Contract
replacement does not claim residual concurrency contention was eliminated.
