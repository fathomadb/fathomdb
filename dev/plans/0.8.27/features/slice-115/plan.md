---
title: FathomDB 0.8.27 Slice 115 — engine characterization plan
status: PLANNED
target_release: 0.8.27
planning_baseline: 1a6cd4938
---

# Slice 115 — engine performance characterization

This is a planning draft. Slice 114 must close before execution. The final
protocol and candidate are frozen at entry, before observing results.

## Changes since the September 28 draft

| Completed or allocated work | Decision for Slice 115 |
| --- | --- |
| Slice 90 qualified the `2/5` engine runtime with named release gates, a six-repetition D27 mixed workload and retained raw receipts; an environment-valid failed campaign and a small owner-accepted throughput shortfall are also recorded. | Reuse the established harness and disclose both PASS and adverse observations. Do not treat characterization as the first D27 proof or silently replace its decision rule. |
| Slice 90 decomposed engine owners and added dispatch/queue/resource observations. Slice 103 altered SQLite/WAL behavior and dependencies. | Profile the current exact candidate and name the measured owner; compare prior receipts only where workload, host, build, features and metric semantics actually match. |
| Slice 110 preserved the native `spawn_blocking` handoff while changing NAPI ownership and subscriber delivery; its Jetson GPU row remains open. | Engine CPU measurements may be designed now; execution waits for Slice 114 and Slice 110 closure. Installed-binding or GPU measurements are separate labeled rows, never evidence for an engine-only run. |
| Existing performance gates, `scripts/d27-runtime-runner.py`, `scripts/run-ac011-write-throughput.sh`, `scripts/perf-experiments/`, and the append-only `dev/perf-history/` already have distinct contracts. | Prefer those runners and fixtures. Do not change gate thresholds, write historical baseline files, or create a general benchmark framework for this slice. |
| Slice 135 owns the published 0.8.26 comparison after Slice 132. | Produce a reproducible current-candidate method and data bundle for 135; do not claim a full cross-release verdict here. |

The draft's broad path list is **approved as a coverage target**, with one
small representative cell per path rather than a combinatorial hardware,
feature and configuration matrix. The accepted default configuration is the
primary cell. A second cell is justified only by a concrete audit finding or
existing gate. Keep real-model CPU/GPU cost separate from engine scheduling,
SQLite and retrieval cost.

## Requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-115A | The protocol is reproducible and fixed before measurement. | AC27-115A: the protocol records exact candidate SHA, binary/features/profile, host and software, dataset/seed, input size and shape, default settings, warm-up, repetition/order, cache and concurrency policy, measured region, metric definitions, profiler method, environmental invalidators and decision rule. Runner and corpus hashes bind the raw data. |
| R27-115B | Representative engine paths have valid, interpretable measurements. | AC27-115B: valid receipts cover open/close, canonical write/ingest, dense projection with a deterministic provider, text/vector/hybrid search, graph expansion, evidence resolution, and erasure. Each path reports its relevant latency distribution or throughput plus correctness count; the mixed D27 row retains queue, backlog, provider concurrency and close/resource observations. An unavailable optional real-model or GPU row is explicitly separate. |
| R27-115C | Lightweight profiles identify cost without changing product behavior. | AC27-115C: at least the slowest or most variable two measured paths have a sampled stack/CPU or equivalent low-overhead profile tied to the same candidate and workload. The report distinguishes SQLite, engine, provider and harness time where visible, and states profiler overhead and unresolved attribution. No optimization is inferred from a single sample. |
| R27-115D | Data and disposition can be independently checked. | AC27-115D: raw samples, environment observations, commands/protocol, artifact hashes, summary calculations, invalid attempts and known limits are retained in a durable receipt. Existing named release gates and the accepted D27 contract are reported separately. A comparable breach blocks and receives focused remediation; an exploratory hotspot is a named follow-up for review, not an untested tuning change. Slice 135 can rerun the fixed workloads against 0.8.26 or records why a cell is not comparable. |

These IDs are release-local; `dev/acceptance.md` remains locked. Do not add a
new numerical release threshold from exploratory data. Existing accepted
thresholds keep their own authority.

## Design and execution method

The measurement unit is one exact-source, exact-protocol cell with raw
observations. This prevents a fast result from a different cache, provider,
profile or host from being compared as though it measured the same engine.
Use a deterministic provider for scheduler/projection attribution; label real
provider runs separately because inference and model loading dominate different
parts of the path. Prefer existing engine integration fixtures and D27 runner;
add only the minimal missing measurement adapter, with tests for parsing,
invalid-run rejection and summary calculations before using it.

1. Reconcile Slice 114's final setting inventory and current engine source.
   Register the protocol, path cells, correctness checks and comparisons
   before the first candidate run. Keep an exact historical D27 rerun only if
   needed to answer a same-protocol drift question.
2. For a new adapter, create RED fixtures for malformed/missing samples,
   mismatched candidate/protocol and invalid environment, then implement GREEN.
   Human-authored correctness assertions use a real database. Measurement-only
   scripts must not change product paths or snapshots.
3. Run cells on a quiet capable host with controlled storage and sufficient
   disk. Retain every attempt, including invalid ones; repeat invalid cells
   only after identifying the invalidator. Profile the two selected paths.
4. Recompute summaries from raw output, review the evidence independently,
   run scoped checks for documentation/data or the full repository gate if
   source/executable scripts change, then record status and the Slice 135
   handoff. A true accepted-gate regression requires focused RED/GREEN repair
   and remeasurement on the corrected candidate.

No cross-release speedup claim, speculative knob sweep or automatic tuning is
part of this slice.
