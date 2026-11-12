# Slice 30 staged measurement design

Status: P accepted at0d8/integrated2381. Historical initial implementation scope
below was **P only**. G/D bounded source prefixes through46ee7cf6e now have
independent acceptance; generic G artifact/route/budget fit remains planning-only
in [extension](gd-extension.md). No G/D campaign or final qualification follows.
M/F remain held; each further source stage needs its exact reviewed authority.
All prerequisites in [parent proposal](prerequisite-change.md) remain in force.

## Current source and original tool fit

The [architecture reconciliation](product-architecture.md) identifies the actual
callers. Bounded P uses existing engine/SDK direct-text APIs with no query
provider and no vector projection configured. It must invoke the direct-text
marker, not hybrid with an absent provider. No production algorithm, environment
hook or observability API change is needed for P.

Selectively adapt these historical sources; retain their refs and changes in
the new committed experiment README:

| Source ref/path | Reuse | Remove or revalidate |
| --- | --- | --- |
|07095a16dad020798bc5125cbde0c909e8c38a78 ac013-scale-search/lme_corpus.py | First-seen unique nonempty turn order and500question order | Add bounded selection/reporting and independent count tests; no old prototype cells or claimed present baseline. |
|07095a16dad020798bc5125cbde0c909e8c38a78 ac013-scale-search/ssweep.py,analyze.py | Repetition/order/raw sample analysis concepts | New explicit cell table; no BASE1024/CS64/ELIG/FTS arms, old globals or historical10% benefit rule. |
|aa30085fcb293d40cf5f34eaa568b3e215339cfc ac013-seed-drain/drain_sweep.py | Bounded run and separate seed/drain accounting concepts for future D | Existing product scans are already bounded; no armlimit prototype or old trace modules copied. |
| Current ac013-page-cache/sweep.py and engine exp_ac013_page_cache.rs | Existing canonical control/index and reader cache API for later diagnostics | Synthetic768-d fixture is not BGE384 or natural-text relevance; no build-flag/cache-size sweep. |
| Current MCP examples/ac013_mcp.rs | Future real stdio framing and per-call IDs | CPU seeding/default-debug log, old queue parsing and unmatched workload are not accepted current evidence. M remains held. |

New committed directory: dev/experiments/0.8.29-slice30-measurements/ with README,
converter, runner, analysis, tests and machine-readable cells. Use the exact Rust measurement example
`src/rust/crates/fathomdb-sdk/examples/slice30_measure.rs`, built with
`cargo build --locked --release -p fathomdb-sdk --example slice30_measure`.
Current SDK already depends on the engine and has serde_json/tempfile dev
dependencies. Add fathomdb-query as a workspace dev dependency and rusqlite0.41 with the
engine-matching bundled/fallible_uint features (existing lock0.41.1) only for
the separate untimed compiler/match-count query; no public SDK API or new
production dependency. All tools are locally built; no downloaded helper or
wholesale historical merge.

## Initial P implementation and measurements

Converter reads the pinned raw file, validates paired session arrays, deduplicates
sessions and stripped nonempty turn texts in original order, emits first100k
JSONL bodies and all500questions, and reports counts. It never reads answer fields
for workload construction. Use synthetic three-question inputs with explicit
expected order for tests; do not regenerate repository goldens.

Runner prebuilds a release executable outside timing. P opens a fresh format37
store per repetition/configuration, uses governed writes and direct-text search,
records real counts and IDs, and closes/exits normally. The driver reports one
sample per search plus actual write/seed/close durations and structured errors.
Separate engine-direct and Rust-SDK adapters on identical input/order. Tiny
real-store tests use literal alpha/beta/marker expected IDs independent of the
runner's data structures. Natural-query result IDs/counts are observed workload
data, not a newly generated relevance oracle.

P uses public freshness/committed write behavior, not private worker freezes.
A dedicated marker query outside latency samples checks every committed batch
against hand-specified current IDs/body versions. Batches replace the same ten
allocated marker records; report their separate count and revision growth.
Each fresh store contains exactly100000 natural records plus10 marker records;
AC083 samples query the natural corpus under an explicit kind filter that
excludes markers, with the unfiltered100000-only read-mostly baseline separate.
Do not quietly call100010 the100k tracked fixture. Filtering is an explicit
write-schedule difference and cannot serve as the read-mostly AC083 result.

Capture read visibility/generation and effective time adjacent to each batch
using available governed/test-only read facilities after GO fit check. Adjacent
observations are labelled adjacent, not proof of a search transaction's exact
snapshot. P can count committed writes/reuse opportunities; it cannot prove a
safe cache key or attribute hybrid scan cost. If no suitable public observation
exists, report that field unavailable and leave40 authority selection held;
do not introduce a production API just to fill the field.

## Deferred cells and observer design boundary

| Group | Actual prerequisite | Scope and current disposition |
| --- | --- | --- |
| P | Reviewed dependency cut, closed10/current source, own tools/input/native artifacts | Provider-free direct-text baseline and write schedules;16cells measured, fresh final P PASS/parent acceptance; immutable evidence. |
| G | P tools qualified; updated concrete G packet/review; own pinned BGE cache, available reserved engaged3090 | Same BGE384 store for engine/SDK hybrid/direct-text/embed-only; read/write schedules and guarded branch attribution. No TC5 dependency. NOT_RUN. |
| D | P tools; reviewed current worker observer/fixture fit and cap feasibility | Synthetic deterministic768-d held/overlap10k/100k/300k drain follow-ups, pending counts, worker concurrency and WAL;300k requires lower-scale extrapolation under15min/16GiB. NOT_RUN. |
| M | Dependable selected G store;20 A3/A5 proof or actual owner disposition; reviewed exact MCP tooling | Real stdio c1/4/8/16/32, actual queue denominator/errors, fresh-process/open/idle/alternating-store costs. NOT_RUN/HELD. No claimed queue proof from timing. |
| F | Exact accepted frozen pack, qualified manifest and candidate-bound CUDA/native/model identities | AC075 TC5 bridge, unchanged accepted CI-high>=0.90; missing exact pack. HELD. |

G diagnostics must separately observe has_source_dependencies,
all_nodes_directly_eligible, vector_arm_requires_fallback and actual guarded
caller counts, including direct/no-dependency, source fallback and time-window
controls. Prefer existing engine-owned test features. If current facilities
cannot expose a required component, propose a minimal engine-owned bounded
feature-gated span observer, exact test-surface/interface note and tests before
source; no process-global unowned tracer or new production perf knobs. Record
branch-not-run separately. Existing public profile placeholders are not measurements.
Do not subtract hybrid/text/embed percentile values or call no-match hybrid pure
vector. Production timing and diagnostic builds retain separate features/native
identities; paired untraced/traced cells bound observer cost.

D retains durable scan/terminal/readiness authority and actual current per-arm
bounds. Existing freeze/pause hooks may be used only in diagnostic held cells;
overlap uses real concurrent writer/worker work, validated pending activity.
No fake provider semantic equivalence to BGE, stale created_at digest, fault
injection or reimplementation of settled armlimit/CS64 fixes.

M wait logs contain only waited admissions. Report waited and all-call counts,
errors/cancellations/timeouts and explicitly unavailable service attribution;
unmatched aggregate logs cannot imply per-call service time. Debug logging
requires a paired overhead control; untraced ordinary calls supply latency.
No deadline/admission/idle defaults change. Windows20 A6 remains separately held.

## Interpretation and completion

Use [preregistration](preregistration.md) for initial cell counts, timing and caps.
P can complete a qualified Linux subset. G/D/M/F remain unsatisfied until their
own actual evidence or explicit owner disposition.40 opportunity selection needs
actual corresponding G/D spans/reuse costs, safe authority proof and prospective
tolerances; P speed alone cannot select a cache.50 may refine text design from
P distributions and hybrid/fusion design from G. Their existing source gates
and whole30 dependency are unchanged. Final local release qualification remains
160-owned and publication/CI remain outside this continuation.

## Actual P preservation method

The private SDK example uses only existing core/SDK direct-text and governed
batch APIs. Its measurement-only recorder appends outside API/write clocks and
flushes every16records per stream plus phase/pass boundaries, without fsync.
Recorded persistence/checkpoint cost and throughput scope are explicit. A stop
between checkpoints may lose<=15tail records; a stop during a pending flush can
leave that checkpoint partial. Strict per-line UTF8 decoding preserves complete
raw prefixes and reports issues without promoting incomplete cells. Resume
updates all prior outcomes by ID, retaining completed trailing attempt identity.
These three reviewed corrections have separate genuine runtime RED/GREEN units.
See [execution](execution.md) and [actual results](results-p.md). No public or
production observer API, cache/ranking/default change or new numerical gate.
