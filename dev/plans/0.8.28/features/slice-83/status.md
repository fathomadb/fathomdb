---
title: Slice 83 status
status: COMPLETE
date: 2026-11-03
---

# Slice 83 status

Slice 83 engineering and measured qualification are complete. Final production
source is `af84f12f8`; the gate campaign completed through scoped corrections and
resumption, not one uninterrupted green invocation. Work used the existing
`dev/0.8.28` checkout and fast-forwards the existing `release/0.8.28` checkout.
No new branch/worktree, tag or publication was created. The end-of-phase
physical-guarantee/API decision remains open. Existing physical-success semantics
are retained; this is not an availability-improvement or deadline qualification.

## Scope and acceptance

[Design](design.md) reconciles the Slice 82 baseline with the draft, allocated
CH1-2A–2F work and affected mutation/read surfaces. The
[successor ADR](../../../../adr/ADR-0.8.28-resumable-physical-cleanup.md) records
reviewed scoped admission and autonomous recovery without changing physical
success. Requirements, acceptance and interfaces were adjusted to a finite plain
legacy-write envelope; provenance/administrative/publication mutations remain
fenced. Atomic canonical deletion is retained.

| Acceptance | Concrete qualification |
| --- | --- |
| CH1-2A | A committed sanitation rendezvous admits real unrelated Engine reads and survivor writes with registered dependency debt. Affected source/identity/endpoint mutations refuse. This proves between-unit progress, not improved total loaded throughput. |
| CH1-2B | Three independently constructed survivor-only distributions vary excluded term length, duplicate density and graph fanout. Body/property/frozen/hybrid scores match bitwise, with positive vector witnesses, nonempty evidence, graph caps, current/frozen search expansion, canonical/list results and stale-frozen refusal. Unsupported historical graph expansion refuses on both stores. Existing physical/history oracles remain unchanged. |
| CH1-2C | Same-source/dependent identities and old edge endpoints stay denied; queued projection publication drains before deletion, rebuilds refuse while owed, and mapped non-head vector residue refuses proof. Child exits at delete, sanitation and tombstone boundaries reopen and finish without a target argument or follow-up Engine call. |
| CH1-2D | Resource/singleton saturation refuses before deletion. Interleaved survivor supersession preserves sanitation progress. Priority contention and lost-wake regressions have deterministic RED/GREEN coverage. Close waits for an admitted owned snapshot, joins maintenance/readers and releases ownership. |
| CH1-2E | Owned/external pinned readers and arriving pooled readers exercise finite drain/gating and truthful incomplete retry. Missing/wrong/multiple original sinks, malformed obligations and late events retain prior regression coverage. Root blockers persist before synchronous return. Offline completion and clean no-op checks pass. |
| CH1-2F | Frozen Phase-82/source-matched baseline and candidate campaigns retain all scheduled/issued/completed/refused/missed/unfinished dispositions and actual native/drain/wait/hold/sanitation/checkpoint/lifecycle counters. Measured report and limits below. |

The route matrix's historical flags do not by themselves establish a new temporal
isolation contract; the unchanged survivor/history tests supply retained-history
coverage. The corpus's edge load reads endpoint-node text. Foreground updates are
plain document supersession, not property-FTS supersession. No broader workload
claim follows.

## Verification and review

Astra approved the production design before code and reviewed the complete source
delta plus narrow corrections. Independent Astra verification read the test
bodies and reran decisive suites. Representative RED/GREEN pairs include
`84fb42d85` → `dfd493f09` (unit mutex release), `66aad5906` → `c4769297e`
(scoped writes), `f0c11af47` → `ca44d4e50` (idle service), `18f7c9a1c` →
`24001813c` (reader admission), `e6cfaa54f` → `f89a4396c` (lost wake),
`34750f1ba` → `b6db80523` (priority under contention), and `96ea3240b` →
`72afa85a6` (durable synchronous blocker). The independent RED audit inspected
commit bodies and recorded failures; it did not claim historical reruns.

Scoped evidence already passed:

- Slice 83 availability: 16 tests, with the ignored child entrypoint exercised
  by its parent at three crash phases; route/publication suite: 2 tests.
- Physical-erasure unit/property suite: 6 tests, including mapped absence proof,
  rollback and randomized retry invariants; maintenance priority regressions pass.
- Existing erasure completeness 11, drain ordering 5, projection registry 4 and
  dependency closure 28 tests; offline CLI recovery 8 tests.
- Existing Slice 82 batch, physical/recovery and telemetry suites; the existing
  1,050,029-byte body oracle is preserved by the reviewed 2-MiB admission limit.
- Scoped engine Clippy, module ownership and Markdown/release-state validation.

The existing dependency tests caught an error-stage regression; `a266ce84d`
restored `dependency_closure` refusal precedence without changing eligibility.
The final measurement audit passed. The workspace gate ran once and resumed only
failed or unrun portions:

- Initial `agent-verify` at `969261911`: full lint passed; default-feature
  typecheck found the offline-only `sanitize` helper compiled without consumers.
  `b721e8647` scopes it to operator/tests; the complete typecheck phase passed.
- Strict security passed with zero violations, blockers or downgrades, including
  live network-namespace and ptrace checks.
- The full collecting test run exercised all 205 suite labels: 201 initially
  passed, four failed (Rust, Python configuration, exact root inventory and board
  currency). There were no excluded or skipped suite labels; individual model,
  platform and long-running checks retain their normal skips.
- The original Rust collection completed all other targets after two confirmed
  deadlocked targets were stopped. `8150e0f8a` adds failing diagnostic/idle-profile
  regressions; Astra-reviewed `af84f12f8` repairs the gate/metadata deadlock, counts
  actual SQLite work rather than materialized handoff, preserves real bounded
  checkpoint observations after drain refusal, suppresses only idle debt-probe
  subscriber callbacks, and restores the pre-primary-lock test hook. The unchanged
  affected graph/configuration suites pass 34/8 tests. The entire engine library
  passes 142 tests with the original `operator,test-hooks` feature profile.
- Independent verification reran nine affected diagnostics, telemetry and reader
  cases. Scoped all-target engine Clippy passed after these source corrections.
- Python originally passed 1,700 tests with 36 individual skips and six stale
  fixed-thread-count failures. The reviewed topology reconciliation keeps exact
  counts, adds named cleanup-worker attribution and exact close restoration; the
  entire configuration suite passes 31 tests. The native binding receipt passes.
- TypeScript, hermetic native-package execution and all remaining collected
  harness suites passed. Exact root inventory/module classification and stale
  board prose were reconciled with the approved two fields/one worker and current
  ladder, then checked with their scoped validators.

Logs are retained at `/tmp/fathomdb-slice83/{verify,typecheck-resume,security-resume,tests-resume}.log`,
with scoped repair logs alongside them. The source correction and final metadata
checks are separate receipts, not a claim that the original full command passed.
The independently authored Slice 84 draft commit `5ba727473` appeared during the
run, changed only three new planning files, and was preserved after confirming
its writer was finished. It adds no execution or guarantee ruling here.

## Operating envelope and exclusions

One admitted root job; 1-GiB logical database, 100,000 canonical rows, 1,000,000
vector mappings, 64-MiB provenance/receipt metadata, 256-MiB total body bytes,
2-MiB individual bodies and 16,384 target cursors. These bound work/admission,
not time. Supported unrelated writes are plain legacy nodes/edges with disjoint
source/stable identities and old/new endpoints, no protected prior provenance and no application-trigger fallback execution.

A finalization attempt gates new owned snapshots and waits at most 100 ms for
owned readers; existing reads run to completion. Checkpoint attempts are finite.
After the identity-free tombstone, mutations remain globally fenced until actual
completion. External readers, unavailable original sinks, process downtime and
OS scheduling can prolong completion without a finite upper bound. No cleanup
API, reader cancellation, wall-clock lease or numerical SLA is selected.

## Measured service costs

The native progress witnesses pass, but **net loaded availability/throughput
regresses** on these corpora. There is no green availability-improvement verdict.
The full paired campaign used Phase-82 baseline `abc4f7ad7` and prepared-query
candidate `5569df4f3`, release builds, performance runtime, an offline fixed
provider, the frozen original 10k/five-revision/10%-target masters, 15-second
windows, eight read clients and one writer cycling eight unrelated identities.
There were three pairs per family plus matched no-erasure controls. Baseline
stage counters are unavailable; do not compare its public call time to a
candidate-only native stage or infer baseline writer ownership.

| Family | Baseline public call median (s) | Candidate public call median (s) | Baseline completed writes median / 1500 | Candidate completed writes median / 1500 |
| --- | ---: | ---: | ---: | ---: |
| Node | 1.354 | 8.070 | 1365 | 729 |
| Edge | 1.394 | 3.393 | 1361 | 1182 |
| Property | 3.108 | 16.176 | 598 | 26 |

Candidate actual stages below are median cumulative seconds per erasure, except
the explicitly marked maximum unit. Counters overlap and must not be added.
Physical writer counters observe all physical-worker ownership during the window,
including any contended idle probes. Public-call/lifecycle counters are distinct.

| Stage | Node | Edge | Property |
| --- | ---: | ---: | ---: |
| Projection drain | 0.0555 | 0.1334 | 0.1571 |
| Atomic native writer hold | 0.7404 | 0.9250 | 1.3528 |
| Physical writer wait | 6.6383 | 1.9524 | 13.3552 |
| Physical writer hold | 1.1534 | 1.1879 | 2.4078 |
| Sanitation commands | 0.2120 | 0.1129 | 0.5025 |
| Maximum committed sanitation-unit hold | 0.0434 | 0.0216 | 0.1236 |
| Owned-reader drain | 0.0365 | 0.0058 | 0.000001 |
| Checkpoint | 0.0159 | 0.0101 | 0.0314 |
| Completed lifecycle | 8.0682 | 3.3920 | 16.1762 |

Maximum observed reader-gate windows were 0.3401/0.2041/0.2117 seconds for
node/edge/property. Native writer-wait samples were sub-microsecond; sanitation
writer waits, finalization holds and service-slot attempts remain separately
available in the machine-readable results. No duration is a guarantee.

Both prepared node/edge no-erasure controls served every offered operation.
Both property controls completed only 754/1500 writes, so 100 Hz exceeds the
observed no-erasure service envelope there. A separately approved 20-Hz property
supplement retained 100-Hz reads: both controls completed 300/300 writes; erasure
completed 247 baseline writes (50 generator misses, three Engine refusals) versus
82 candidate writes (218 generator misses, no Engine refusals). Candidate reads
completed 1466/1500. Fewer than 1,000 write samples cannot qualify p99 even for a
complete low-rate control. Refused, partial and missed streams never qualify p99.

The first candidate's regression is retained separately. Reusing its prepared
mapped-vector probe reduced deterministic preparation work but demonstrated no
latency benefit in the paired follow-up. Writer-wait measurements locate the
observed delay between sanitation units; attribution to repeated full envelope
and zero-proof certification is an inference from the executed path, not a
separately instrumented per-check timing claim. Retain atomic deletion. A future
performance change should measure and redesign those repeated certifications
under explicit scope/rollback/reopen authority; this slice does not remove checks
to improve a benchmark.

Three initial `/proc` exit-monitor failures retain unqualified process accounting
despite complete raw product results. Confirmed-exit monitoring was fixed with a
bounded wait and live-failure regression. Separately identified reruns complete
the original grid; no result was overwritten and the original shared 45-minute
budget was retained. Independent audit reconciled all original/prepared/low-rate
172,200 offers across all 59 attempts exactly: 144,158 issued, 28,042 generator
misses; issued split into 143,970 completions and 188 Engine refusals, with zero failed/unfinished/unknown
offers. These are aggregate accounting totals across distinct protocols, not a
combined service benchmark.

The final application-trigger guard is a separate source correction. All frozen
masters' 61 triggers match tracked definitions, and the driver creates no TEMP
triggers, so the preceding workload remains in the certified class. A final
paired/control smoke records the corrected binary separately. The final `e96fd78f4` node smoke completed all four cells: public erasure
1.384 seconds / 1361 completed writes on the baseline versus 7.237 seconds /
807 writes on the candidate; both no-erasure controls completed 1500 writes.
This confirms the corrected supported path and retains the regression; it is
not a replacement for the three-pair campaign.

[Machine-readable measurements](../../../../experiments/erasure-availability/slice83-measurements.json)
retain all 59 attempts, source/binary/input identities, stage counters and offer
accounting. Raw events remain under `/tmp/fathomdb-slice83/`. All timing finished
in 1873.955 seconds from the original start, within the 2700-second budget;
known child durations sum to 1093.582 seconds and are a distinct quantity.
Observed peaks were 1,073,094,656 RSS bytes and 1,041,460,116 scratch bytes.
Independent verification supports the engineering witnesses and honest cost
characterization, not an availability/latency improvement claim. Physical WAL
qualification was on Linux. The later `af84f12f8` diagnostic/idle-telemetry and
reader-handoff corrections were verified by scoped correctness tests and were
not retimed. Measurements retain their preceding exact source identities; no
performance improvement is inferred from those corrections.
