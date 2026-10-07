# Slice 135 first inspectable Phase 1 results — 2026-10-07 UTC

**Status:** one bounded, inspectable result in each of the four required areas.
This is the first-results milestone in the [approved plan](../../plan.md),
not the full Phase 1 checkpoint or a 0.8.27 release verdict. Dedicated
gold-answer correctness work has not started.

## Source, protocol and validity

The 0.8.26 baseline is peeled tag source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`. The measured candidate is
`50eaa5035b9a79ce2723251853846e6cc96ea7f7`, the integrated
post-Slice-132 source. Its engine/query/schema implementation bytes match
`8cbd330c8f83f35ef46d538ea23811c08af8066b`; later Slice 135 changes
add tests, scripts and evidence. Each receipt checks the source tree, resolved
Cargo lock, runner hash, corpus and operation manifest. The comparison used
the [frozen first-results protocol](../../first-results-protocol.json),
committed before candidate timing, and the five-block
[baseline-only mixed noise pilot](../2026-10-07-mixed-noise-pilot/).

The protocol fixes five alternating baseline/candidate pairs, 100 samples per
cell per block, one warm-up per cell, at least 20 seconds between blocks,
nearest-rank p50/p95 and a deterministic 10,000-draw paired-block bootstrap
of the median pair delta. P99 is unsupported. All ten paired blocks have
`VALID_BLOCK_RECEIPT`, 100 valid and zero invalid attempts in each of three
cells, exact expected semantic checks, zero child swap events and zero major
faults. Host-only swap warnings remain visible in candidate pair 1 (one page)
and baseline pairs 4 and 5 (one and six pages). All raw attempts and receipt
bindings are under [paired/](paired/); [SHA256SUMS](SHA256SUMS) covers the
retained evidence. The exact candidate release-profile workload binary is
retained as [candidate-workload.gz](candidate-workload.gz), with its
[build provenance](candidate-build/); the exact baseline binary is in the
[noise pilot](../2026-10-07-mixed-noise-pilot/baseline-workload.gz).

## 1. What matters / Pareto path

The fixed proxy mix made 100 observations each of text search, fresh close,
and a whole sequence of open, governed write, projection drain, text search,
erasure, close and reopen. In a separate candidate
[stage-attributed receipt](attribution/), an independent sum of each stage's
100 raw durations plus the standalone text and close-cell durations ranked
11 operations by aggregate measured cost. The first four reached **84.12%**
of this proxy's total attributed cost:

| Rank | Operation | Aggregate time | Share of proxy cost |
| --- | --- | ---: | ---: |
| 1 | Open in mixed sequence | 2.044 s | 45.37% |
| 2 | Reopen in mixed sequence | 0.835 s | 18.54% |
| 3 | Standalone fresh close | 0.536 s | 11.91% |
| 4 | Erasure in mixed sequence | 0.374 s | 8.30% |

The [independent Pareto analysis](pareto-analysis.json) has all 11 counts,
costs and cumulative shares. This is **four of 11 operations**, not evidence
that 20% of production code consumes 80% of real-user time. The synthetic
mix is a qualified first proxy; future consenting user traces and the full
Phase 1 workload matrix must test whether its weights are representative.
Child CPU was captured for the whole block, not attributed per operation.

The separate [GDB stack receipt](profile/candidate-mixed.json) has 16 samples
of a symbolized 15-second mixed loop, 14 with matching engine operation frames.
Open frames appeared in nine sample backtraces. The raw
[transcript](profile/candidate-mixed.transcript.gz) and unprofiled-control log
are retained; the profiled loop completed 344 operations versus 374 in its
control. Stack presence across all threads is not exclusive CPU time. No
profiled timing enters the latency result.

The [coverage evidence](coverage/) compares the same 100-observation fixed
workload with three existing real-database open/lifecycle test targets, using
separate Rust line and diagnostic branch instrumentation. `open.rs` had
634/1122 workload lines and 40/108 workload branches hit; the selected tests
had 690/1108 lines and 50/108 branches hit. All **40 workload-hit branch
identities** in that file were hit by those tests. Eleven workload-hit lines
missed by the selected tests are lines 680–689 and 691 in the test-only open
helper. For `connection_runtime.rs`, the selected tests covered all six
workload-hit branch identities shared by the two builds; one additional
workload-hit branch at line 528 is in a `test-hooks`-only pragma witness and
is absent from the selected test build. Raw compressed profiles, merged
profiles, LCOV exports, [build commands](coverage/commands.txt) and test outputs are retained. These
are selected tests, not a full-suite coverage claim. The existing
[search-path test coverage](../../pareto-test-coverage-2026-10-06.md) remains
a separate provisional lead; it is not substituted for the open-path overlay.

A [negative control](negative-control/) changed one otherwise valid mixed
attempt's reopened row count from zero to one. The independent validator
rejected it with `mixed_sequence: attempt 0 observed checks mismatch` (exit
1), demonstrating that this frequently exercised state assertion detects a
plausible wrong result. It mutates the receipt, not product code; product
mutation coverage remains for the full checkpoint.

## 2. System latency

The paired comparison used unprofiled release-profile real-engine binaries
and the same corpus, manifest, semantic checks, host and block order. The
`text` boundary is engine call to materialized result. `mixed_sequence`
times the complete open-through-reopened-check sequence; its stage timers
were disabled in these primary receipts. Fresh close is a supporting cell.
The [independent paired analysis](paired-analysis.json) rehashed every raw
file, rechecked 3,000 attempt-level semantic flags, recomputed all block
nearest-rank percentiles, then formed candidate-versus-baseline deltas within
each preassigned pair. Negative means candidate faster.

| Cell / statistic | Median of five pair deltas | Diagnostic paired bootstrap interval |
| --- | ---: | ---: |
| Text p50 | −2.79% | −7.04% to +1.10% |
| Text p95 | −2.78% | −14.75% to +2.38% |
| Whole sequence p50 | +0.00% | −0.98% to +0.58% |
| Whole sequence p95 | +0.88% | +0.72% to +1.47% |
| Fresh close p50 | +0.90% | −0.27% to +3.70% |
| Fresh close p95 | +2.09% | −1.78% to +2.95% |

Whole-sequence p95 was 0.72%–1.47% slower on the candidate in all five
pairs. This is a **follow-up signal**, not a regression verdict: five paired
blocks make the bootstrap interval descriptive, the baseline pilot showed
day-to-day text-tail variation, and host swap warnings appeared in three
blocks. Text p50 was lower on the candidate in four pairs, but its interval
crosses zero. No valid slow sample was trimmed. Baseline child CPU user time
was 9.41–9.58 s per block versus 9.38–9.54 s candidate; peak RSS was
20,752–21,336 KiB baseline versus 21,224–21,688 KiB candidate. The raw
resource receipts contain system CPU and I/O. Installed SDK, vector, hybrid,
graph, scale, contention and competitor cells remain outside this result.

## 3. System robustness

The [dated real-database robustness result](../2026-10-07-robustness-first/)
records three passing bounded cases on the same engine implementation bytes:
two concurrent readers and two writers with exact 17-row state after reopen;
process kill before and after an acknowledged write with correct reopened
state; and an injected pre-transaction refusal with no partial row, followed
by a successful recovery write. It retains commands, build/test logs,
before/after/reopened assertions, resource snapshots and time bounds.
`PRAGMA integrity_check=ok` is supplementary. A deliberately missing row
failed the negative control as intended. This does not simulate power-loss
write reordering or complete the fault/schedule matrix in the plan.

## 4. Logic and exception handling

Current disposition: the edge FTS defect below was repaired in `9186eb3d0`.
See the [permanent regression and verification receipt](../2026-10-07-edge-fts-repair/README.md).
This first-results section preserves the original finding on its earlier
source SHA; its engine timing does not qualify the repaired candidate.

The [dated diagnostic](../../logic-first-result-2026-10-06.md) found a
confirmed, still-open defect on the same engine implementation bytes: an
invalid UTF-8 BLOB in an edge FTS row makes the independent integrity check
report `SearchProjectionIdentityMismatch`, while `search_text_only` returns
`Ok([])` because `rows.flatten()` discards a row-decoding error. Its RED
real-database probe, raw output, audit of seven `rows.flatten()` sites,
error/panic/cleanup boundary notes and measured branch slice are retained.
The probe exits 101 as intended; it is not counted as a passing product test.
The other six flatten sites are unclassified leads. Repair requires a
test-first product change and rerunning affected measurements before Slice
135 closeout.

Standalone `./scripts/agent-lint.sh` and `./scripts/agent-typecheck.sh`
both exited zero on the integrated branch; [static/check-status.json](static/check-status.json)
and exact empty success streams record them. The full
`./scripts/agent-verify.sh` exited one after **180 of 182 run suites** passed
(184 registered, two skipped); its Rust suite passed. The two failing suites are
retained in [static/](static/): the real-repo orientation output exceeds its
4096-byte runtime cap while its test allows 5120, and Python has one
1132-versus-1131 declaration-count assertion plus two subprocess imports
missing `eval` under this checkout's non-editable install. The two imports
passed when rerun with `PYTHONPATH=src/python`; the declaration-count failure
remains. These failures do not turn the full gate green. No Semgrep or Kani
gate was added for this bounded result; the existing lint/typecheck, source
audit, dynamic fault probe and measured branch coverage supplied concrete
findings without a generic rule-pack claim.

## Next checkpoint and evidence limits

The [full Phase 1 protocol](../../phase1-protocol-draft.md) remains a draft.
Expand engine and installed-SDK cells, cold/warm and scale/concurrency
conditions, CPU/queue attribution, broader fault schedules, boundary audits
and coverage, then publish the full four-area checkpoint before dedicated
gold-answer correctness scoring. Investigate the whole-sequence p95 signal
and confirmed swallowed-error defect with test-first changes. The frozen
first-results source and workload identities must be requalified if the
release branch changes product bytes. A passing scoped result or this dated
note does not claim a release acceptance gate.
