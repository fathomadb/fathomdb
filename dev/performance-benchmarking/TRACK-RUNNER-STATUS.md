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

No writer lane is active. The Track Runner control, its scoped agent
instructions, and the existing LOCOMO/Mem0 harness track tags are integrated
at `aa12226a`; their guard and harness-contract tests passed before that
integration. The next coordination action is to commission the `TRACE-01`
canary through its dated contract and test handoff. No corpus, GPU/model, paid,
or external execution is authorized by this status entry.

## Track status

| ID | Portfolio state | Runner state | Verified evidence / next gate |
| --- | --- | --- | --- |
| SAFETY-01 | Complete infrastructure | Closed; re-check on each new track | Safe receipt/index contract exists; retain as campaign control. |
| TRACE-01 | Planned | Queued canary | Reconciled against `aa12226a`; create dated trace contract and human-intended lifecycle tests. |
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
