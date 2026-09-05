---
title: 0.8.25 Slice 45 — minimal pagination and operational state
status: READY
depends_on: 40
design: design.md
design_status: READY
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

Use checked-in deterministic fresh 10k and 50k logical-node/state-row corpora
with fixed 256-byte bodies/payloads and 100-item pages. Primary causal cells
compare mint-plus-first-page versus the same first page with a pre-minted
context, the exact page query without versus with frozen/cursor work, first
page versus continuation with stage timing, and operational-state point reads
without versus with one pre-minted context. The context-mint cell reports mint
validation, snapshot binding, token codec, and page-query stages
separately. Keep the existing public list and full page walk as separate
operational observations. Record terminal-row count and frozen parse,
authentication, binding, and query stages. Retain the pre-GREEN O(N) terminal
digest pilot, then verify the schema-33 compact binding and support indexes
remove that cost without weakening drift refusal.

Run ten independent paired steady processes per primary cell and scale, with
each pair/repetition isolated in its own fresh process and at least 1,000
operations per process, three additional matched cold repetitions
with every arm in its own fresh process, and five fresh peak-RSS processes per
arm. Reuse one restart-portable pre-minted frozen-context fixture for matched
cold operations and report database-open time separately. Keep the exact-page
and current-state RSS controls free of context minting or frozen-page setup;
reconstruct pre-minted fixtures only for arms that consume them. Independently
balance each steady pair's order and alternate each cold pair's order. Pin
host/build/SQLite/configuration, input and runner digests, CPU affinity, reset/
warm-up procedure, and retain raw/result hashes. Use a fixed-seed 10,000-draw
paired percentile bootstrap over repetition-level p95 deltas.

An effect is material per primary cell when the median paired p95 steady-state
latency or median paired cold-operation latency increases by both more than 10%
and more than 0.25 ms, or median peak RSS increases by both more than 5% and
more than 8 MiB. Report intervals and raw repetitions even when neither
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
