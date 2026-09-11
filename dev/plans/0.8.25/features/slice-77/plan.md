---
title: Slice 77 — adaptive AC-020 experiments and selection
status: DRAFT
depends_on: 76
---

# Slice 77 — adaptive experiments and selection

## Outcome

Use Slice 76 evidence to determine whether a small statistics-enabled
correction can meet unchanged AC-020, and deliver a reviewed decision dossier
for Slice 80 consultation. Do not implement every research suggestion.
Do not start until Slice 76 has closed and this slice's selected treatment
manifest has independent prospective review.

The [shared protocol](../../ac020-experiment-protocol.md) applies in full.

## Requirements and falsifiable outputs

| ID | Requirement | Acceptance evidence |
| --- | --- | --- |
| R77-1 | Selection is informed by Slice 76 | Ranked residual mechanisms and prospectively sealed eligible treatments |
| R77-2 | Effects are attributable | Matched controls, individual treatment series, separate combination confirmation |
| R77-3 | Semantics and prior recovery survive | Focused RED/GREEN, parity/representation tests, exact protected candidate guards |
| R77-4 | Implementation decision is supportable | Ranked candidate, cost/risk/maintenance assessment, exclusions and unresolved questions |
| R77-5 | Failure does not force isolation | Mechanism-specific fallback assessment or bounded reserved-slice proposal |

Negative findings can close the experiment slice if the decision dossier is
complete. They block Slice 80 implementation until further consultation,
not the honest publication of the experiment result.

## Input seal and adaptive design

Retain Slice 76's safe baseline B and its cache candidate C, but select anchor A
explicitly: use C only if correctness passes, concurrent benefit is supported,
and sequential behavior meets the guard; otherwise A=B. A is a non-shipping
experiment anchor, not a release default.

Before changing code, write selection.md and manifest.json with:
measured residuals, eligible treatments, exact source diffs and tests,
parameters, expected mechanism signals, receipt paths and execution order.
No result-informed change to selection thresholds is allowed.

If B or C already passes all seven, select the confirmation-only route:
a fresh seven-run series on the registered executor, focused robustness and
protected guards. Stop if confirmed; no BLOB, quantization or scheduler work
is needed merely to fill a slice.

Otherwise select at most TWO additional treatments from the following.
Each gets one setting by default; a second setting consumes the other slot.
A conditional third independent idea requires a reserved experiment slice.

### V — BLOB binding, preserving SQLite quantization

Eligible when vector JSON conversion/parsing or associated allocation is
observed on the actual path. Replace JSON query-vector transport with exact
f32 BLOB encoding while retaining current SQLite-side quantization/reranking.
No stored-vector/schema change.

Pre-register differential/property checks for supported dimensions,
finite edge values, signed zero, byte order, query results, ordering and
existing invalid-input handling. Confirm no assumed BLOB subtype or native
endianness leaks across platforms. Record residual copies/allocations;
BLOB binding need not eliminate sqlite-vec scratch allocations.

### Q — Rust quantization, conditional on V

Use only if V is correct and measurement shows SQLite-side quantization is
still a worthwhile cost. Q consumes the second treatment slot, comparing
A+V against A+V+Q; report its incremental effect. Preserve exact sign rules,
bit packing, dimension restrictions, tie behavior and full-vector reranking.
No approximate recall tradeoff is authorized.

### D — dispatch correction

Eligible only from measured queue waiting plus idle-worker overlap, not merely
full-channel counts. Select one smallest bounded policy correction after
review; shared-queue redesign is not the default. Prove bounded memory,
backpressure, shutdown/cancellation, fairness and connection ownership.
Keep per-request transactions and synchronous API behavior unchanged.

### L — counter-directed lookaside

Eligible only when measured size/full misses correspond to a material portion
of the contested allocations. If FTS5/vec public allocator calls bypass it,
do not expect resizing to fix those calls. Select one slot/count setting
from miss sizes and peak demand. Report reserved bytes per reader and total,
allocation source and failure behavior. Do not install a global allocator.

### S — smaller statement/copy correction

Eligible only if Slice 76 shows remaining preparation, hydration or
materialization work not addressed by caching. Seal one precise change and
its semantic tests. Preserve complete ranking, eligibility-before-cap,
frozen/current snapshot semantics and error precedence. Broad query-engine
rewrites need another planning slice, not this slot.

## Execution and quantitative decisions

1. Seven fresh anchor runs verify continued environment and identity.
2. For each selected treatment, seven control and seven treatment processes
   in the shared counterbalanced order. For independent treatments compare
   each to A. For Q compare to A+V and disclose the dependency.
3. Apply the shared correctness, >3% sequential-degradation exclusion,
   noise and concurrent-improvement screening rules. Keep all failures.
4. If individual effects justify combining, freeze the union, run focused
   interaction tests, then seven matched A/combination pairs. Do not combine
   a semantically failing treatment or an unexplained slowdown.
   If the final treatment already is the V+Q combination and no other change
   is added, reuse that exact series rather than relabeling and rerunning it.
5. If recovery is not stable, reserve one final diagnostic pass (up to three
   executions) to measure the actual residual on the best candidate.
   Earlier profiles cannot quantify a changed residual.
6. For the candidate selected for implementation, run the retained AC-072
   campaign and both 71B 10k candidate guards using the commands and criteria
   resolved by Slice 76. No historical write baselines or full small-write
   matrix. Changed write behavior would require explicit scope reassessment.
7. Record all seven AC-020 verdicts, median and margin. All seven must pass
   for stable experimental recovery; no six-of-seven override.
   A baseline/candidate green on an unregistered host is diagnostic, not a
   replacement for the registered executor.

Budget: maximum 7 anchor + 28 independent-treatment/control + 14
combination/control = 49 normal AC-020 executions. Reserve 7 for the one
environment restart, total cap 56. Confirmation-only route uses 7, not 56.
At most 9 diagnostic executions across the slice (including final residual).
Protected guards are separately budgeted: one exact retained three-repetition
candidate campaign per workload, not a new baseline investigation. Record
their actual count and commands before execution. No profiling in timing runs.
Four-hour active-work checkpoint and shared timeout/retry stops apply.

## Decision dossier for Slice 80

Deliver decision.md with:

- Ranked eligible candidate(s), exact patches/SHAs, reproduction, functional
  proof, absolute timings, ratio/margin, allocation/queue changes and memory.
- Why rejected/inconclusive candidates were rejected; absent evidence remains
  unknown rather than an implied zero cost.
- Whether there is stable statistics-enabled recovery on the registered
  executor and whether protected guards pass.
- Source and artifact applicability map, expected production blast radius,
  minimal implementation path and focused RED cases still needed.
- Same-file safety census disposition and questions requiring owner input.
- Estimated implementation/verification tasks and maintenance burden,
  distinguished from measured performance evidence.

If no candidate recovers AC-020, do not automatically choose isolation.
Show which residual mechanism isolation would change. WAL/VFS contention
among FathomDB readers, dispatch delays and system allocator bottlenecks are
not by themselves a justification for private MEMSTATUS disabling.
No fixed 60% threshold proves recoverability.

Recommend either a precisely scoped reserved Slice 78/79 experiment or a
consultation on a bounded isolated-Rust proof. The latter must separate
symbol/runtime ownership from performance, memory-control loss, same-file
locking safety, public migrate(Connection) compatibility, and fork/update
burden. No shared-runtime MEMSTATUS run, shutdown sequence, dependency fork
or packaging implementation is authorized by this fallback discussion.

## Closure and handoff

Write durable manifest, observations, result, decision, and reviews under
dev/plans/runs/0.8.25-slice-77/. Independent prototype-code and evidence
reviews must find no unresolved material issue in the claims being handed off.
Update status with each R77 disposition and exact receipt identifiers.

Slice 80 receives a planning contract, not automatic implementation approval.
Additional owner consultation is mandatory even after a clearly winning result.
No broad verification, hosted CI or new package matrix in this slice.
