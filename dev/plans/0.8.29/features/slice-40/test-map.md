# Slice 40 conditional test strategy

Status: proposed test obligations, no tests written or executed. Exact tests
are selected only after actual30 benefit evidence and concrete safety review.
Existing coverage must be read before claiming a gap; no passing-on-arrival test
can satisfy the required runtime RED. Use owned real WAL databases, existing
managed connection attribution and barriers/channels instead of sleeps.

| Behavior / local AC | Required runtime assertion for selected change | Key controls |
| --- | --- | --- |
| Eligibility reuse / A40-1/2 | Human-specified eligible identities and pre-cap candidates agree with uncached control, including order/scores, cursor, typed fallback and errors; work-count RED proves repeated work reduction. | Direct-node false/true, dependency absent/present, strict/inactive/superseded/out-of-window combinations, all route guards and edge expiry. |
| Reader authority / A40-2/4 | Old WAL reader keeps its own answer while new writer/reader sees committed changes; no shared latest value authorizes either. | Canonical writes; lifecycle/validity; dependency register/closure; projection success/failure/no-op; configuration/rebuild/recenter; erase/purge; external ordinary SQLite commit; close/reopen and equal-generation/different-nonce copied branches. |
| Time / A40-2 | Start is inclusive, end exclusive; source/record time transitions occur without writes and without stale answers. | Exact before/at/after boundaries, empty/null/extreme intervals, backward explicit effective time; every view combination and both warm/miss routes. |
| Frozen / A40-2 | Existing token authentication/drift precedence and reproduce-or-fail remain; ordinary and frozen results each match their own context. | Foreign/diverged store, changed registry/serving state, old/new reader barriers; retain pinned historical token bytes and add new fixtures only when human-authorized. |
| Coalescing / A40-2/3 | No armed or rolled-back value becomes reusable; malformed/committed-armed state refuses through existing typed behavior before effects. | Savepoint, ABORT/ROLLBACK trigger, commit-hook refusal, retry/reopen, generation exhaustion, clean external invalidation; preserve authenticated vector-span DELETE cleanup. |
| Batch descriptors / A40-3 | Runtime metadata/query counts decrease for multi-outcome batch; vector bytes/attributes/spans/terminal/cursor are independently correct. | Zero/nonzero attrs, absent/present-empty values, changed nested source with same shape, mixed kinds/success/failure/deferred, stale generation, duplicate terminal, physical tuple corruption. |
| Trigger-sensitive batching / A40-3 | Trigger path observes its real per-row metadata effects and cannot use stale hoisted descriptors; optimized no-custom path matches reference. | Main and TEMP triggers, spoofed names, metadata mutation on success/failure/mean-pin reachable writes; do not assume existing coalescing admission proves descriptor invariance. |
| Mid-batch mean / A40-3 | Threshold crossing, retroactive requantization, accumulator and MeanVecPinned event match reference; rollback leaves shared state/events untouched. | Crossing before/middle/after batch, failure after crossing and commit refusal; timestamps keep existing semantics. |
| Bounds / A40-4 | Reviewed numeric entry/byte bound and lifetime hold during time/view/database churn and eviction; miss cannot alter correctness. | Old-reader eviction, reopen, exact-time low hit rate, write-every-query, unsupported proof and allocation/lock error route within the stated failure model. |

Use stateful property tests for selected projection/recovery state changes:
write, supersede, validity, dependency closure, project, configure/rebuild,
erase and reopen. Add only the sequences needed for changed behavior and retain
minimized regression seeds. Compare optimized/reference routes on the same
snapshot, plus independently specified public eligibility rules and physical
vec0/sidecar/span/terminal assertions; shared predicates are not independent
oracles. Do not mock the database.

For each behavior: stage/commit the runtime RED first, observe its intended
assertion failure at that source, freeze its intent, then implement separate
GREEN. Missing symbols, compile errors, timeout and harness setup failure are
not runtime RED. Main session samples actual RED revisions independently. New
measurement math tests use hand-computed examples; no golden regeneration.

Run scoped lint/typecheck/affected tests in latency order; broaden only for
changed concerns. Required full `agent-verify` runs once on the final clean owned
code phase, with checkout-owned venv/modules/native artifacts. Preserve genuine
failures/platform gaps; do not borrow another slice's qualification. Fresh Astra
final review follows implementation and the scoped correctness/performance proof.
This documentation phase runs only scoped Markdown/link/source-citation checks.
