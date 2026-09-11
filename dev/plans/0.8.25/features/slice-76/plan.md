---
title: Slice 76 — AC-020 attribution and statement reuse
status: APPROVED_FOR_EXECUTION
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

## Draft reconciliation

The draft was checked against the release branch before execution:

1. Since the safe product checkpoint `5056db9e`, `b8a1a228` changes planning
   and evidence records only. Product code, tests, dependencies, packaging and
   workflows are unchanged.
2. Search-statement reuse was tried and reverted in the 0.6.0 Pack 5 E.1
   experiment. It reduced the old sequential arm by 13.7% but did not improve
   its concurrent arm. That experiment preceded F.0's sticky thread-affine
   reader workers; the corresponding G.2 reuse experiment was never run on the
   new topology. The imported research was grounded on main `8b4bc1c6`; its
   phrase "never tried on this path" means the current 0.8.25 path, not the
   project history.
3. A current retest is justified because E.1 preceded the thread-affine reader
   pool, per-reader lookaside, schema 33, current dependency/lifecycle reads,
   Slice 71's deferred text hydration and linear fusion, and the current
   rusqlite/SQLite versions. Its four-statement counter is not reusable as a
   current oracle.
4. The current common AC-020 path contains stable and generated SQL shapes;
   only deferred identity hydration is already cached. Exact executed shapes,
   preparations and capacity are measured rather than copied from either the
   old experiment or the research estimate.
5. Slice 75 carry-forward bookkeeping is limited to classifying the checked-in
   manifest and known receipts. It does not trigger artifact hunting or test
   reruns. Same-file dual-runtime safety remains a bounded contract census, not
   an implementation task in Slice 76.

These findings approve the experiment while narrowing diagnostics to evidence
that can choose Slice 77's next step. A new private SQLite runtime, packaging
change, global SQLite configuration, or shipping optimization is rejected here.

## Requirements and falsifiable outputs

| ID | Requirement | Acceptance evidence |
| --- | --- | --- |
| R76-1 | Protect the release checkpoint and existing receipts | Source/fixture/executor manifest, clean baseline and Slice 75 cell inventory |
| R76-2 | Establish current scaling | Seven baseline observations with real test counts, raw logs, statistics enabled and dispersion |
| R76-3 | Attribute enough of the current cost to choose the next experiment | Exact current SQL/prepare census plus one bounded CPU/wait profile and available per-reader counters; unknown share stays explicit |
| R76-4 | Test statement reuse independently | Matched B/C series and focused RED/GREEN evidence, or explicit documented infeasibility |
| R76-5 | Supply Slice 77's decision inputs | Reviewed ranked hypotheses, candidate eligibility, memory/correctness risks and exact remaining questions |

A negative or inconclusive experimental result can satisfy this slice.
A missing measurement cannot be renamed a negative result. Environment-blocked
work records its blocker and does not become a completed attribution.
If seven valid current baselines all pass, R76-3 and R76-4 are explicitly
`NOT_APPLICABLE_BASELINE_GREEN`: retain the positive statistics witness and SQL
census, skip prototype timing, and route Slice 77 to confirmation. This is a
successful early exit, not infeasibility or a renamed missing measurement.

## Entry and checkpoint inventory

1. Verify owner reallocation and Slice 75 checkpoint closure at 5056db9e.
   No ongoing writer may share the checkout.
2. Read the imported research and reconcile its main-branch grounding with
   current release code. In particular, verify uncached search sites, existing
   deferred identity hydration, stable SQL shapes and reader cache capacity.
3. Create a cell-by-cell Slice 75 carry-forward inventory from its checked-in
   manifest and status: declared receipt, checked-in presence, status (verified,
   missing, unavailable or unrun), and Slice 85 owner. Do not search outside
   documented roots or rerun suites for this inventory.
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

Use at most three initial diagnostic executions:

1. Exact SQL/prepare census and EQP/opcode inspection for the common AC-020
   path, using current code rather than the stale four-statement A.3 harness.
2. One sequential/concurrent CPU and blocked-time profile, classifying visible
   preparation, SQLite/extension, WAL/VFS, allocator and dispatch stacks.
3. Available per-reader lookaside/page/dispatch counters only where an existing
   hook or one small tested diagnostic supplies them. Absence stays explicit;
   it does not authorize a new instrumentation subsystem.

Measurements must cover both sequential and concurrent arms where the
instrument supports it. Establish actual calls/search and counts by reader;
aggregate metrics alone must not hide an idle or disproportionately busy worker.
Collector/unit tests precede instrumentation changes. Diagnostics cannot change
request scheduling or SQLite configuration in verdict binaries.

### Phase B — isolated statement-reuse prototype

B is unchanged safe baseline. C differs only in connection-owned search
statement reuse and its explicitly selected capacity. Keep SQL, bindings,
query ordering, snapshot scope and result collection unchanged.

Set one capacity from each reader's cyclic distinct-SQL reuse distance,
including existing cached statements, not from simultaneous statement count or
a guessed minimum of 32. The current rusqlite default is 16; seal the smallest
observed no-eviction capacity once and record retained memory per connection and
across eight workers. If SQL contains request-varying literals, parameterize
only where semantics are straightforward and independently tested; otherwise
flag a separate experiment rather than widening C.

RED tests target the actual changed mechanism: alternating query bindings and
results, statement/row release before transaction completion, error recovery,
and current/frozen visibility. One schema-change witness confirms cached
statements recover through SQLite's supported reprepare behavior. Existing
dependency/eligibility/ranking tests remain fixed. Property tests are required
only if new reusable binding or codec machinery is introduced; changing
`prepare` to `prepare_cached` with unchanged bindings introduces none.

Run seven matched control and seven treatment processes in the predeclared
alternating order. No second cache-size tuning sweep. Count preparations in
separate diagnostics: after warm-up, fresh preparations should scale with
distinct resident SQL shapes rather than request count; legitimate schema
reprepare and evictions are recorded, not hidden.

Reserve one post-C diagnostic execution to confirm preparation reduction and
identify the visible residual. Ephemeral reuse may be refuted while ordinary
prepare reuse still succeeds.

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

## Implementation and review sequence

1. **RED:** add focused experiment tests for the selected common-path statement
   sites and prove they fail while those sites are uncached. Commit the RED
   witness without changing established acceptance tests.
2. **GREEN:** convert only those measured sites to connection-owned cached
   statements and set the single census-derived capacity. Do not parameterize
   unrelated SQL or change query semantics.
3. Run the focused correctness selectors, matched B/C measurements and one
   residual diagnostic. An independent code reviewer inspects the exact diff.
4. Remove the experimental product/test diff after evidence is retained. The
   final Slice 76 tree keeps only plans, design, status and experiment receipts;
   the prototype commits and commands remain identifiable in history.
5. A separate read-only verifier checks raw logs, counts, digests, statistics
   status, selection math and the clean final relevant-tree identity.

## Budget, exclusions and completion

- Timing: 7 initial B + 14 matched B/C = 21 normal runs; cap 28 including the
  single allowed invalid-environment restart. Baseline-green route uses fewer.
- Diagnostics: maximum 4 total; no broad tests, hosted dispatch, package builds,
  global configuration ablation or new model campaign.
- Focused check/clippy/test selectors and expected positive counts are frozen
  in the manifest after code census, before execution. Do not invent selectors;
  use cargo --list to verify exact existing names without running broad bodies.
- Four-hour active-work checkpoint; stop/rethink on repeated failure.
- Outputs under dev/plans/runs/0.8.25-slice-76/: manifest, raw logs, summary,
  result, review, Slice 75 inventory and explicit Slice 77 recommendation.
  Do not duplicate the same observation into multiple prose records.
- Independent code review covers any diagnostic/prototype code; a separate
  evidence review checks attribution, all observations and selection claims.
  No review pass is claimed merely because the plan was written.
- Close only after R76-1 through R76-5 are discharged and evidence is durable.
  Leave product baseline unchanged; record prototype SHAs and rebuild commands.
