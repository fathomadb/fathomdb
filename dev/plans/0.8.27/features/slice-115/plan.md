---
title: FathomDB 0.8.27 Slice 115 — engine characterization plan
status: APPROVED_FOR_EXECUTION
target_release: 0.8.27
planning_baseline: 1a6cd4938
---

# Slice 115 — engine performance characterization

This plan is approved for execution on the local release branch. Slice 114
closed under the HITL's 2026-10-04 sequencing exception. The HITL explicitly
authorized Slice 115 to proceed while Slice 110's Tegra row remains open; this
does not close or qualify that row. Freeze the protocol and candidate before
observing measurements. The [design](design.md) owns cell details.

## Changes since the September 28 draft

| Completed or allocated work | Decision for Slice 115 |
| --- | --- |
| Slice 90 qualified the `2/5` engine runtime with named release gates, a six-repetition D27 mixed workload and retained raw receipts; an environment-valid failed campaign and a small owner-accepted throughput shortfall are also recorded. | Reuse the established harness and disclose both PASS and adverse observations. Do not treat characterization as the first D27 proof or silently replace its decision rule. |
| Slice 90 decomposed engine owners and added dispatch/queue/resource observations. Slice 103 altered SQLite/WAL behavior and dependencies. | Profile the current exact candidate and name the measured owner; compare prior receipts only where workload, host, build, features and metric semantics actually match. |
| Slice 110 preserved the native `spawn_blocking` handoff while changing NAPI ownership and subscriber delivery; its Jetson GPU row remains open. | The 2026-10-04 HITL instruction authorizes this engine-only slice now. Installed-binding or GPU measurements are separate labeled rows, never evidence for an engine-only run; Slice 110 remains open. |
| Slice 114 completed a 152-declaration census, verified the five public settings/defaults and corrected stale SQLite guidance without changing behavior. It assigned the unused graph-evidence `_material` construction to this slice for cost investigation. | Use the unchanged default `2/5/30,000/1,000,000/100` primary cell. Profile graph-evidence resolution and identify whether `_material` costs enough to merit separate remediation; do not silently remove it in a measurement slice. |
| Slice 114's outcome and Slice 110's current Tegra blocker postdate the original September 28 baseline; Slice 90's final candidate gate and Slice 103 WAL changes also postdate the first draft. | Bind all new measurements to the current source and report historical results as historical. No earlier receipt qualifies this candidate. |
| Existing performance gates, `scripts/d27-runtime-runner.py`, `scripts/run-ac011-write-throughput.sh`, `scripts/perf-experiments/`, and the append-only `dev/perf-history/` already have distinct contracts. | Prefer those runners and fixtures. Do not change gate thresholds, write historical baseline files, or create a general benchmark framework for this slice. |
| Slice 135 owns the published 0.8.26 comparison after Slice 132. | Produce a reproducible current-candidate method and data bundle for 135; do not claim a full cross-release verdict here. |

The draft's broad path list is **approved as a coverage target**, with one
small representative cell per path rather than a combinatorial hardware,
feature and configuration matrix. The accepted default configuration is the
primary cell. Include one real default-embedder cell alongside deterministic
provider projection so inference cost is visible. Further cells need a
concrete audit finding or existing gate. Keep real-model cost separate from
engine scheduling, SQLite and retrieval cost; GPU is a separately labeled
optional platform cell.

## Requirements and acceptance

| ID | Requirement | Falsifiable acceptance |
| --- | --- | --- |
| R27-115A | The protocol is reproducible and fixed before measurement. | AC27-115A: the protocol records exact candidate SHA, binary/features/profile, host and software, dataset/seed, input size and shape, default settings, warm-up, repetition/order, cache and concurrency policy, measured region, metric definitions, profiler method, environmental invalidators and decision rule. Runner and corpus hashes bind the raw data. |
| R27-115B | Representative engine paths have valid, interpretable measurements. | AC27-115B: valid receipts cover open/close, canonical write/ingest, dense projection with a deterministic provider, a small targeted real default-embedder CPU repeatability/compatibility cell, text/vector/hybrid search, graph expansion, evidence resolution, and erasure. Each path reports its relevant latency distribution or throughput plus correctness count. The historical D27 mixed receipt retains queue, backlog, provider concurrency and close/resource observations; a current rerun is required only for a concrete drift question. Any substantial real-model workload runs on an available 3090 and is separately labeled. |
| R27-115C | Lightweight profiles identify cost without changing product behavior. | AC27-115C: every measured path has a low-overhead, nontrivial timing breakdown or sampled profile tied to the same candidate and workload. A selection rule fixed before measurement chooses at least two paths for deeper stack/CPU sampling, using a reviewed available profiler. The report distinguishes SQLite, engine, provider and harness time where visible, and states profiler overhead and unresolved attribution. Missing mandatory attribution blocks completion; no optimization is inferred from a single sample. |
| R27-115D | Data and disposition can be independently checked and acted on. | AC27-115D: raw samples, environment observations, commands/protocol, artifact hashes, summary calculations, invalid attempts and known limits are retained in a durable receipt. Every path receives a supported conclusion: accepted gate met, comparable breach needing remediation, exploratory hotspot with a named owner, characterized with no actionable finding, or inconclusive with a stated missing control. Existing named release gates and the accepted D27 contract are reported separately. A comparable breach blocks and receives focused remediation; an exploratory hotspot is a named follow-up for review, not an untested tuning change. Slice 135 can rerun the fixed workloads against 0.8.26 or records why a cell is not comparable. |

These IDs are release-local; `dev/acceptance.md` remains locked. Do not add a
new numerical release threshold from exploratory data. Existing accepted
thresholds keep their own authority.

## Design and execution method

The measurement unit is one exact-source, exact-protocol cell with raw
observations. This prevents a fast result from a different cache, provider,
profile or host from being compared as though it measured the same engine.
Use a deterministic provider for scheduler/projection attribution; measure the
real default provider in a separate required cell because inference and model
loading dominate different parts of the path. Prefer existing engine
integration fixtures and D27 runner;
add only the minimal missing measurement adapter, with tests for parsing,
invalid-run rejection and summary calculations before using it.

1. Reconcile Slice 114's final setting inventory and current engine source.
   Register the protocol, path cells, correctness checks and comparisons
   before the first candidate run, including the rule for selecting deeper
   profiles. Keep an exact historical D27 rerun only if needed to answer a
   same-protocol drift question.
2. For a new adapter, create RED fixtures for malformed/missing samples,
   mismatched candidate/protocol and invalid environment, then implement GREEN.
   Human-authored correctness assertions use a real database. Measurement-only
   scripts must not change product paths or snapshots.
3. Run cells on a quiet capable host with controlled storage and sufficient
   disk. Retain every attempt, including invalid ones; repeat invalid cells
   only after identifying the invalidator. Record low-overhead attribution for
   every path and deeper profiles for the selected paths.
4. Recompute summaries from raw output, review the evidence independently,
   run scoped checks for documentation/data or the full repository gate if
   source/executable scripts change, then record status and the Slice 135
   handoff. A true accepted-gate regression requires focused RED/GREEN repair
   and remeasurement on the corrected candidate.

Design changes receive a first independent `gpt-6.1-sol` high review, with
any further design review by `gpt-6-sol` high. Implementation follows
RED/GREEN; independent `gpt-6-sol` high code review and Terra verification
check the exact candidate. See the design and status for the bounded tests,
review findings and observed results.

No cross-release speedup claim, speculative knob sweep or automatic tuning is
part of this slice.
