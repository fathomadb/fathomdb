---
title: Slice 85 product-focused review plan
status: COMPLETE
target_release: 0.8.27
date: 2026-09-30
branch: slice-85-fix
review_candidate: 6ec4194e7f2bc514922a1dcbf4514957f0c788b6
---

# Slice 85 review plan

Completed on 2026-09-30. Results: [product review](dev/plans/0.8.27/features/slice-85/product-review.md).

Review the complete recovered Slice 85 deliverable, including its original
engine moves. Compare `8b2a9edaf..6ec4194e7`; separately inspect
`df1ffd000..6ec4194e7` to distinguish recovery changes. Bind findings and test
results to the candidate above. Its product source matches `294af94b5`, the
candidate qualified in the [recovery receipt](dev/plans/0.8.27/features/slice-85/recovery-receipt.md).
The scope is review only; implementation and landing require separate instructions.

## Effort allocation

Allocate reviewer investigation, code reading, test selection and result analysis
by the percentages below, rather than by changed-line counts. For any total
budget, reserve 60% / 30% / 10% for these three groups. Unattended test runtime
is not reviewer effort; blocked hardware work must not consume another group's
allocation. Review critical paths first.

| Group and area | Effort | Review focus |
| --- | ---: | --- |
| Critical: engine lifecycle and SQLite/WAL ownership | 20% | Trace open, failure unwind, explicit close and implicit drop through `lib.rs`, `reader_pool`, `reader_transaction` and `wal_attribution`. Check callback userdata lifetime, transaction finish order, reader shutdown, worker-zero WAL pinning and checkpoint safety. |
| Critical: read dispatch and snapshot correctness | 20% | Trace public reads through `read_api` to reader-owned requests and handlers. Check request/factory ownership, reply/error propagation, frozen snapshot consistency, filter refusal precedence and the boxed vector-stage payload. |
| Critical: ordinary search | 20% | Trace `search_api`, `search`, `filter` and `frozen_read` end to end. Check same-snapshot eligibility, filtering before caps, ranking/result parity and narrow-error conversion at public boundaries. |
| Other touched runtime: graph expansion | 15% | Trace `graph_api` through execution, traversal, codec and types. Check continuation validity, transaction release, filter errors, result/error parity and required facade visibility. |
| Other touched runtime: optional ML routes | 10% | Inspect moved embedding/reranking facade calls and actual provider invocation. Confirm CPU behavior and feature wiring; obtain focused GPU evidence on a capable executor or record it unverified. |
| Other touched runtime: operational/operator routes | 5% | Check moved operational-state reads, structural state, operator verification and touched telemetry/erasure interactions for unchanged behavior. |
| Non-runtime: gate, test infrastructure and records | 10% | Spend 6% on the reduced gate/policy and tier wiring, 2% on Engine-owned pause hooks and test synchronization, and 2% on ADR/interface/receipt accuracy. |

## Review method

1. Trace concrete runtime callers, owners and teardown paths; compare moved
   bodies with the pre-slice versions. Read the runtime-topology
   [ADR](dev/adr/ADR-0.8.27-engine-owned-runtime-topology.md) against code,
   distinguishing current ownership from future Slice 90 work.
2. Inspect assertions and RED/GREEN evidence; reuse candidate-bound receipts.
   Run focused tests for missing evidence or a suspected regression: lifecycle/WAL,
   reader/frozen/filter and search races; slice60 graph/wire; slice80 transaction
   release; slice85 errors; tc5 vector; and actual CPU/provider routes. Enable
   required features. Full regressions require comprehensive subsequent changes.
3. Bound tooling review to ownership/forbid enforcement, supported root/helper
   paths including Engine methods, cfg coverage, guards, wrapper freshness and
   tier wiring. Inspect pause-hook isolation and retained test assertions.
   Do not restore inventories or pursue absent Rust laundering forms.
4. Report each finding's severity, location, product consequence, reproduction
   or code proof, affected callers/features and smallest suggested correction.
   Separate defects from missing evidence. Follow-up work must rate at least 4/5.

## Deliverable and stopping point

One short report: actual 60/30/10 effort breakdown, area verdicts, findings,
focused checks and evidence gaps. Review each area once; validate concrete
findings with focused checks. Preserve tests and baselines; avoid another
adversarial review loop.

Carry forward the receipt's limits: original full verification failed, with
its eight Python import failures separately resolved by a focused rerun;
official public/hidden comparisons and the hidden release probe are unverified
due to disk/CUDA blockers; GPU runtime evidence remains outstanding. Review
completion does not establish release acceptance, authorize rebinding, or settle
Slice 90 prerequisites.
