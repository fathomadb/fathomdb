# Track Runner status

This is the coordinator-owned, live progress board for the
[performance benchmarking and experiments program](PROGRAM.md). It records
coordination state, not measurements: receipts and the append-only experiment
index remain the source of execution evidence.

**Last reconciled:** 2026-08-16

**Integration base:** `afbfeed2` on
`experiments/performance-experiments-20260815`. This integrates the accepted
SCALE-01 manifest-runner preparation history; replace it with the verified
integration SHA at each accepted lane close.

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

The final cross-lane review accepted the integrated safety, configuration, and
receipt boundaries but requested this documentation correction: active
references to legacy `L0` and `T0` labels must use `LOCOMO-01` and `SCALE-01`.
No implementation or execution evidence was challenged. The coordinator will
obtain focused review of the correction before closing this preparation set.

`PARENT-01` may consume the accepted trace contract in preparation but is not
commissioned by this entry. The coordinator alone edits this board and PROGRAM
state. This commission authorizes no corpus acquisition, live grid or smoke
run, extractor, GPU/model, paid service, external write, or push.

## Track status

| ID | Portfolio state | Runner state | Verified evidence / next gate |
| --- | --- | --- | --- |
| SAFETY-01 | Complete infrastructure | Closed; re-check on each new track | Safe receipt/index contract exists; retain as campaign control. |
| TRACE-01 | Complete canary | Closed and integrated | `ca5b656d` integrates the independently accepted `a4a7ed0b` history: three red-first fixes, 10 focused tests, and a full `agent-verify` pass. |
| LOCOMO-01 | Active | Phase-A preparation integrated | `4bbc1b4f` integrates accepted worker `65427d29`: six frozen treatment tuples, 48-cell plan-only catalog, and no live runner. Next gate: separately authorized fixed-subset dry run/execution preparation; no live grid is authorized here. |
| PARENT-01 | Planned | Contract dependency satisfied; not commissioned | May consume the accepted trace contract in preparation; hold writer work behind the two-lane authorization. |
| SCALE-01 | Planned | Manifest-runner preparation integrated | `afbfeed2` integrates accepted worker `9997c2cd`: a pure TC-5 manifest validator and safe planning receipt. Next gate: final cross-lane review, then separately authorized smoke/execution preparation; no live execution is authorized here. |
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
