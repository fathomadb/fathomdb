---
title: 0.8.25 Slice 45 — minimal pagination and operational state
status: DRAFT_REVIEW
depends_on: 40
design: design.md
design_status: FORMAL_REVIEW_REQUIRED
---

# Slice 45 plan

## Outcome and boundaries

Implement S45-R1 through S45-R7 from the
[`design`](design.md): bounded stable pages over logical canonical nodes and
governed point/page reads over the existing `operational_state` table. Reuse
the Slice 35 frozen-read authority and Slice 40 generation state. Do not add a
second snapshot mechanism, persisted cursor lease, graph/search pagination,
semantic latest-state policy, or client-side filtering.

## Delivery sequence

1. Reconcile design v4 with implemented Slices 15/35/40, architecture v2, and
   the approved scope adjustment. Obtain independent design review; allow at
   most four documented FIX-n cycles. Unresolved P1/P2 findings block READY.
2. Add the pagination ADR and update Rust/Python/TypeScript/wire interface docs
   with the exact additive contract and successor pointers.
3. Commit failing real-database, property, schema, race, query-plan, wire, and
   binding tests before product implementation. Record exact RED commands and
   diagnostics; do not alter these tests during fix-to-spec.
4. Implement schema step 33, cursor codec, reader-snapshot page primitives,
   operational collection governance, Rust facade, Python, and TypeScript in
   that order. Run focused tests after each GREEN seam, then refactor without
   changing the accepted contract.
5. Obtain independent code review at the exact implementation commit. Allow at
   most seven documented FIX-n cycles; every fix receives a focused regression
   test and re-review.
6. Give a separate read-only verifier the exact commit, requirements matrix,
   and commands. Run focused, fast, heavy, all/applicable features,
   source-independent packages, Windows, and the registered 10k/50k page
   workload. Preserve raw evidence and hashes.
7. Execute the matched Slice 45 overhead campaign. Measure context minting,
   first-page and continuation latency, unfrozen versus frozen canonical reads,
   current versus frozen operational-state points, throughput, RSS, database
   bytes, and cursor/token size. Classify materiality using the preregistered
   dual absolute/relative policy before considering an optimization.
8. Write `status.md`, update the design index/interface successors and release
   state through its JSON authority, regenerate views, run the final clean
   verifier, commit, and push the explicit release branch.

## Performance experiment

Use deterministic fresh 10k and 50k databases and fixed 100-item pages. Compare
the existing indexed `read_list_filter` result with the new canonical page over
an equivalent view/filter, and compare operational-state point reads with and
without a pre-minted frozen context. Report context mint, first page,
continuation, full-walk, and point-read costs separately. Run three process-cold
and five steady repetitions, with at least 1,000 measured operations in each
steady cell; alternate treatment order and pin host/build/SQLite/configuration,
input digest, CPU affinity, and reset/warm-up procedure.

An effect is material when candidate p95 latency increases by both more than
10% and more than 0.25 ms, or peak RSS increases by both more than 5% and more
than 8 MiB. Report confidence intervals and raw repetitions even when neither
threshold is crossed. A material effect keeps the slice open for causal
analysis and an explicit reviewed disposition; it never justifies weakening
snapshot, eligibility, or cursor integrity.

## Stop gates

Stop on mixed-state pages; duplicate/omitted successful walks; cursor content
leakage or forgery; eligibility after truncation; operational mutation-log
fallback; unindexed ordering; anonymous rows represented as logical nodes;
public-surface drift without parity; test-oracle relaxation; an undisposed
material performance effect; or an unresolved P1/P2 review finding.

## Verification routes

Selected: focused Rust/schema, fast, heavy, all, applicable
all-feature/operator, Windows CPU/native Rust/Python/Node, fresh-wheel Python,
npm/native, locally packed pagination smokes, and matched 10k/50k CPU latency/
RSS workloads. CUDA, CE, GPU, operator recovery, live-model, and registry
publication routes are N/A.
