# Track Runner status

This is the coordinator-owned, live progress board for the
[performance benchmarking and experiments program](PROGRAM.md). It records
coordination state, not measurements: receipts and the append-only experiment
index remain the source of execution evidence.

**Last reconciled:** 2026-08-16

**Integration base:** `acc5fa9a` on
`experiments/performance-experiments-20260815`. This is the verified integrated
LOCOMO-01/SCALE-01 preparation set and its accepted cross-lane correction;
replace it with the verified integration SHA at each accepted lane close.

## Current lanes

`LOCOMO-01` handed off `6bfc1004` from
`/tmp/fathomdb-locomo-01-20260816` on
`experiments/performance-locomo-01-20260816`; independent review requested a
targeted correction before integration. The review accepted its red-first,
no-live scope and pin evidence, but found that treatment IDs could retain their
names while their semantic parameters drifted. Remediation handoff `65427d29`
adds red-first mutation coverage and fail-closed, type-checked validation for
all six frozen treatment mappings; follow-up independent review accepted
`65427d29`, integrated here as `4bbc1b4f`. Its full verifier passed Rust but
stopped at Python collection because this isolated worktree has no local native
binding; that environmental limitation is recorded in its handoff, not treated
as a green gate. The integrated catalog is plan-only; no LOCOMO execution is
authorized.

`SCALE-01` handed off `9997c2cd` from
`/tmp/fathomdb-scale-01-20260816` on
`experiments/performance-scale-01-20260816`; independent read-only review
accepted it and it is integrated here as `afbfeed2`. Its red-first, focused 10-test, full
80-experiment-test, and full `./scripts/agent-verify.sh` evidence passed
(`72/73` suites, one documented skip). The worktree-local native binding used
for that verifier is ignored local state, not a source change. No TC-5
execution is authorized.

The final cross-lane review accepted the integrated safety, configuration,
receipt, and no-live boundaries at `acc5fa9a`, after focused review accepted
the active-identifier correction from legacy `L0`/`T0` to
`LOCOMO-01`/`SCALE-01`. The unchanged combined full verifier passed
(`agent-test.sh: 72/73 suites passed`, one documented skip; AC-037 had its
expected environmental downgrade). No implementation or execution evidence was
challenged.

HITL authorization `seq-249` permits: the LOCOMO fixed-subset dry run and
Phase-B CPU/FTS grid; LOCOMO GPU/CE cells; the SCALE TC-5 smoke and long CPU
characterization; and CORPUS-01 corpus and human-gold work. These permissions
do not commission a writer or replace the required frozen run configuration,
worker handoff, review, safe receipt/index, or artifact boundaries. No paid
service, extractor, push, or downstream ANSWER-01/MEMORY-01/SCALE-02 execution
is authorized.

`PARENT-01` may join the LOCOMO grid by HITL direction but is not commissioned:
its exact child/parent relation, neighbor bound, fusion, and returned context
remain a proposal for HITL approval. The coordinator alone edits this board and
PROGRAM state.

## Track status

| ID | Portfolio state | Runner state | Verified evidence / next gate |
| --- | --- | --- | --- |
| SAFETY-01 | Complete infrastructure | Closed; re-check on each new track | Safe receipt/index contract exists; retain as campaign control. |
| TRACE-01 | Complete canary | Closed and integrated | `ca5b656d` integrates the independently accepted `a4a7ed0b` history: three red-first fixes, 10 focused tests, and a full `agent-verify` pass. |
| LOCOMO-01 | Active | Execution authorization recorded; not yet commissioned | `seq-249` authorizes the fixed-subset dry run, Phase-B CPU/FTS grid, and GPU/CE cells. Next gate: commission a reviewed executable lane with frozen external inputs and receipts. |
| PARENT-01 | Planned | Grid inclusion authorized; frozen treatment proposal pending | TRACE-01 dependency is satisfied and HITL directed that it join the LOCOMO grid. Next gate: approve the exact treatment, then commission preparation/implementation. |
| SCALE-01 | Planned | Execution authorization recorded; not yet commissioned | `seq-249` authorizes the TC-5 smoke and long CPU characterization. Next gate: commission a reviewed executor with manifest, external-root, and host preflight evidence. |
| CORPUS-01 | Planned | Corpus and gold-work authorization recorded; not yet commissioned | `seq-249` authorizes corpus/license matrix and human-gold work. Next gate: commission the preparation lane with external-data and licensing boundaries. |
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
