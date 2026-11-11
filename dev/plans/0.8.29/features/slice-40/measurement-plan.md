# Slice 40 conditional selection method

Status: methodology only. No benchmark, tool/prototype implementation or native
execution is authorized by this document. Actual Slice 30 measurements are
absent; full20 and the exact TC5 input pack remain held. Do not replace them with
synthetic, legacy .28 or cache-friendly results.

## Receiving evidence and opportunity table

After 30's real exit, bind its source, actual native artifacts, workload inputs,
provider/backend, schema, query order, warmup/repeat schedule and resource limits.
Reread current source after 60 and any intervening engine changes. Preserve
read-mostly and committed write→search/batch-write→search schedules separately;
projection overlap/drain policy is part of each cell. TC5 remains the accepted
recall bridge; legacy EU7 is not a substitute. Preserve exact frozen inputs.

Populate one row per candidate: source function/caller, number of consumers,
observed component cost, end-to-end share, reuse opportunities per authority/time,
invalidations, full miss cost, key cost, memory cost, correctness proof status and
proposed disposition. Include dependency presence, direct eligibility, source
fallback and each residual batch descriptor separately. Existing per-call/batch
reuse receives zero invented incremental saving.

Estimate an upper bound before code: an answer can save at most the measured
work avoided on observed hits, less authority/lookup and miss bookkeeping cost.
Time/view churn and a committed write before each search may eliminate hits.
No speculative throughput multiplier, new counter table or schema fallback
follows a weak estimate. Transaction-local reuse needs an actual repeated
consumer; batch hoisting needs a material residual.

## Decision stages

| Evidence state | Permitted next step |
| --- | --- |
| 20/input/30 missing | BLOCKED for selection and source/runtime entry; bounded document review may proceed. |
| Current cost negligible, reuse absent or projected overhead consumes saving | Propose NOT_SELECTED for that opportunity with source/measurement evidence. No prototype needed. |
| Cost exists but snapshot authority/invariance proof incomplete | DEFERRED or BLOCKED with exact missing proof; preserve reference path. No performance shortcut. |
| Cost/reuse and safety both support a concrete treatment | Freeze exact scope, ownership/limits, tests and prospective tolerances; independent review before per-behavior implementation. |
| Qualified treatment improves only a narrower workload or regresses writes/memory | Narrow only within reviewed scope and report tradeoff, or reject/defer. Do not quietly make it a default. |
| Treatment passes reviewed benefit and correctness rule on both required schedules | Recommend adoption with uncertainty and release-gate results; fresh final code review remains. |

Before treatment measurements, reviewers must fix minimum meaningful absolute
and relative benefit, permitted regression, memory bound, percentile statistic,
repeat count/order and uncertainty method using 30's observed scale/noise. Those
values are currently **unset**; historical thresholds are not .29 policy. This
is a hard prerequisite for a treatment campaign, not a reason to run exploratory
caches now. Existing accepted release thresholds remain independently binding.

## Later paired comparison, if selected and authorized

Keep a trustworthy disabled-optimization/reference route in test/measurement
scope without a new production environment switch. Compare baseline,
eligibility-only, batch-only and combined only for actually selected treatments;
do not require an unselected treatment to exist. Build outside timing, own native
artifacts/environment, fresh controlled stores per repetition and balanced arm
order. Run serialized under reviewed 30 resource limits; no compilation or
unrelated campaign in the measurement window.

Report end-to-end p50/p99 and the preregistered comparison statistic, raw counts,
errors/degradations, scan/lookup/key/miss work, invalidation/hit rates, peak RSS
and retained bytes. Use sample-level measurements for attribution, never
subtract percentiles. Trace only separate diagnostic cells with overhead controls;
use untraced product calls for latency claims. Keep zero-hit/write-every-query,
validity-boundary and custom-trigger fallback controls. Preserve outliers,
aborts, missing cells and inconclusive results.

All future gate/probe/analysis tools must be committed, rerunnable and indexed
in the experiment README with origin/changes, commands and configurable inputs.
Test math/counts/cell ordering and real-database behavior; no tamper/drift/path
hardening suite or autonomous golden regeneration. Raw outputs stay outside
tracked source according to existing experiment policy. Source/script changes
receive the required final code-phase gate; raw measurements and scoped tests
alone cannot qualify a release.
