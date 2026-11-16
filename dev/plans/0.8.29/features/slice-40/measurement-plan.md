# Slice 40 conditional selection method

Status: evidence table and bounded dependency amendment passed independent
Astra-medium review. The recommended dispositions remain unselected. No treatment, source entry, runtime or closure
is authorized here. The owner approved caching only if current measurements and
snapshot safety justify it ([Batch 3](../../prework/slice-9/decision-log.md#batch-3-tests-measurements-and-exact-search)).
The recommendation leaves the existing reference paths in place.

## Accepted inputs and what they establish

Current source was read at frozen parent
`2a6401da02123ddc0016023a4bd8e747ef3717da`: `search.rs`,
`dependency_closure.rs`, `projection_commit.rs`, `projection_registry.rs`,
`projection_generation.rs` and format37/frozen-read authority. No database,
model or GPU was opened; the numbers below reuse completed raw results.

- **P:** [accepted results](../slice-30/results-p.md), measured source `2fd772558e46d0b1645dce2ffea730d0bdd3305c`, default SDK, provider none, format37, no vector rows. The 16 cells contain 20,000 timed natural calls. Four unfiltered100k passes have p50 27.184–27.509 ms/p99 106.911–107.682 ms against tracked, nongated AC083 targets 80/300 ms. This establishes direct-text baseline behavior, not hybrid performance or cache benefit. P2's kind-filtered write/probe/search route differs from P1.
- **G:** all84 fixed outcomes are now independently accepted, including both100k read repeats and all72 G-W configurations. The retained full report is `summary-g-fb46a7d0197a43939b67238c49d5494c.json` under `/mnt/data/tmp2/fathomdb-0829-continuation/slice30-g-20261113/gd-allocation/g-closed-write-matrix-v2`:144 populations/97318 successful samples/0 errors in the selected qualified outcomes. Read-mostly engine/SDK text p50 is27.195–28.422ms/p99105.894–108.628ms; hybrid p50184.123–187.191ms/p99348.599–351.946ms. Across24 write-interleaved hybrid populations, p50 is263.954–282.966ms/p99544.601–586.823ms. These are reference workloads, not cache-treatment comparisons or component-cost attribution. Original failed/refused attempts and native producer identities remain retained; current Slice50 performance is separate.
- **G attribution:** `g-o-natural-r0-on` and `g-o-natural-r1-on` each contain 300 observations: 50 queries per route/adapter, no warmup. They are descriptive instrumented first-50 subsets, not canonical latency percentiles. Component means/shares below are per adapter/repeat; share is summed component duration divided by summed enclosing API duration on the same records. No percentile subtraction or ordinary-minus-instrumented speedup is used.
- **Raw provenance:** P retains `p-campaign-v1/summary.json` and per-cell raw under `/mnt/data/tmp2/fathomdb-0829-continuation/slice30`. G retains `manifest.json`, `results/g-outcomes.json` and each named `results/<cell>/attempt-0001/read/observations.jsonl` under `/mnt/data/tmp2/fathomdb-0829-continuation/slice30-g-20261113/gd-allocation/g-frozen-g-assessment-v5`. The manifest owns actual artifact/model/input identities; ordinary/diagnostic native builds remain their original `5c7b1988d342488288ae83e022b358254c60edc8` subjects, not new builds at 29de. Six reused cases retain original provenance.

## Current source and write-evidence recheck (2026-11-16)

Status: completed-G84 reconciliation accepted by independent Astra-medium review; actual owner treatment/deferral selection remains pending.

Rechecked `040b66b8ad21b3599e2a9d259f9ef9dce11afef3` against the earlier
`2a6401da02123ddc0016023a4bd8e747ef3717da` inventory. Eligibility, ReadView,
reader ownership and attribute descriptor helpers are unchanged. Search now
includes Slice 50 record identity/winner hydration; the direct-check guards and
three eligibility functions are unchanged. Old G-O exclusive SQL duration remains
attributed to its original executable; its API share is not a current hybrid share.
Projection commit has additional measurement attribution around unchanged per-row
shape/registry/fragment work. Generation, effective time and initial mean remain
batch-scoped; mean may still pin mid-batch.

The complete G-W baseline now supplies every registered write/search schedule.
Before timed sample0, batch1 replaces `s30doc-0`; batch10 replaces all of
`s30doc-0..9`. Registered schedules
create10/100/1000/10000 replacement versions; physical rows grow from100000 to
100010/100100/101000/110000 while current natural records remain100000. Ordinary
native provenance stays5c7b1988d342488288ae83e022b358254c60edc8; the completed
controller/report is2d6. The original G-O component observations remain attributed
to their original subject, not the current Slice50 route. No paired cache
candidate, key validation cost, hit rate or descriptor treatment was measured.

G-W updates existing logical records, retaining superseded canonical versions.
On strict hybrid searches any such row makes `all_nodes_directly_eligible` false;
its EXISTS can stop at the first unsafe row and keeps reference FTS. This is a
source inference, not a quantified G-W SQL measurement. The 48–49 ms all-true
G-O scan cannot be charged to every G-W query. Relevant commits also invalidate
cross-read authority. No constant-time claim or measured cache saving follows.

A plausible bounded candidate for later study is **only** the strict whole-node
direct answer: one entry per reader worker/connection, key read in the consuming
transaction's pinned snapshot from clean visibility generation/branch nonce plus
exact resolved epoch second. Preserve every route guard; misses execute reference
SQL. Old/new WAL readers may replace the entry and lose hits. Drop with worker;
reopen/copy cannot share it. Source fallback, dependency presence and frozen
validation remain separate and fresh. No persistent metadata or retained snapshot.
This is not a completed proof/approved key: schema/trigger identity or conservative
DDL refusal, main/TEMP/external-write invalidation, malformed/armed-state precedence
and fixed byte bound require review before selection. Never reuse primary-connection
status or a latest atomic as reader authority. Exact time remains necessary: the
accepted source-window control changes fallback with identical generation/nonce.

Recommendation remains no new Slice 40 treatment for .29: NOT_SELECTED local
eligibility memos, cross-read presence, dependency-free fallback memo and duplicate
batch generation/time/mean hoists; DEFER material cross-read direct eligibility,
source-bearing fallback and shape/registry/fragment batch descriptors. Presence is
already once/query and tiny. Direct eligibility has material original read cost but
no current paired benefit/hit/miss evidence. Source-bearing scale and batch residual
cost are unmeasured. Descriptor invariance additionally needs trigger-sensitive
admission; IMMEDIATE excludes external writers, not same-transaction triggers.
This is a release-scoped conditional choice, not measured-negative or impossible-
safety claims. The actual owner choice is still required and has not been recorded.
No whole30/Windows/M/D/F closure or new calibration campaign is needed to accept
that exclusion. Current Slice20 functional + 238 common-source tests and S60/70
are closed; stale Windows-preparation prose is not a Slice40 hold. No source entry,
prototype, new cache tests or adoption-only campaigns are authorized by this memo.

## Concrete opportunity and disposition table

All dispositions are recommendations for review/owner choice. Unknown costs are
unmeasured, not zero. Existing statement/page caches receive no new saving.

| Opportunity and current consumers | Accepted cost, actual change and reuse opportunity | Proposed disposition and adoption-only gap |
| --- | --- | --- |
| Dependency presence: `has_source_dependencies`; one call in `read_search_in_tx`, captured by arm predicates | Already one prepared `EXISTS` per search transaction, not per arm. G diagnostic text means 0.00494–0.00657 ms (0.0312–0.0408% of observed API totals); hybrid 0.00996–0.01452 ms (0.00652–0.00951%). A local memo has no second consumer; cross-read caching adds authority/lookup/invalidation to avoid this one small query. | **NOT_SELECTED** another local/per-arm memo and cross-read presence cache for .29 on this evidence. Source-bearing scale and cache overhead are unmeasured; no universal no-benefit claim. Preserve existing reuse. |
| Whole-node direct eligibility: `all_nodes_directly_eligible`; one guarded deferred-text decision per eligible hybrid search | Direct text never calls it in G. Each of 200 observed hybrid calls executes SQL once and returns true. Per adapter/repeat means 48.336–49.223 ms, 31.994–32.439% of observed API totals. A local memo avoids no duplicate. A cross-read boolean could avoid the scan on a proven hit; native pre-cap KNN eligibility and all deferred-route guards must remain. | **NOT_SELECTED** local memo; **DEFER** cross-read treatment. This is a material read opportunity, not a measured negative. Component cost is only a gross per-hit upper bound, not promised latency saving. Missing: affected hybrid hit/miss/authority costs, safe reader key/time/invalidation proof, bounded ownership and current paired treatment evidence. The actual G-W update schedule also introduces superseded rows, so its direct check need not execute the all-true full scan. |
| Source fallback: `vector_arm_requires_fallback`; one conditional call per vector route | Dependency-free G hybrid returns false without SQL; descriptive means 0.01048–0.01655 ms (0.00686–0.01111%). Direct text skips it. Cached false cannot replace direct-node proof. Source-bearing branch controls execute SQL and flip false→true across a validity boundary with unchanged generation/nonce. | **NOT_SELECTED** memo of the existing dependency-free return; **DEFER** source-bearing cross-read reuse. Large source-dependent cost/reuse is unmeasured. Time/view and unconditional closure-barrier proof are hard adoption needs; no general fallback-cache rejection. |
| Generation, effective time and initial mean in `commit_projection_outcomes` | Already once per admitted IMMEDIATE batch. `current_mean` changes at threshold pin; accumulator/events publish only after commit. Re-hoisting supplies no incremental saving. | **NOT_SELECTED** duplicated hoists. Preserve mutable mean and row-specific eligibility/terminal/physical checks. |
| Live attribute-column shape: `actual_vector_attr_columns` | Every successful projected row checks shape; attribute-bearing rows read it again inside `vector_attr_insert_fragments`. Batch reuse could remove repeated shape reads. No-attribute path already avoids body lookup. | **DEFER**. No isolated shape cost, multi-outcome saving or write regression/memory evidence. Need fresh trigger-sensitive invariance/admission proof and material residual before selection. |
| Registry extraction descriptors: `load_projection_registry` in `vector_attr_insert_fragments` | Loaded per attribute-bearing success; potential reuse across outcomes in one batch. Body/scalar extraction and lifecycle/validity stay per row. P has zero vectors; G seed/drain totals do not isolate this work. | **DEFER**. Need descriptor cost/reuse and invariance across every reachable trigger/mean-pin write, including configuration changes with unchanged column names. IMMEDIATE alone only excludes external writers. |
| Attribute SQL/placeholder shape in `vector_attr_insert_fragments` and projection commit | Rebuilt per attribute-bearing row; candidate reuses validated shape/fragments only, still binding every live column and preserving absent versus present-empty encoding. | **DEFER**. No component cost/material residual established. Need descriptor invariance, row/error-order equivalence and bounded per-batch lifetime; no retained row bodies/scalars. |

P2 has governed-write means 1.316–1.834 ms for 1-row calls and 3.360–3.459 ms
for 10-row calls, outside natural API timing. These are filtered provider-free
writes with marker probes, not projection-outcome batch cost or candidate
invalidation overhead. Search snapshot authority is unavailable in P; write
receipt cursors cannot prove cache reuse. The completed G84 write matrix supplies the missing reference write/interleaved
hybrid baseline. It does not supply exclusive eligibility/descriptor cost or a
cache-treatment comparison. Missing baseline cells are no longer a deferral reason. Candidate hit rates, key/miss cost, retained bytes
and invalidation cost are unknown.

## Decision and exact remaining conditions

**Recommended owner disposition:** retain .29 reference paths, adopt none of the
new Slice 40 treatments, accept the table's NOT_SELECTED redundant/small-work
opportunities, and explicitly DEFER the material cross-read direct check,
source-bearing fallback and unmeasured residual batch descriptors. This avoids
a measured-negative claim for a real hybrid opportunity. Conditional-scope
closure requires independent review and the owner's actual choice; this draft
neither selects nor closes Slice 40. The alternative is to keep those opportunities
open for adoption after their exact missing evidence arrives. There is no
supported adopt-now option, prototype or new measurement campaign here.

| Outcome | Required evidence or action |
| --- | --- |
| Adopt eligibility treatment | Consume qualified Linux 20 subset `657edeb76caaf9c8d2aab433766bec9362e73217`, A3/A5 proof/ruling `f803a964ad8c8154a481e6f9385599bef5c0e054`, P/G baseline above and actual affected G-W committed-write/search schedules. Resolve 100k repeat/noise uncertainty where comparison requires it. Demonstrate hits under authority/time churn and full lookup/miss/write/memory cost. Fix benefit/regression tolerances before paired treatment runs. |
| Prove cross-read correctness before source selection | Establish/read authority in the consuming reader transaction's own SQLite snapshot. Bind database/branch nonce, complete relevant invalidation state, effective time and view. Prove external writes/custom triggers, old/new WAL readers, reopen/diverged copies, fresh clean-state refusal and frozen reproduce-or-fail. Primary-connection status caching/shared latest generation is no reader-pool proof. G's source-window control has identical generation/nonce at consecutive times and different fallback answers: generation alone fails. Numeric entry/byte limits, eviction and unsupported-proof fallback remain unset. |
| Adopt batch descriptors | Attribute repeated shape/registry/fragment costs in current multi-outcome projection batches and establish material saving after admission/miss cost, including read/write regressions and memory. Prove invariance across reachable main/TEMP custom-trigger and mean-pin paths, or fresh reviewed bypass preserving effects/errors. Existing coalescing admission is not automatically that proof. No per-row body/scalar/mean/eligibility hoist. |
| Select and qualify code | Concrete scope/safety/test review, prospective absolute/relative benefit and regression/memory rule, genuine per-behavior runtime RED then GREEN, paired reference correctness and affected schedules, required final code-phase gate and fresh Astra implementation review. No new global ACs. |
| Close by negative/defer disposition | Independent review verifies source/result distinctions and every opportunity disposition; owner explicitly accepts no new Slice 40 treatment for .29 and named deferrals under Batch 3 conditional approval. Parent records actual choice in existing release state. No cache, new test, paired treatment run or adoption-only proof is required to claim an authorized exclusion; unavailable evidence is not PASS. |

Hard adoption dependencies are relevant qualified Linux 20/P/G inputs, affected
write-interleaved baseline, selected safety/invariance proof and reviewed
benefit/correctness rule. Whole 20 Windows A6, whole 30 completion, M/D/frozen-TC5 F,
pack reconnection and unrelated planning are not technical prerequisites for
this decision or the accepted artifact narrowing. Their release gates remain
open. The bounded narrowing is accepted; source entry remains HELD on the
treatment-specific adoption conditions. Changed relevant 50/60/70/source contracts need scoped
reconciliation before eventual adoption; 40 before 50 is attribution/merge
preference, not a semantic dependency.

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
