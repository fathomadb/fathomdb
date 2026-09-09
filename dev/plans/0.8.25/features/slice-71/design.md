---
title: 0.8.25 Slice 71 — AC-072 and 71B performance design
status: PASS
design_version: 8
target_release: 0.8.25
depends_on: 60
---

# Slice 71 design

Independent design review passed for v4 after three bounded correction cycles.
The durable verdict is `design-review-cycle3.md`. Version 8 records the focused
AC-072 correction selected from bounded diagnostics; focused review passed.

That verdict covers the original design v4 and retained campaign only. The
owner-approved [71B sub-plan](write-regression-subplan.md) now supersedes the
write protocol, fixed event-count requirement, and verification scope below.
It requires a versioned protocol and independent review before new timings,
and a reviewed concrete correction before implementation. The original closed
manifest/receipt schemas below remain historical contracts, not schemas to
silently extend for 71B. AC-013 read-latency evidence remains unchanged.

## Evidence model

`Slice71ManifestV1` is checked in before measurement and rejects missing or
unknown material fields. Required keys bind schema version, source refs,
build/runtime/host identities, fixture/configuration digests, arm order,
repetitions, thresholds, timeouts, environment limits, and allowed outcomes.
`Slice71ReceiptV1` requires per-cell arm/ref, start/end time, process/build
identity, raw-log digest, metrics, trigger inventory, visibility state,
generation/nonce observations, resource observations, error, and result state.
The state is `passed`, `failed`, `insufficient_samples`, or
`environment_invalid`. Partial cells are retained but never pooled into pass.
For the pre-visibility ingest baseline only, trigger inventory is zero,
visibility state is `not_applicable`, generation/nonce values are null, and
the exact reason is `schema_predates_visibility_state`.

The exact machine contract is frozen by
`experiments/configs/release-0825-slice71-manifest.schema.json` and
`experiments/configs/release-0825-slice71-receipt.schema.json`, both with
`additionalProperties: false` at every object. The manifest's exact top-level
keys are `schema_version`, `candidate_ref`, `baselines`, `arm_order`,
`workloads`, `environment_policy`, `timeouts_s`, and `raw_root`.
`baselines` has exactly `ac013_ref` and `ingest_ref`; `workloads` has exactly
`ac013` and `ingest`. AC-013 has exactly `corpus_n`, `vector_dim`, `samples`,
`treatment`, `repetitions_per_arm`, `p50_budget_ms`, `p99_budget_ms`, and
`max_within_arm_range_percent`. Ingest has exactly `corpus_n`, `batch_size`,
`embedder`, `repetitions_per_arm`, `median_regression_limit_percent`,
`max_within_arm_spread_percent`, `expected_ingest_events`,
`ablation_repetitions`, and `ablation_cells`.

The receipt's exact top-level keys are `schema_version`, `manifest_sha256`,
`candidate_ref`, `started_at`, `finished_at`, `cells`, `classifications`, and
`errors`. Every cell has exactly `workload`, `arm`, `ref`, `ordinal`,
`started_at`, `finished_at`, `process_identity`, `build_identity`,
`host_identity`, `runtime_identity`, `environment_observation`,
`fixture_sha256`, `config_sha256`, `raw_log_sha256`, `result_state`, `error`,
and `metrics`. AC-013 metrics have exactly `n`,
`samples`, `vector_dim`, `seed_write_ms`, `projection_drain_ms`, `p50_ms`, and
`p99_ms`. Ingest metrics have exactly `corpus_rows`, `batch_size`,
`ingest_ack_ms`, `projection_drain_ms`, `row_counts`, `trigger_inventory`,
`visibility_state`, `visibility_reason`, `generation_before`,
`generation_after`, `generation_delta`, `nonce_before`, `nonce_after`,
`database_bytes`, `wal_bytes`, `cpu_seconds`, `peak_rss_bytes`,
`load_average`, `available_memory_percent`, `swap_io_delta`, and
`thermal_throttled`. Enums and nullable fields are closed in the two schemas;
the RED validator tests use those files as the sole shape authority.

`runtime_identity` binds the SQLite runtime and `libsqlite3-sys` version.
`environment_observation` binds start/end one-minute load, available-memory
percentage, swap-in/out deltas, the `k10temp:Tctl` start/end temperature,
derived throttling state against the sealed 90 C ceiling, and competing
build/test process inventories. The validator recomputes the manifest digest,
derives per-cell pass/fail and the total classification from retained metrics,
and lets an invalid environment supersede performance classification.

## AC-013 workload

Use detached source worktrees and separate target directories for branch-point
baseline `4fc1b890a11ebfaa8f11b15823656e856002807a` and the exact Slice 71
candidate. A single checked-in wrapper invokes the exact integration test with
`AGENT_LONG=1`, corpus 10,000, dimension 384, treatment `warm`, 1,000 measured
samples, fixed seed, release mode, one test thread, and no runner retry. The
order is `B,C,C,B,B,C`; seed and drain are outside measured samples.

The first six-cell observation preceded its manifest commit and is retained
under `ac013-exploratory/` only. It cannot satisfy acceptance. The admissible
campaign starts only after the revised manifest, schemas, validator, and
environment-capture wrapper are committed together.

Report every repetition and per-arm median p50/p99. Each repetition is judged
independently; all three candidate repetitions must satisfy p50 <= 80 ms and
p99 <= 300 ms. An arm with mixed pass/fail results or a greater-than-20% range
relative to its median p50 or p99 is invalid. Uniform arms map completely to
`both_pass`, `candidate_regression`, `candidate_recovery`, or
`pre_existing_gate_failure`. Tracking-scale and recall results cannot
substitute for this fixture.

The final campaign does not reuse or rewrite the v1 manifest, which binds the
obsolete `5546585d` candidate. After GREEN is committed, create and commit a
new manifest bound to the final product source, unchanged 10k/384d/1,000-query
workload, `B,C,C,B,B,C` ordering, and environment rules. An append-only
execution binding records the actual checkout, executable and runner digests,
and exact `dev/plans/runs/0.8.25-slice-71/ac072-final/` output path. Validate
the receipt against the manifest while retaining the old campaign byte-for-byte.

If product code changes, the two 71B 10k candidate-only checks use the retained
71B runner and environment rules. The protected `eda95b07` medians become an
additional numeric guard: Scale-02 acknowledgement <= 1,543.539 ms and total
<= 1,548.545 ms; projection-active AC-013 total <= 1,442.198 ms. Scale-02
acknowledgement/total and AC-013 total each require <= 25% spread. AC-013
acknowledgement is reported but remains non-gating because it is the variable
asynchronous work partition. Historical +20% limits also remain in force.

The earlier 71A correction removed the common per-hit barrier post-filter and
its fixed 24-hit test remains a regression contract. The remaining AC-072
cause was different. A bounded six-call diagnostic on the exact 10k/384d
fixture measured 174-197 ms full searches: global eligibility reached 7-8 ms,
vector retrieval reached 17-29 ms, unbounded node FTS reached 96-111 ms, and
quadratic body fusion raised completion to 174-197 ms. A fixed-prefix FTS
ablation reached p50 59 ms but was rejected because deep vector/text overlap
can require text ranks beyond the prefix for exact RRF.

The selected correction preserves the complete text candidate set. When the
same read snapshot proves there are no source dependencies and every canonical
node is directly eligible, the default unfiltered hybrid path reads body, kind,
cursor, and BM25 directly from FTS, stably sorts every match by BM25 then
cursor, and defers stable-ID/source hydration until after final truncation.
Ownerless rows retain their content-derived IDs. Any dependency, unsafe node,
filter, reranking, graph arm, reweighting, explanation, or forced test control
uses the existing joined path. Dependency predicates are omitted only after
the same snapshot proves the dependency table empty.

RRF accumulation becomes linear in the common case through a hashed body index
while retaining exact body comparison on hash collision, vector-first identity,
weights, tie order, duplicates, and all deep-overlap contributions. No candidate
count, arm, eligibility rule, public limit, or AC-072 threshold changes. A
100-query 10k/384d release ablation of this exact path measured p50 70 ms and
p99 77 ms. The RED/GREEN real-database test compares limit-100 output, including
deep text ranks, duplicate bodies, and an ownerless row, against the forced
complete joined control. Existing lifecycle, dependency-closure, rank-stream,
and fusion suites remain the focused blast-radius oracles.

## Ingest workload

The following records the original v4 protocol. New execution follows 71B,
including its three initial trigger arms (no nonce-only arm), foreground/drain
accounting, total-completion criteria, and separately bound projection-active
fixture. Do not execute this historical protocol as the current sub-plan.

Use Slice 30 parent `b2bfb1f318f58041144acb2356a6a4c9624068b9`
as baseline and the exact Slice 71 candidate, with the existing Scale-02 10k
fixture, batch size 256, no embedder, fresh database, isolated process, and
order `B,C,C,B,B,C`. Record acknowledgement and drain separately alongside
rows, errors, schema trigger inventory, generation before/after/delta, nonce,
database/WAL bytes, CPU, RSS, host pressure, source/package hashes, and raw-
result digest.

If candidate median acknowledgement is more than 20% slower, run three fresh-
database repetitions for each current-candidate attribution cell after the
sealed production comparison. The production cell leaves the schema triggers
byte-for-byte untouched. Counter-only preserves the exact exhaustion-checked
`CASE WHEN generation=9223372036854775807 THEN RAISE(ABORT,...) ELSE
generation+1 END` and removes only nonce rotation. Nonce-only leaves generation
unchanged and writes a fresh `lower(hex(randomblob(32)))` nonce. No-op bodies
execute only `SELECT 1`. For altered cells, keep one no-embedder Engine open
after it creates and validates the production schema. Before timed writes, a
bounded side connection replaces the trigger bodies, verifies the complete
replacement inventory, and closes. Timed writes use the already-open Engine;
the database is not reopened after alteration. These cells are diagnostic and
can never satisfy product acceptance; reopening an altered database is expected
to fail the production trigger-manifest check.

The primary ingest statistic is median acknowledgement time. Every arm must
have max/min acknowledgement spread no greater than 25%. A cell is invalid if
the load average at start or end exceeds half the online CPU count, available
memory is below 25%, swap-in/out increases, thermal throttling is reported, or
another build/test/Scale-02 process is active. Invalid or disagreeing cells
stop without retry or exclusion after results are visible.

The original design required three foreground changes per simple node and 54
schema-33 triggers. Under 71B, these implementation counts may change only
through a reviewed design with coordinated migration/open-manifest updates.
Fresh branch-sensitive nonce behavior, rollback safety, cross-process and
raw-SQL drift detection, virtual-table coupling, exhaustion, and authoritative
coverage remain required guarantees.

## Historical v4 TDD and verification

The original v4 RED commit contains exact-runner selection/exit-status tests,
strict manifest/receipt rejection tests, and the deterministic query-profile
oracle.
GREEN changes only code directly supported by those failures. Focused
verification includes the changed tests, relevant dependency-closure and
Slice 35 trigger tests, workspace clippy, workspace check, both sealed
campaigns, independent code review, and a separate verification reviewer.
This paragraph records the original verification scope only. The AC-072
completion section below governs current read work, and 71B's affected-crate
policy governed the completed write work. Full release regressions remain
deferred to Slice 75 by owner direction.

## 71B implemented write design

The accepted implementation keeps the 54 persistent schema triggers as the
external-SQL contract. Engine-owned canonical and projection transactions may
disable triggers only on their own connection after confirming that no custom
main- or TEMP-schema trigger targets the transaction's tables. They perform one
checked random-nonce visibility advance after successful mutations and restore
trigger execution before commit; any custom trigger selects the original row-
trigger path, and a restoration failure poisons the connection.

The governed foreground statements and projection validation statements use
the connection cache. Values invariant under one IMMEDIATE transaction
(generation and eligibility instant) are read once. Enrolled rows skip the
logically redundant declaration and absent-dependency probes. `drain` uses the
writer connection for its final durable pending-work check instead of opening
a duplicate connection. The two projection workers retain bounded batching,
with 64 outcomes per commit and an in-flight limit of 128.

This changes no schema, public API, visibility guarantee, exhaustion behavior,
or external trigger coverage. Exact results and focused proof are recorded in
[71b-performance-recovery.md](71b-performance-recovery.md).

## AC-072 completion design

### Isolation from 71B

The read-latency correction starts from the completed 71B product candidate
`eda95b07`. The governed write, visibility, projection-commit, batching, and
drain changes are protected inputs, not new optimization space. AC-072 work
must not modify them unless diagnostic evidence makes such a dependency
unavoidable. Any such overlap requires an explicit blast-radius account and
the two candidate-side 71B 10k checks before Slice 71 can close.

### Equivalent execution basis

Before changing product code, produce one compact equivalence record for the
historical synthetic 10k run, branch-point baseline, retained Slice 71
candidate, and prospective candidate. It binds effective corpus/vector-row
counts, dimension, deterministic corpus/query seeds, embedder, candidate
fanout, warmup and measured-query counts, public operation, build/features,
SQLite/sqlite-vec identities, CPU identity/instructions, affinity, experimental
environment variables, background projection state, and host controls.

The historical real-embedding 36/49 ms result used 7,667 rows and remains a
separate production reference. The historical synthetic 15/17 ms result used
10,000 rows at 384 dimensions and is the closer diagnostic comparison. The
current fixture's default is 768 dimensions; the executed Slice 71 record says
384, so the equivalence record must inspect the effective setting instead of
presuming either configuration.

`ac_013_vector_retrieval_latency` measures full `Engine.search`. Acceptance
therefore includes query embedding, reader dispatch, vector retrieval,
canonical hydration, FTS retrieval, eligibility, ranking, and fusion. A
vector-only probe is diagnostic only. The test's `ac_013` name is retained for
compatibility; the binding contract it evaluates is AC-072.

### Bounded diagnostic seam

Add opt-in `test-hooks` instrumentation at existing execution boundaries. It
may record elapsed component time, normalized statement identity, execution
count, rows/results consumed, preparation count, and SQLite statement counters
such as VM steps, full-scan steps, and sorts when the runtime exposes them.
Capture `EXPLAIN QUERY PLAN` for the exact statements and effective parameters.
Do not enable this instrumentation in the acceptance campaign or production
builds.

Decompose one representative slow full search in this order:

1. query embedding and serialization;
2. reader dispatch and queue/lock wait;
3. vector eligibility and fallback classification;
4. binary KNN candidate scan and float reranking;
5. canonical hydration;
6. FTS retrieval, lifecycle/dependency eligibility, and ranking; and
7. fusion and final materialization.

The decomposition stops when it identifies the dominant component and enough
mechanism evidence to write a deterministic RED test. It is not a second
performance campaign.

### Initial SQL blast radius

Inspect `vector_arm_requires_fallback` first. Its negative `EXISTS` result on
an entirely eligible corpus may require examining all vector rows through
canonical-state joins. Confirm or reject that hypothesis with its real plan,
statement counters, and elapsed contribution.

Inspect the hybrid FTS arm next. Full `Engine.search` does not automatically
use the streaming optimization restricted to direct text-only search. Measure
matched rows, eligibility placement, ranking work, cap placement, and fusion.
Inspect hydration against the actual schema and query plan; do not rely on
comments that predate cursor indexes now created by migrations.

The direct blast radius includes search dispatch, dependency/lifecycle
eligibility, vector candidate production and reranking, FTS query generation,
hydration, fusion, relevant schema indexes, and their focused tests. It excludes
SDKs, publication, unrelated writers, CUDA, Windows, and release-wide gates
unless the selected correction actually crosses one of those boundaries.

### Correction constraints

Choose the smallest measured correction: prepared-statement reuse, a
semantics-equivalent SQL rewrite, a justified index, removal of a redundant
same-snapshot check, an FTS execution improvement, or correction of proved
runner/runtime drift. Preserve eligibility before truncation, lifecycle and
dependency semantics, stable ranking/fusion, and one read snapshot.

Eligibility cannot be cached solely by generation because validity windows can
change with time and external processes can mutate the database. Any cache must
bind effective time, snapshot/read-view identity, and cross-process visibility.
Do not reduce candidate counts, disable an arm, substitute vector-only search,
skip eligibility, or change AC-072 thresholds.

### RED/GREEN and evidence

The RED test targets the measured mechanism rather than elapsed wall time. Good
oracles include a repeated-statement count, a stable scan/plan property, a
proved redundant-check count, or a real-database eligibility result that the
old query violates. GREEN preserves the oracle and existing affected search,
dependency, lifecycle, frozen-read, ranking, and pagination tests.

After focused correctness tests and affected-crate check/clippy, run the exact
release-mode AC-072 campaign: 10,000 rows, 384 dimensions, 1,000 measured
queries after warmup, three repetitions per arm in `B,C,C,B,B,C` order, with
the registered environment and spread rules. All valid candidate repetitions
must satisfy p50 <= 80 ms and p99 <= 300 ms.

If product code changed, run three candidate repetitions for only the Scale-02
10k and projection-active 10k 71B workloads. Compare their medians with both
the protected `eda95b07` results and the unchanged historical limits. Do not
rerun historical write baselines. No product-code change means Git source-drift
proof is sufficient and no write timing rerun is needed.

Full regressions, Windows, CUDA, installed cross-SDK, hosted CI, and publication
remain outside Slice 71. Slice 75 consumes these focused receipts and owns the
integrated candidate-side release matrix.
