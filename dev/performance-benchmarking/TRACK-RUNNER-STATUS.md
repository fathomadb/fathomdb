# Track Runner status

This is the coordinator-owned, live progress board for the
[performance benchmarking and experiments program](PROGRAM.md). It records
coordination state, not measurements: receipts and the append-only experiment
index remain the source of execution evidence.

**Last reconciled:** 2026-08-16

**Integration base:** `aa12226a` on
`experiments/performance-experiments-20260815`. Track Runner control and the
verified harness-contract changes are committed at this base; replace it with
the verified integration SHA at each accepted lane close.

## Current lanes

`TRACE-01` is in an isolated writer worktree at
`/tmp/fathomdb-trace-01-20260816` on branch
`experiments/performance-trace-01-20260816`, based on `1613824f`. The worker
handed off `7ae0ae50` after its red checkpoint `ebb30ff0`; focused lifecycle
tests passed, but the first independent review requested changes. It found that
the writer did not fully validate sidecar payloads, identifiers were too
permissive for a content-free artifact, and deterministic diagnostics/order
needed explicit tests. The worker now owns a new red-test/fix cycle; this
coordinator owns this board and PROGRAM state. The next gate is a re-handoff,
full verifier result, and fresh independent read-only review. No corpus,
GPU/model, paid, or external execution is authorized by this status entry.

## Track status

| ID | Portfolio state | Runner state | Verified evidence / next gate |
| --- | --- | --- | --- |
| SAFETY-01 | Complete infrastructure | Closed; re-check on each new track | Safe receipt/index contract exists; retain as campaign control. |
| TRACE-01 | Planned | Review changes requested | `7ae0ae50` is not accepted: enforce complete safe-sidecar validation, safe identifier grammar, and deterministic diagnostics/order before re-review. |
| LOCOMO-01 | Active | Prepared, no worker lane | Complete Phase-A provenance/timing/GPU readiness before the full grid. |
| PARENT-01 | Planned | Blocked on TRACE-01 integration | Consume the accepted trace contract; then freeze its bounded treatment. |
| SCALE-01 | Planned | Queued independent lane | Implement and test manifest-backed all-real fidelity runner. |
| CORPUS-01 | Planned | Queued independent preparation | Create corpus/license matrix and human-gold readiness contract. |
| ANSWER-01 | Blocked | Waiting for selected retrieval survivor | Require LOCOMO/PARENT selection plus scorer and cost preflight. |
| MEMORY-01 | Blocked | Waiting for ANSWER-01 | Require selected profile, native prerequisites, and declared spend ceiling. |
| SCALE-02 | Blocked | Waiting for SCALE-01 and selected profile | Freeze workload matrix and distinguish canonical from derived counts. |
| LATENT-01 | Parked | Not commissioned | Requires diagnosed cross-window discourse failure. |
| GRAPH-01 | Planned | Not commissioned | Requires bounded graph design and supporting-evidence protocol. |
| GLOBAL-01 | Parked | Not commissioned | Requires GRAPH-01 relevance and native cost/reproduction preflight. |
| REASON-01 | Parked | Not commissioned | Requires GRAPH-01 relevance and external Python/credential/corpus prerequisites. |
| SEARCH-01 | Complete historical | Closed | Retained FTS descriptive baseline; no current execution. |

## Update protocol

Only the Track Runner coordinator edits this file. Update it in the same
integration change whenever any of these events occurs:

1. A lane is commissioned: record its base SHA, branch/worktree, owner role,
   owned paths, and the next gate in **Current lanes**.
2. A worker hands off: record the commit SHA, test/verification result,
   independent-review verdict, and any blocker or authorization request.
3. A lane is integrated, rejected, blocked, resumed, or closed: update the
   affected row, PROGRAM/charter state if it truly changed, and link any safe
   receipt or run ID.
4. A final cross-lane review closes: record its reviewed integration SHA and
   the resulting next authorized action.

Never copy raw metrics, corpus payloads, credentials, or model output here.
Never change a row from blocked or planned to complete based on narration: link
the reviewed commit and receipt, then apply the charter's exit rule.
