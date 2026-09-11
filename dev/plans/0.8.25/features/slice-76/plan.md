---
title: Slice 76 — AC-020 attribution and statement reuse
status: DRAFT
depends_on: 75
---

# Slice 76 — attribution and statement reuse

## Outcome

Determine the current AC-020 bottleneck with statistics enabled and test the
lowest-risk allocation-reduction hypothesis. Deliver enough evidence for
Slice 77 to select its experiments without rediscovering the baseline.
No shipping correction or package redesign lands in this slice.

The [shared protocol](../../ac020-experiment-protocol.md) is the experiment
design of record. Read it in full; all identities, counter semantics,
correctness requirements, verdict rules and stop conditions apply here.

## Requirements and falsifiable outputs

| ID | Requirement | Acceptance evidence |
| --- | --- | --- |
| R76-1 | Protect the release checkpoint and existing receipts | Source/fixture/executor manifest, clean baseline and Slice 75 cell inventory |
| R76-2 | Establish current scaling | Seven baseline observations with real test counts, raw logs, statistics enabled and dispersion |
| R76-3 | Attribute fixed-cost and contention sources | SQL census, allocations/search, page traffic, lookaside and dispatch measurements; known/unknown attribution separated |
| R76-4 | Test statement reuse independently | Matched B/C series and focused RED/GREEN evidence, or explicit documented infeasibility |
| R76-5 | Supply Slice 77's decision inputs | Reviewed ranked hypotheses, candidate eligibility, memory/correctness risks and exact remaining questions |

A negative or inconclusive experimental result can satisfy this slice.
A missing measurement cannot be renamed a negative result. Environment-blocked
work records its blocker and does not become a completed attribution.

## Entry and checkpoint inventory

1. Verify owner reallocation and Slice 75 checkpoint closure at 5056db9e.
   No ongoing writer may share the checkout.
2. Read the imported research and reconcile its main-branch grounding with
   current release code. In particular, verify uncached search sites, existing
   deferred identity hydration, stable SQL shapes and reader cache capacity.
3. Create a cell-by-cell Slice 75 carry-forward inventory from its manifest:
   original source, result location/digest, actual execution/counts, status
   (verified pass/fail, unavailable, missing, or unrun), and Slice 85 owner.
   Bounded read-only recovery: at most 30 minutes in documented run/artifact
   roots; ask the prior agent/owner for missing receipts rather than broad
   filesystem searches or rerunning suites.
4. Seal the benchmark executor and source/build manifest. Resolve exact
   protected AC-072/71B commands from retained receipts for Slice 77.
5. Independent protocol review precedes prototype work. Review may narrow
   measurements but cannot lower the oracle or add candidates silently.

## Hypotheses

- H76-A: repeated preparation/finalization materially increases allocation
  traffic and concurrent contention; caching lowers measured prepare calls
  and concurrent elapsed time.
- H76-B: temporary b-tree/pager churn is a material subset; caching may
  reduce it. This is independent of H76-A and requires measured lifecycle
  evidence, not an inference from one SQLite opcode.
- H76-C: another source (extension scratch, WAL/VFS, allocator or dispatch)
  dominates; caching cannot recover enough performance alone.

Report each as supported, refuted, unresolved or not measurable with the
available instrument. Historical PCACHE2/MEMSTATUS wins inform these hypotheses
but are neither the control arm nor newly authorized runtime treatments.

## Execution phases

### Phase A — current baseline and instrumentation

Run seven uninstrumented baseline processes using the shared exact command.
If the baseline passes all seven, do not immediately optimize: verify source,
statistics and registered executor, then give Slice 77 a confirmation-only
route. Do not assume a different CI host will also pass.

At most six initial diagnostic executions (up to two for each group):

1. Allocator/page-cache calls and stacks; distinguish prepare, execution,
   parsing, FTS5/vec scratch, and net/peak retained memory.
2. Contention/CPU/blocked-time classification with symbol visibility and
   unattributed share disclosed; collect per-arm lookaside hit/miss counters.
3. SQL/EQP/opcode census and dispatch queue/service/response waits, with
   busy-worker/idle-worker overlap.

Measurements must cover both sequential and concurrent arms where the
instrument supports it. Establish actual calls/search and counts by reader;
aggregate metrics alone must not hide an idle or disproportionately busy worker.
Collector/unit tests precede instrumentation changes. Diagnostics cannot change
request scheduling or SQLite configuration in verdict binaries.

### Phase B — isolated statement-reuse prototype

B is unchanged safe baseline. C differs only in connection-owned search
statement reuse and its explicitly selected capacity. Keep SQL, bindings,
query ordering, snapshot scope and result collection unchanged.

Set one capacity from the measured simultaneous working set, not a guessed
minimum of 32. Record unique SQL across shapes, lifetime overlap, memory per
connection and aggregate across eight workers. If SQL contains request-varying
literals, parameterize only where semantics are straightforward and independently
tested; otherwise flag a separate experiment rather than widening C.

RED tests must prove fresh bindings/results on alternating requests,
error-path release, schema invalidation, transaction release and current/frozen
visibility. Existing dependency/eligibility/ranking tests remain fixed.
Use property tests for any new reusable binding/round-trip machinery.

Run seven matched control and seven treatment processes in the predeclared
alternating order. No second cache-size tuning sweep. Count preparations in
separate diagnostics: after warm-up, fresh preparations should scale with
distinct resident SQL shapes rather than request count; legitimate schema
reprepare and evictions are recorded, not hidden.

Reserve up to three post-C diagnostic executions to measure changes in
allocation/page calls and identify the residual. Ephemeral reuse may be refuted
while ordinary prepare reuse still succeeds.

### Phase C — interpretation and handoff

Compare absolute sequential/concurrent times, speedup distribution, preparation
and allocation deltas, memory retention and semantic tests. Apply the shared
selection rules. If C is fast but violates semantics, it is not eligible.

Write a Slice 77 decision table:

| Observation | Slice 77 route |
| --- | --- |
| Baseline or C passes all seven with valid evidence | Confirmation/robustness route; no obligatory extra optimization |
| Parsing/binding allocations remain material | BLOB-first route, with optional separately tested quantization |
| Work waits behind busy workers while another is idle | Dispatch route can compete for a bounded treatment slot |
| Lookaside misses materially reach the contested allocator | Counter-selected lookaside route |
| Other statement/hydration work dominates | One tightly scoped SQL/copy route, prospectively reviewed |
| Mechanism uncertain or environment invalid | Stop; resolve evidence or propose reserved Slice 78, not implementation by guess |

## Same-file safety census

Read current Rust migration, Python/Node embedding and raw-SQL usage contracts.
Determine whether same-process, different-SQLite copies may open the same
database with overlapping connection lifetimes. Distinguish separate files,
separate processes, offline migration, and concurrent same-file access.
Produce a usage/contract inventory with known VFS/platform assumptions.
Do not claim demonstrated corruption or silently prohibit an existing supported
API. Any material gap becomes an explicit Slice 80 consultation input, or a
separate urgent bounded correction proposal if current correctness is at risk.

## Budget, exclusions and completion

- Timing: 7 initial B + 14 matched B/C = 21 normal runs; cap 28 including the
  single allowed invalid-environment restart. Baseline-green route uses fewer.
- Diagnostics: maximum 9 total; no broad tests, hosted dispatch, package builds,
  global configuration ablation or new model campaign.
- Focused check/clippy/test selectors and expected positive counts are frozen
  in the manifest after code census, before execution. Do not invent selectors;
  use cargo --list to verify exact existing names without running broad bodies.
- Four-hour active-work checkpoint; stop/rethink on repeated failure.
- Outputs under dev/plans/runs/0.8.25-slice-76/: manifest, raw observations,
  allocation/SQL/dispatch evidence, result, review, Slice 75 inventory, and
  explicit Slice 77 recommendation. Status links exact source and receipts.
- Independent code review covers any diagnostic/prototype code; a separate
  evidence review checks attribution, all observations and selection claims.
  No review pass is claimed merely because the plan was written.
- Close only after R76-1 through R76-5 are discharged and evidence is durable.
  Leave product baseline unchanged; record prototype SHAs and rebuild commands.
