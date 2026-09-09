---
title: Slice 71B — write attribution protocol
status: SEALED_PENDING_INDEPENDENT_REVIEW
date: 2026-09-08
---

# Slice 71B write attribution protocol

## Decision boundary

This protocol executes Phase 2 of the approved
[write-regression sub-plan](write-regression-subplan.md). It is prospective:
no result produced before the manifest, runner, probe, schemas, validator, and
this protocol are committed and independently reviewed is an admissible cell.
The earlier probe build and two one-record executions are harness smoke only.

The campaign answers which parts of the schema-31/33 visibility trigger work
account for the already-established write regression. It does not rerun
AC-072, collect search latency, broaden Slice 71 verification, or compare a
shipping correction. A separate prospective recovery manifest will bind the
historical baselines, unchanged current code, and exact fixed candidate after
a correction is selected and reviewed.

## Bound artifacts

The executable contract is
`experiments/configs/release-0825-slice71b-attribution-manifest.v1.json`.
Its custom validator is `experiments/release_0825_slice71b.py`; its JSON Schemas
are the adjacent `release-0825-slice71b-attribution-*.schema.json` files.
The cell runner and external Rust probe are digest-bound by the manifest.

The external probe is compiled in a temporary crate whose path dependencies
point at a clean checkout of the exact product source. This keeps the probe
byte-identical while avoiding product-tree edits. Cargo runs offline and uses
an external target directory. Builds finish before environment observation and
are outside the timed region. Every timed cell uses a fresh database and a
fresh process.

The Scale-02 input is the exact 10,000-row output of the retained frozen fixture
and `build_rows` growth policy, materialized as canonical JSONL outside the
checkout and bound by both input SHA-256 and the original fixture digest. The
AC-013 input uses the exact retained seed, Zipfian body generator, 384-d dense
varying embedder, vector-kind configuration, and batch size. Its generator and
derived fixture identity are bound by the manifest. Timings from the two
fixtures are never pooled.

## Arms and order

Each fixture runs these current-source diagnostic arms:

1. `production`: the validated schema-33 trigger inventory and bodies remain
   unchanged.
2. `generation_only`: after a successful production open, every retained
   visibility trigger is replaced in one diagnostic transaction by the same
   checked monotonic generation update with nonce rotation removed.
3. `no_op`: after a successful production open, every retained visibility
   trigger is replaced in one diagnostic transaction by a `SELECT 1` body.

Names, target tables, operations, and inventory count remain unchanged in the
two counterfactuals. Alteration happens only after production open validation.
SQLite schema invalidation applies the replacement to already-open writer and
projector connections. The AC-013 arm is the qualification that the seam works
with projection-worker connections: its generation observations must reflect
foreground and drain phases as specified by each treatment. Any missing
trigger, unexpected suffix, nonce-less production body, DDL error, write/drain
error, or inconsistent observation invalidates the cell rather than falling
back.

The preregistered order for each fixture is:

```text
production, generation_only, no_op,
no_op, generation_only, production,
generation_only, production, no_op
```

This gives three repetitions per arm with position balance. Execute Scale-02
and AC-013 as separate campaigns. Do not reorder, replace, discard, or silently
retry a cell. A failed or invalid cell is retained; any replacement requires a
new prospective amendment.

## Measurements and interpretation

The timed endpoints are:

- acknowledgement: immediately before the first `Engine.write` through return
  of the final batch write;
- drain: immediately before `Engine.drain` through its return;
- total: the exact sum of those two non-overlapping measured intervals.

The probe records configured rows, batch size, actual transaction count,
visibility generation and nonce at setup/acknowledgement/drain boundaries,
trigger inventory, process CPU, peak RSS, database bytes, and WAL bytes.
The generation observations count trigger effects in production and
generation-only arms. They do not count no-op executions. SQLite exposes no
stable per-connection preparation, reprepare, or lock-wait counter through the
public product surface used by this external probe; those quantities are
therefore not fabricated. Extend to the sealed preparation factorial only if
the three arms do not identify a correction target.

The primary comparisons are per-fixture median acknowledgement and total time:

- production versus generation-only isolates incremental nonce work;
- generation-only versus no-op isolates the checked singleton generation
  update, including its interaction with the existing writer/projector;
- production versus no-op bounds the combined per-row visibility body cost
  while retaining trigger dispatch.

Do not label residual production/no-op time as contention without a direct
lock-wait observation. Do not interpret acknowledgement/drain redistribution
as recovery unless total also changes. The synthetic recovered benchmark and
the one-run diagnostic remain non-acceptance context only.

## Environment, stop rules, and retention

The runner samples one-minute load, online CPUs, available memory, swap I/O,
`k10temp:Tctl`, and competing build/test/benchmark processes immediately before
and after each timed process. A cell is invalid if load exceeds half the online
CPU count, available memory falls below 25%, swap changes, temperature exceeds
90 C, the required thermal signal is absent, or a competing process exists.

For acknowledgement and total separately, an arm is invalid when
`(max / min - 1) * 100 > 25`. Timings must be finite and positive. Any invalid
arm stops that fixture campaign after retaining every cell already collected.
No outlier deletion, threshold adjustment, or same-protocol replacement run is
allowed.

Raw output, environment snapshots, and strict cell JSON are retained under
`dev/plans/runs/0.8.25-slice-71/71b/attribution/`. The aggregate receipt follows
`slice71b-attribution-receipt.v1`, binds the exact manifest, and records a
supported-cause classification. An independent read-only reviewer validates
the retained receipt and logs; the reviewer does not execute another campaign.

## Focused verification accounting

Protocol verification consists only of the new Python contract tests, Python
syntax/lint for the changed runner/validator, JSON/JSON-Schema parsing, a probe
release build against the sealed current source, the two one-record harness
smokes already identified as non-evidence, and independent protocol review.
These are focused checks and consume zero broad-verification rounds. No
`agent-verify`, `scripts/check.sh`, AC-072 search run, full regression, CUDA,
Windows, package matrix, hosted CI, or long stress run is authorized here.
