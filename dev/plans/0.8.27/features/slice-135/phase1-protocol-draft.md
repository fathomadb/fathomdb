---
title: FathomDB 0.8.27 Slice 135 — Phase 1 measurement protocol draft
status: DRAFT_NOT_FROZEN
target_release: 0.8.27
---

# Phase 1 measurement protocol draft

The [approved Slice 135 plan](plan.md) controls this draft. It covers Pareto
paths, system latency, system robustness, and logic/exception handling. Use
existing benchmark datasets as workload inputs where qualified. Dedicated
gold-based quality scoring begins only after the four-result checkpoint. This
full protocol is **not frozen**: the remaining workload cells, mapping the
completed Slice 132 SDK surface to benchmark operations, corpus/model hashes
for those cells and independent review remain.

The bounded engine-only first-results subset is frozen separately in
[first-results-protocol.json](first-results-protocol.json) after the
[mixed-sequence baseline noise pilot](results/2026-10-07-mixed-noise-pilot/).
Its five alternating pairs, three cells and supported statistics govern the
first comparison. This broader Phase 1 protocol remains a draft until the
installed-SDK and remaining workload cells are qualified.

An [installed 0.8.26 Python S01 functional probe](results/2026-10-07-python-s01-baseline-probe/README.md)
established a real-database 32/256-row text, vector-bearing and hybrid
workload with basic validity assertions. The later
[baseline-only noise pilot](results/2026-10-07-python-s01-noise-pilot/README.md)
retains five valid separate blocks per size, raw attempts, a failed launcher
attempt and independent recomputation. The 100-sample pilot does not qualify
p99 or a candidate speed comparison. Its 32-row text p95 varied by 28.75%
across blocks; the S01 paired rule must use longer blocks and alternating
version order. The resulting
[frozen installed-Python S01 subset](s01-python-comparison-protocol.json) pins
1,000 warm samples per cell, five alternating version pairs per size and the
descriptive paired-delta rule before candidate timing. Neither the pilot nor
the S01 subset freezes the full Phase 1 protocol.

The [paired installed-Python S01 result](results/2026-10-07-python-s01-paired/README.md)
now has five alternating pairs at each declared size, 1,000 warm samples per
query shape per block, raw materialized-result checks and independent
recomputation. It remains a Python-only diagnostic; the Rust/TypeScript
installed boundaries, S02 and E01–E12 still need qualification under the
broader protocol.
The later [integrated-candidate Python S01 subset](s01-python-integrated-comparison-protocol.json)
refroze the unchanged workload against exact candidate source and wheel bytes
before timing. Its [paired diagnostic](results/2026-10-07-python-s01-paired-integrated/README.md)
passed 20 blocks and independently checked 60,120 materialized calls. The
32-row vector-bearing p50 was lower in all five candidate pairs, while seven
blocks had host-only swap warnings and the 256-row warning-free sensitivity
has only one pair. This result does not establish whole-product equivalence.
The [vector-repaired Python S01 subset](s01-python-vector-repaired-comparison-protocol.json)
refroze the same schedule and workload against source `3f29d649d` and its
installed wheel. Its [paired refresh](results/2026-10-07-python-s01-vector-repaired-paired/README.md)
passed all 20 blocks and 60,120 independently checked calls. At 32 rows,
vector-bearing p50 decreased in all five pairs; 256-row signs vary. Four
blocks have host-only paging warnings, and engine and SDK call boundaries
must be interpreted separately.

The [baseline-only installed-TypeScript S01 noise pilot](results/2026-10-07-ts-s01-noise-pilot/README.md)
now has five 100-warm-call blocks per corpus size, retained npm archives,
materialized-result assertions and independent recomputation of 3,060 call
attempts. Its text p95 range was 34.059% of the five-block median at 32
rows and 26.312% at 256 rows. Four blocks recorded host swap-counter drift
with zero measured-child swap events. Ten-sample baseline/candidate
feasibility probes showed matching IDs and branches, but are unqualified for
performance. The resulting
[frozen installed-TypeScript S01 subset](s01-ts-comparison-protocol.json)
pins 1,000 warm calls per cell and five alternating pairs per size before
candidate paired timing. The full Phase 1 protocol remains open.

The [paired installed-TypeScript S01 result](results/2026-10-07-ts-s01-paired/README.md)
now has 20 valid alternating blocks and 60,120 independently checked
materialized call attempts. It is a narrow installed-SDK diagnostic, with
host paging warnings in 12 blocks and only one warning-free pair per size.
Its descriptive deltas do not close the whole-system latency or full Phase 1
checkpoint.
The [integrated-candidate TypeScript package qualification](results/2026-10-07-ts-integrated-candidate/README.md)
then rebuilt the native addon and installed npm archives from exact source
`cdf253cd`. Its audited selected-operation exercise had 39 executed, zero
failed, two supported gaps and three unavailable. The
[integrated TypeScript S01 subset](s01-ts-integrated-comparison-protocol.json)
froze before its [paired diagnostic](results/2026-10-07-ts-s01-paired-integrated/README.md).
All 20 blocks and 60,120 calls passed independent audit. At 256 rows, the
vector-bearing p50 rose in all five pairs (median +2.89%); two blocks had
host-only warnings. This is a workload-specific lead, not a release verdict.
The [vector-repaired TypeScript S01 subset](s01-ts-vector-repaired-comparison-protocol.json)
bound source `3f29d649d`, rebuilt npm archives and campaign code before its
[paired refresh](results/2026-10-07-ts-s01-vector-repaired-paired/README.md).
All 20 blocks and 60,120 materialized calls passed independent audit. Median
pair p50 rose 2.78% for 32-row vector-bearing queries and 0.57% at 256 rows;
seven blocks had host-only paging warnings. This is a boundary-specific
diagnostic, not an equivalence verdict.

The later `b65283317` release integration adds the off-ladder
`0.8.26+tegra` install-route fix after these paired SDK artifacts were built.
The stored comparisons remain exact-source diagnostics. Python and TypeScript
S01 and Python S02 were rerun on that integrated candidate; TypeScript
S02 has an integrated-source paired diagnostic. A later vector-row repair
supersedes those candidate artifacts; repaired-source Python S01/S02 and
TypeScript S01 refreshes are linked above; TypeScript S02 has the paired
refresh below, while contention remains before
the Phase 1 checkpoint. The corrected embedder-close
landing is also an intended
lifecycle change; measure repeated open/close and post-close memory with the
S02-L cell below.

An [installed Python S02 feasibility pair](results/2026-10-07-python-s02-feasibility/README.md)
now passes the same fresh-database write/project/retrieve/graph-evidence/
erase/reopen sequence through both wheels, including direct canonical-row
counts after erasure and reopen. Its one attempt per version and whole timer
that includes validation do not qualify the sampled S02 latency cell. The
parallel [installed TypeScript S02 feasibility pair](results/2026-10-07-ts-s02-feasibility/README.md)
exercises the same real-database shape and independent canonical erasure
counts on the repaired candidate. Its one pair and validation-inclusive
whole-sequence timer likewise do not qualify S02 latency or contention. A
[controlled installed-TypeScript S02 baseline pilot](results/2026-10-07-ts-s02-baseline-controlled/README.md)
later separated direct SQLite verification from the whole product timer.
Its five independently audited blocks had 15 measured sequences, no host
warnings and a 114.934 ms (2.324%) block-median spread. The
[frozen TypeScript S02 subset](s02-ts-comparison-protocol.json) then fixed
five alternating pairs of 20 measured sequences per block before candidate
timing. Its [paired diagnostic](results/2026-10-07-ts-s02-paired-integrated/README.md)
passed 100 measured sequences per version and an independent raw, state and
order audit. Pooled p50 was +2.870% and pooled p95 -0.256% on the candidate;
all five paired block-p50 deltas were positive (+2.601% to +3.667%). Every
pair had host-only paging warnings and no child swap, so no warning-free
sensitivity estimate is available. This is a TypeScript S02 latency lead,
not an equivalence or release verdict; contention remains open. The later
Rust S02 candidate-only result is linked below.
The subsequent [vector row-error repair](results/2026-10-07-vector-row-repair/README.md)
changes the candidate engine source. The TypeScript S02 and other prior
candidate latency observations remain exact-source diagnostics; affected
cells need a rebuilt-artifact paired refresh before the final checkpoint.
The [vector-repaired TypeScript S02 subset](s02-ts-vector-repaired-comparison-protocol.json)
then bound source `3f29d649d`, rebuilt npm archives, the unchanged schedule
and the campaign runner before candidate timing. Its
[paired refresh](results/2026-10-07-ts-s02-vector-repaired-paired/README.md)
passed all ten blocks and 200 independently checked whole sequences.
Pooled p50/p95 changes were +3.308%/+1.370%; all five pair-p50 deltas were
positive. Every pair had a host-only paging warning, so there is no
warning-free sensitivity estimate. An independent order audit verified all
nine idle gaps. The reopened-stage median lead is 147.193 ms, a diagnostic
attribution target rather than a proven cause or release verdict.
A [baseline-only installed Python S02 timing pilot](results/2026-10-07-python-s02-baseline-noise-pilot/README.md)
subsequently separated the product timer from direct SQLite verification and
retained five independently audited blocks, with three measured sequences
each. Its block-median spread was 64.264 ms (1.20%), and reopened default-
model startup dominated the sequence. Per-block host snapshots and adequate
tail sampling were absent, so a fully qualified S02 noise pilot, frozen
comparison boundary and alternating blocks remain ahead.
The later [controlled Python S02 baseline pilot](results/2026-10-07-python-s02-controlled-noise-pilot/README.md)
retained five valid blocks with start/end host snapshots, per-process
resources, independent state recomputation and a governor-drift negative
control. Its block-median spread was 38.457 ms (0.71%); the one host-only
paging-warning block remains in the primary result. The
[frozen Python S02 subset](s02-python-comparison-protocol.json) now fixes five
alternating pairs of 20 measured sequences per block, 100 per version for
p50/p95, and explicitly omits p99. The subsequent
[audited Python S02 paired run](results/2026-10-07-python-s02-paired-current/README.md)
accepted all 100 samples per version. The candidate's pooled p50/p95 were
+0.158%/+0.076% versus 0.8.26; all ten blocks had host swap-counter movement
with zero measured-child swaps, so no warning-free sensitivity pair exists.
That earlier run preceded TypeScript and Rust S02 timing. Contention and the
broader Phase 1 protocol remain ahead.
The later [integrated-candidate Python functional receipt](results/2026-10-07-python-integrated-candidate/README.md)
uses a clean installed wheel containing both off-ladder landings. Its
independent audit accepted 40 executed operations, one committed-closure
gap, three unavailable provider/model conditions and 47 reopened real-DB
snapshots. The S02 sequence also passed materialized-state and tampered-state
checks. These are functional results; the single validation-inclusive S02
duration is not a new paired latency estimate.
The [external Rust SDK S02 functional consumer](results/2026-10-07-rust-sdk-s02-feasibility/README.md)
exercises the candidate-only Rust boundary through a separate Cargo package
and retained real database. It is source-bound, not a published-crate install;
0.8.26 has no Rust SDK peer. Its one run does not qualify S02 latency or the
remaining Rust operation contracts.
The [repaired-source Rust S02 refresh](results/2026-10-07-rust-sdk-s02-current-refresh/README.md)
rebuilt that external consumer after the vector error fix and repeated its
real-database sequence with direct reopened-state checks. This remains a
single candidate-only functional run, not qualified Rust S02 timing.
The later [frozen candidate-only Rust S02 subset](rust-s02-candidate-timing-protocol.json)
used a five-block noise pilot before 100 fresh-process measurements. Its
[independently audited timing result](results/2026-10-07-rust-s02-candidate-timing/README.md)
has whole p50/p95 of 5,754.390/5,815.786 ms and a reopened-open p50 of
4,767.522 ms. It is not a paired 0.8.26 comparison. The broader Phase 1
protocol and bounded contention remain open.
The later [integrated-candidate Python S02 paired run](results/2026-10-07-python-s02-paired-integrated/README.md)
kept the earlier frozen workload, order and sample design while binding the
post-Slice-132 wheel with both off-ladder landings. All 100 measured sequences
per version passed independent semantic, resource and order audits. The
observed whole-sequence p50/p95 were +0.495%/+0.924% versus baseline; every
pair has a host swap-counter warning and one candidate child had a major
fault. This is diagnostic and cannot establish equivalence or a release
performance verdict. It supersedes the earlier Python S02 result for the
integrated candidate, without freezing the broader Phase 1 protocol.
The later [vector-repaired Python S02 subset](s02-python-vector-repaired-comparison-protocol.json)
kept the workload and 100-sample-per-version design while binding source
`3f29d649d`, its installed wheel and campaign code before timing. Its
[paired refresh](results/2026-10-07-python-s02-vector-repaired-paired/README.md)
passed ten blocks, 200 independently checked whole sequences and the
execution-order audit. Pooled p50/p95 changes are +0.010%/+0.155%, with
four warning-free pairs. Close and reopened-close stage leads persist;
the small whole-sequence differences do not establish equivalence.
The [frozen installed-Python S02 contention subset](s02-python-contention-comparison-protocol.json)
added synchronized readers and writers sharing one engine handle, then
projection drain, evidence, erasure and close/reopen state checks. Its
[paired diagnostic](results/2026-10-07-python-s02-contention-paired/README.md)
passed 100 fresh-process sequences per version and independent raw and
reopened-state audit. Pooled p50/p95 changes were +0.373%/−0.143%; all five
paired p50 changes were positive, and all pairs had host-only paging warnings.
The result does not establish equivalence. Rust SDK contention and the
broader Phase 1 protocol remain open.
The [installed TypeScript S02 contention feasibility
pair](results/2026-10-07-ts-s02-contention-feasibility/README.md) later
confirmed actual shared-handle writer/reader overlap on both installed
packages, with independent semantic and reopened-state checks. Its corrected
timer leaves direct SQLite verification outside the whole sequence; one
functional run per version does not qualify a latency delta.
The [baseline-only TypeScript contention pilot](results/2026-10-08-ts-s02-contention-baseline-pilot/README.md)
then passed five independent blocks and 15 measured fresh-process sequences.
Block medians spanned 81.814 ms; two blocks had host-only paging warnings and
no child swapped. The subsequent
[frozen contention subset](s02-ts-contention-comparison-protocol.json)
fixes five alternating pairs, 20 measurements per block and 20-second idle
gaps, binding the runner, auditor, installed archives and source before
candidate timing. The pilot did not supply a paired latency result. The
subsequent [paired TypeScript contention diagnostic](results/2026-10-08-ts-s02-contention-paired/README.md)
passed 100 measured fresh-process sequences per version and an independent
raw, resource, overlap and reopened-state audit. Pooled whole-sequence p50
increased 3.489%; all five pair-p50 changes were positive. Four pairs carried
host-only paging warnings; one warning-free pair retained a positive p50
change. This is a bounded installed-SDK latency lead, not a release verdict.
An earlier candidate changed with the graph traversal row-error
[repair](results/2026-10-07-graph-arm-row-repair/README.md) at `3ce1a6352`.
The prior functional receipts still describe their exact SHAs; final Phase 1
timing and installed-SDK qualification must use rebuilt artifacts from the
checkpoint candidate.
The later [installed Python capability exercise](results/2026-10-07-python-capability-exercise/README.md)
rebuilt a wheel from the repaired candidate and accounted for all 44 governed
operations: 39 selected cases executed, one failed frozen-error field
contract, one committed-closure gap and three unavailable provider/model
cases. This is functional evidence; it does not qualify S02 latency or the
complete Python contract matrix.
The [binding repair and rebuilt-wheel receipt](results/2026-10-07-python-frozen-error-fix/README.md)
then passed the four red field cases and 36 installed frozen-read tests. Its
refreshed operation exercise has 40 executed, zero failed, one committed-
closure gap and three unavailable cases. Engine/query/schema bytes used for
the paired E01–E12 cells did not change.
The [current installed Python S02 functional refresh](results/2026-10-07-python-s02-current-refresh/README.md)
passed the mixed sequence through that rebuilt wheel with independent
observation and archive-byte checks. Its single validation-inclusive timer
does not qualify S02 latency or contention.
The [current installed TypeScript S02 functional refresh](results/2026-10-07-ts-s02-current-refresh/README.md)
passed the equivalent sequence through the rebuilt npm packages with
independent observation and archive-byte checks. Its single
validation-inclusive timer has the same latency and contention limits.
The [installed TypeScript capability exercise](results/2026-10-07-ts-capability-exercise/README.md)
accounts for all 44 governed operations with 39 selected positive cases,
two explicit positive-path gaps and three unavailable provider/model cases.
The [external Rust SDK exercise](results/2026-10-07-rust-sdk-capability/README.md)
accounts for the same live set with 42 selected cases and two unavailable
provider/model cases. Its source-bound Cargo package is candidate-only and
does not qualify a published Rust artifact or a version-paired SDK result.

## Source-grounded starting point

Slice 115's [runner](../../../../../scripts/slice115_runner.py),
[workload](../../../../../scripts/slice115_workload.rs), and
[protocol](../slice-115/protocol.json) cover twelve engine paths: fresh/open
populated, close, canonical write, projection, small CPU model probe, text,
vector stage, hybrid, graph expand, graph evidence and erasure. The runner is
locked to its own source SHA, Cargo lock, model and seven-sample count. It
cannot be run unchanged as a 0.8.26 versus final-0.8.27 comparison. Reuse
the cell semantics and receipt validation pattern; build a separate Slice 135
adapter with red tests for wrong source, state, sample count and output.
The [paired E01–E12 feasibility receipt](results/2026-10-07-e01-e12-feasibility/README.md)
confirms that the inherited workload executes all twelve paths on both exact
sources. It exposes the adapter work still required: environment snapshots,
exact query IDs/order, pinned model bytes at run time, a true protocol binding,
and pilot-derived sample counts. Its seven observations per cell are not a
qualified latency comparison.
The later [source-bound E01–E12 adapter and baseline noise pilot](results/2026-10-07-e12-adapter-pilot/README.md)
now satisfy those adapter and evidence-binding requirements. Five separate
baseline blocks passed with 100 valid text and close/reopen observations each;
one full-path run per version passed 12 cells with 100 observations each. The
candidate full-path run used superseded engine bytes and is functional
feasibility only. The [frozen E01–E12 comparison subset](e12-comparison-protocol.json)
sets 1,000 valid query samples, 100 lifecycle samples and five alternating
version pairs before current-candidate paired timing. It does not freeze the
broader installed-SDK, S02, S03 or C01 protocol.
The [pre-integration E01–E12 paired diagnostic](results/2026-10-07-e12-paired-current/README.md)
passed all 20 blocks and found repeatable vector/hybrid median-latency leads.
A later [integrated-candidate E01–E12 smoke](results/2026-10-07-e12-integrated-smoke/README.md)
passed all twelve unchanged paths at 100 samples each with an independent
audit. Because the candidate Rust tree changed, the
[integrated E01–E12 paired subset](e12-integrated-comparison-protocol.json)
refreezes the exact candidate source and tree before the paired refresh; it
preserves the original workload, sample counts and order. The smoke itself is
functional feasibility, not paired latency evidence.
The [integrated-candidate paired diagnostic](results/2026-10-07-e12-paired-integrated/README.md)
then passed all 20 blocks and its independent audit accepted all twelve cells.
Its query blocks were warning-free; five lifecycle blocks carried host-only
swap warnings, with zero measured-child swaps. Median within-pair p50 changes
were +8.60% for vector stage, +7.99% for hybrid and +11.82% for populated
open. These are investigation leads, not a performance-equivalence or release
verdict. The archive is local and untracked pending final Gitleaks retention.
The later [vector-repaired E01–E12 subset](e12-vector-repaired-comparison-protocol.json)
froze product source `3f29d649d`, runner bytes and the same workload after an
audited 12-path smoke. Its [paired refresh](results/2026-10-07-e12-vector-repaired-paired/README.md)
passed 20 blocks and independently re-audited from the retained archive.
Median within-pair p50 changes were +8.79% for vector stage, +8.04% for
hybrid and +11.16% for populated open, each positive in all five pairs.
Four blocks had host-only paging warnings and no measured-child swaps.
These remain engine-boundary investigation leads; the installed-SDK and
contention refreshes are separate obligations.
A separate [futex profile](results/2026-10-07-vector-futex-profile/README.md)
found about 2.48 times as many whole-process futex calls in the candidate
query workload. The trace is attribution evidence, not an unprofiled latency
measurement or proof of a dispatcher root cause.

Historical 0.8.26 gauntlet receipts are context, not matched system timing:
the [result report](../../../../performance-benchmarking/gauntlet-v1.2/RESULTS-0.8.26.md)
separates retrieval scores from latency and records different boundaries.
Readiness snapshot on 2026-10-06: this host's
`/proc/sys/kernel/perf_event_paranoid` is `4`, so the Slice 115
GDB sampling fallback is available if unconfined ptrace is permitted. A
different host may qualify `perf`; record its setting. This checkout lacked a
checkout-owned `.venv` and LLVM coverage tools at that snapshot. Resolve
these before claiming binding tests or measured source coverage. The
worktree filesystem had about 53 GB free; estimate corpus, index and
temporary build footprint before any heavy comparator re-ingest.

## Workload cells to freeze

| ID | Boundary and scenario | Samples and validity |
| --- | --- | --- |
| E01–E12 | Port the twelve Slice 115 engine paths to separately labeled 0.8.26 and final-candidate runs; preserve pre/post semantic checks and fresh mutation state. | Query cells need at least 1,000 valid observations for p99; lifecycle cells at least 100 for p50/p95. A path that cannot meet this count reports only supported statistics. |
| S01 | Installed SDK call through materialized text/hybrid/vector result, with cold and warm read states and two corpus sizes. Map the completed Slice 132 Rust SDK and the Python/TypeScript methods to matching capabilities. | Pair Python and TypeScript where the same capability exists in both versions. Pair equivalent Rust engine cells: 0.8.26 has no Rust SDK crate, so the new Rust SDK is candidate-only and cannot support a version-paired installed-SDK comparison. Keep each boundary labeled. |
| S02 | Mixed sequence: open, governed write, projection-to-ready, text/vector/hybrid retrieval, graph/evidence retrieval, erasure, close/reopen. Include single caller and bounded contention. | Time whole sequence plus declared stages; verify state at each transition. A stage timer cannot substitute for the end-to-end timer. |
| S02-L | Repeated open/close cycles with the same default model, including a retained closed engine handle and a later handle drop. Compare 0.8.26 with a candidate containing both off-ladder fixes at or after `ef4bb42da`. | Record close latency and process memory before open, after model use, after `close()`, and after handle drop for each version, with identical idle intervals and at least 100 valid cycles after baseline noise qualification. On Linux retain RSS plus `/proc/self/smaps_rollup` PSS/private bytes; mark unsupported fields elsewhere. The engine-owned embedder release at close is an intended change, checked separately from caller-owned and module-level model lifetimes. Preserve the focused ownership/deadlock tests as the contract oracle; allocator retention means RSS alone is diagnostic. |
| S03 | Qualified LOCOMO, LongMemEval, MuSiQue or query-correctness cases selected for distinct filter, temporal, graph, evidence and memory load shapes. | Record subset selection, adaptation, answer-bearing fixtures used for basic validity, and denominators. Phase 1 measures workload cost and exercised paths, not the full gold-quality score. |
| C01 | Matched native Mem0 warmed external client-to-materialized-top-10 LOCOMO cell. | Validate the pinned index before reuse; paid re-ingest has a $20 ceiling and requires checkpoint/resume/backoff. Report separately from 0.8.26 comparison. |

The [C01 qualification check](results/2026-10-07-c01-qualification/README.md)
found the pinned raw LOCOMO corpus and nonempty historical Docker volumes,
but the external harness, configuration and output root required to bind the
old index to the frozen input are missing. It records a raw-backed omitted
cell, not a matched competitor timing result. The $20 re-ingest ceiling was
not used.

The [installed Python S02-L baseline-only pilot](results/2026-10-07-python-s02-lifecycle-baseline-pilot/README.md)
qualified five environment-checked blocks and 100 fresh-process cycles. Its
independent audit and semantic negative control precede the
[frozen S02-L paired subset](s02-python-lifecycle-comparison-protocol.json),
which binds the integrated candidate wheel and runner bytes. The
[paired campaign](results/2026-10-07-python-s02-lifecycle-paired-current/README.md)
has 100 valid cycles per version and an independent audit: close p50 increased
141.2% while median candidate PSS release at close was 45,719 KiB versus
-33 KiB on baseline. This is a measured lifecycle-boundary regression with an
intended memory benefit; whole-system impact and release disposition remain
open. This subset does not freeze the broader Phase 1 protocol.

All paired cells use fresh databases from a byte-identical seed, a verified
release artifact or exact source build, the same eligible operation, model,
settings, cache state, host, storage and concurrency. Record intentional
capability differences rather than masking them. Timer start is client
invocation; timer end is materialized output. Setup and verification are
outside the timer except named lifecycle/ingest cells. Alternate version
blocks to reduce drift. Record full raw samples, failures and run order.

Run a baseline-only noise pilot before freezing the sample count and
uncertainty method. Percentiles use a declared nearest-rank rule. Report
paired deltas, throughput, CPU, memory, I/O and backlog where measured.
Unprofiled latency is the primary timing; instrumented/coverage/profiler
runs are attribution and test-quality evidence with overhead controls.
Slower or inconclusive results are diagnostic; accepted release gates are
reported separately.

### Baseline-only noise pilot procedure

After the adapter's negative fixtures and exact 0.8.26 build provenance pass,
run at least five **separate** unprofiled baseline blocks, each with a unique
output directory and at least 100 valid text-query and 100 valid close/reopen
observations. Use the same 32-row seed, model, settings, host and artifact in
every block. Keep one warm-up per cell outside the timed samples. Preserve
each block's raw attempts, validator summary, start/end environment snapshots,
command, runner/protocol/artifact hashes and any invalid attempt. Run blocks
serially using the same hash-verified workload binary without rebuilding
between blocks. Let the host return to the recorded idle/thermal condition
after the qualification build; run without competing builds or profilers.
Never replace a slow valid sample with a rerun. A single passing block is an
adapter smoke test, not a noise estimate.

Report the per-block p50/p95, valid-attempt fraction, elapsed block time and
start-order trend, plus spread of block medians and p95 values. Treat drift or
invalid environments as a fixture problem and repeat the baseline pilot after
the cause is addressed. Use the baseline spread to justify final block and
sample counts and a paired uncertainty rule; the current 100-sample pilot
blocks cannot support p99. Freeze those decisions and the workload mix before
examining paired candidate timing. Retain baseline blocks even if the final
comparison protocol needs a larger sample count.

## Pareto and logic coverage overlay

Use the fixed query-correctness/benchmark workload mix as a **proxy**, with
operation frequencies and measured total cost by operation and material
condition. Rank by aggregate elapsed time and separately by CPU/queue cost.
Report the smallest set accounting for 80% of observed cost and its fraction
of paths; do not presuppose a 20% code share. Keep severe rare cases visible.
Capture test and workload line/branch coverage in separate compiler-
instrumented runs and map hot branches to tests. A targeted plausible mutant
must be detected for at least one high-use assertion gap. Run existing static
checks, then audit `unwrap`/`expect`, error conversions, cleanup and FFI panic
containment on the ranked and boundary paths. Classify every finding.

## Robustness matrix

An interim [projection commit recovery diagnostic](results/2026-10-07-projection-recovery/README.md)
ran six real-database, debug-hook tests once through Cargo and ten further
times through the same test binary. It covers one-shot busy/storage failures,
worker/subscriber panic cleanup, redispatch, mean-pin rollback and one
stop/reopen schedule. The full state/fault matrix below remains open.
The focused [current-product provider and close result](results/2026-10-07-provider-close-current/README.md)
adds 14 passing real-database cases for provider timeout/error, bounded
dispatch, close cancellation and pending-projection recovery after reopen.
Expected injected provider panics are explicit Rust caller-boundary assertions;
FFI containment and the remaining fault positions still require separate
evidence.
The [interrupted-erasure reopen diagnostic](results/2026-10-07-erasure-reopen-current/README.md)
checks a durable pending telemetry redaction across close and fresh engine
open. Retry without the original sink refuses completion; restoring it clears
the obligation and preserves a control record. This closes one named schedule,
not the remaining erasure or persistent-fault rows.

| Fault or schedule | State oracle after real-database reopen |
| --- | --- |
| Concurrent read/write and projection completion | Snapshot consistency, no missing committed record, truthful readiness and bounded worker shutdown |
| Process crash before and after a FathomDB-owned commit/queue transition | Canonical state and projection work either committed or absent as promised; replay/idempotence and typed status are truthful |
| Provider timeout/error before and after queued work | No false ready state, retained or explicitly failed work, bounded close and recoverability |
| SQLite busy and full-disk/permission failure at a named write boundary | No partial governed mutation; usable reopened database and exact error precedence |
| Close/cancellation under load | No stranded worker, outstanding handle or silent late commit |
| Interrupted erasure | No visible erased source after promised completion; interrupted work resumes or reports incomplete state truthfully |

At selected FathomDB-owned side-effect points, exercise one-shot and
persistent failures. Retain seed, fault point, schedule, before/after state
and resource counts. `PRAGMA integrity_check` is supplementary to FathomDB
state assertions. A process-kill case does not simulate torn or reordered
power-loss writes. Bounded resource pressure is separate from promising a
typed error for every Rust allocator exhaustion.

## Receipt and freeze criteria

Each attempt retains artifact/source SHA, runner/protocol SHA, corpus/model
digests, resolved settings, host/kernel/CPU/storage, toolchain and profiler
versions, call boundary, raw observations, semantic checks, environment
invalidators and invalid-attempt reasons. The independent summarizer must
reject mismatched hashes, altered settings, insufficient samples, missing
states and a deliberately wrong result. Disk pressure, competing heavy jobs,
swap, governor/storage drift and profiler substitution are explicit
invalidators. Do not silently discard slow valid samples.

The provisional [Slice 135 receipt validator](../../../../../scripts/slice135_receipt.py)
is the first bounded harness increment. Its protocol and raw JSON objects
both use `schema_version: 1`, exact 40-digit Git `source_sha`, 64-digit
`runner_sha256`, relative-path-to-SHA-256 `artifact_sha256` maps,
`features` and resolved `settings` maps. The raw object additionally binds
the exact protocol-file bytes with `protocol_sha256`; the command checks the
measured runner or bundle passed with `--runner` and each artifact's actual
bytes. Invoke it with `--raw`, `--protocol`, `--runner`, `--artifacts-root`,
`--source-sha` and `--output`. The summary records the validator's own hash
separately; it does not substitute for the measured runner hash.
Feature, setting and observed-state comparisons preserve JSON value types.
The source SHA is an explicit input and still needs separate source/build
provenance verification. A package version string is not candidate identity:
the current 0.8.27 source still uses a 0.8.26 pre-release version string.

Each protocol cell declares `kind` (`query` or `lifecycle`), `boundary` and
nonempty `expected_checks` for basic semantic/state validity. The matching
raw cell contains an ordered `attempts` array. A valid attempt has positive
integer `latency_ns`, `semantic_ok: true` and matching `observed_checks`.
An invalid attempt has `valid: false` and a nonempty `reason`; it remains
in the raw file and counts in the summary's `valid_attempt_fraction`.
An attempt that records `semantic_ok: false` or mismatched `observed_checks`
is rejected even if labeled invalid. A genuine no-output failure can omit
these fields while retaining its reason. Every cell
needs at least 100 valid attempts for nearest-rank p50/p95. A query cell
reports p99 only with at least 1,000 valid attempts; otherwise it records
`unsupported_statistics: ["p99"]`. Lifecycle p99 is never reported.
The summary includes counts, maximum, bindings and the exact raw-file hash.
No valid slow sample is trimmed.

The raw `environment` has `start`, `end` and an empty `invalidators` list.
Both snapshots require nonempty host, kernel, CPU, storage, governor,
toolchain and profiler strings; zero named competing jobs; integer swap pages;
and disk space at or above the protocol's `min_disk_free_bytes`. Stable
identity fields must agree. The validator rejects child swap events using a
hashed GNU Time resource report; host-only swap drift remains an exact
warning, as justified by the [mixed pilot](results/2026-10-07-mixed-noise-pilot/).
It rejects a declared invalidator. The producer must record truthful inventory
and semantic observations; this validator does not collect them, prove that
a feature was actually enabled, or produce a quality gold answer. Its
thresholds and shape for the remaining Phase 1 cells are provisional until
their own pilots and protocol review. The same script can validate separate 0.8.26 and 0.8.27
receipts by binding each to its own source, protocol, runner and artifacts.

Freeze the **full** Phase 1 protocol as an executable, hashed protocol only after the
adapter's negative fixtures pass, the baseline-only pilot fixes counts and
uncertainty, exact assets and installed SDK methods are known, and a review
confirms feasible disk/time budgets. The
[dated first-results note](results/2026-10-07-first-results/) fulfills the
bounded first-results milestone only. The full four-area expansion and
independent recomputation required by the approved plan will constitute the
Phase 1 checkpoint; this draft and baseline-only data do not cross it.
