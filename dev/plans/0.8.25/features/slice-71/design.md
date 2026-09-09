---
title: 0.8.25 Slice 71 — investigation design
status: APPROVED
design_version: 4
target_release: 0.8.25
depends_on: 60
---

# Slice 71 design

Independent design review passed after three bounded correction cycles. The
durable verdict is `design-review-cycle3.md`.

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

Before a code correction, an opt-in `test-hooks` trace on the search reader
records normalized statement identities at their actual execution sites. It
does not use `ProfileRecord` or the timing-gated slow-statement callback. A
fixed 24-hit node-FTS fixture must observe exactly 24
`post_filter_source_lookup` events before GREEN and zero afterward. Removal of
the common post-filter is allowed only after eligibility is present at every
replacement site and real-database tests prove barred vector-node, node-FTS,
vector-edge, and edge-FTS hits remain excluded before truncation.

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

## TDD and focused verification

The RED commit contains exact-runner selection/exit-status tests, strict
manifest/receipt rejection tests, and the deterministic query-profile oracle.
GREEN changes only code directly supported by those failures. Focused
verification includes the changed tests, relevant dependency-closure and
Slice 35 trigger tests, workspace clippy, workspace check, both sealed
campaigns, independent code review, and a separate verification reviewer.
This paragraph records the original verification scope; 71B's affected-crate
and focused-test policy governs new write work. Full release regressions
remain deferred to Slice 75 by owner direction.

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
