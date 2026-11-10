---
title: Slice 150 — Conditional selection, coalescing and SDK hints experiments
status: PROPOSED
target_release: 0.8.29
slice: 150
---

# Slice 150 — Conditional selection, coalescing and SDK hints experiments

## Goal, maturity and inspected sources

- Decide whether a bounded experiment earns further product consideration; proposals are not treatments to ship.
- Authority: [brief](../../planning-brief.md), [reconciliation](../../reconciliation-plan.md),
  [review schedule](../../../0.8.29-0.8.33-experimental-review-schedule.md), and
  [roadmap checkpoint](../../../../ROADMAP.md#0829--candidate-selection).
- Preserve the [.28 decision source](../../../0.8.28-draft-scope.md), its
  [checkpoint relocation](../../../plan-0.8.28.md), [recorded rulings](../../../release-state-0.8.28.json),
  and [historical slice proposal](../../../0.8.28-0.8.29-slice-proposal.md); later rulings govern placement.
- Inspected current [fusion](../../../../../src/rust/crates/fathomdb-engine/src/fusion.rs),
  [search](../../../../../src/rust/crates/fathomdb-engine/src/search.rs), and pure
  [hints](../../../../../src/rust/crates/fathomdb-engine/src/hints.rs): body-keyed fusion is still baseline;
  the engine hint subset exists. MCP default-on does not prove SDK stage-2 delivery or its benefit.
- Existing queue/Block 1 is integrated; old queue maps and timings are historical, not current bottleneck proof.
  No experiments, product tests, priced calls or environment changes occur during this drafting slice.

## Slice preparation: local needs, requirements and acceptance drafts

- S150-N1: consumers need complementary, attributable evidence within accepted eligibility and latency bounds.
- S150-N2: operators need evidence that coalescing or hints improve useful work enough to justify their costs.
- S150-R1 / S150-A1: every inventory item has an explicit authorize-bounded-study, move or postpone/Park
  disposition from Slice 9; no authorization is inferred from this plan or historical placement.
- S150-R2 / S150-A2: each authorized study has an independently reviewed preregistration before any held-out
  result is examined, with fixed baseline, hypothesis, cells, budgets, stopping rule and numerical decision rule.
- S150-R3 / S150-A3: report separately retrieval/support-set results and end-to-end answer-system results;
  no answer-use claim follows from recall alone, and no invalid/incomplete run produces an acceptance verdict.
- S150-R4 / S150-A4: every accepted, rejected or inconclusive treatment has reproducible artifacts and a
  recorded next disposition; product changes require a separate reviewed slice and explicit placement before 160.
- Produce `requirements.md`, `design.md`, `test-map.md`, `checkpoint-dispositions.md`, per-study
  preregistrations and a results index. These are future deliverables; local IDs create no canonical ACs.

## Checkpoint inventory: preserve individual dispositions

| Item | Question preserved for Slice 9; none is automatically committed |
| --- | --- |
| D28-01 | Manual named-profile qualification; a reproducible non-A0 use case first; omission retains A0. |
| D28-02 | Time/changed-fact profile needs source-derived validity gain beyond existing filters; no truth inference. |
| D28-03 | Rich graph and operational-state continuation needs a bounded consumer, authorization and replay case. |
| D28-04 | Expanded exclusion tracing needs a decision the compact explanation cannot answer. |
| D28-05 | Frozen-snapshot leases need a multi-operation case unmet by compact frozen reads. |
| D28-06 | Opaque cursors need demonstrated omission/duplication or authorization risk under current continuation. |
| D28-07 | Persisted evidence receipts need a lifetime/replay use case preserving current authorization. |
| D28-10 | Multi-hop reachability needs frozen source/gold/model/protocol and held-out support-set scoring by hop. |

- D28-01–07 and D28-10 moved to the .29 checkpoint; they did not acquire implementation approval.
  Retain D28-10's historical 36/100 complete-at-20 and 1/25 four-hop observations as source-bound evidence,
  not current native baselines. Record all eight decisions even if no family is authorized.
- Candidate families: entity/alias expansion; complementary-evidence/coverage-aware selection;
  MMR/diversity beyond accepted A0. Slice 9 may select one bounded family, move or Park any/all others.
- Separately retain placed studies of identical concurrent-call single-flight, burst query-embedding batching,
  and SDK hints stage 2/default. Historical placement does not authorize production code or default changes.

## Proposed study design and research fit

- Every study, including coalescing, hints and optional probes, records current source commit, native artifact/
  features, format, corpus/snapshot, query/gold splits, fusion identity/weights, model/tokenizer and protocol.
  Freeze native pre-cap lifecycle/validity/dependency eligibility and fallback behavior as the baseline contract.
- Preregister per-family search space, development/held-out separation, paired units, seeds/repetitions,
  uncertainty/multiple-comparison method, minimum useful gain and allowed latency/cost regression before runs.
  Select parameters on development data only; held-out failure cannot trigger test-set tuning or extra cells.
- Selection families compare with accepted A0 on the same eligible pool and budgets. Measure complete support
  coverage and answer use, correctness, groundedness, attribution, lifecycle and p50/p99; freeze answerer/judge
  and gold isolation. Accept only the preregistered useful held-out gain with every guardrail satisfied.
- Coalescing/batching compare separately with the integrated queue; fix workload, duplicate ratio, concurrency,
  burst shape and write interleaving. Measure useful completions, errors, queue/service/end-to-end latency,
  GPU work and resource release. Include cancellation, follower abandonment, isolation and lifecycle boundaries;
  deduplication keys must not cross stores, views, authorization, filters, generation or configuration identities.
- SDK hints study reconciles [accepted ADR](../../../../adr/ADR-0.8.29-agent-hints-sidecar.md) and
  [stage-2 design](../../../../design/agent-access/hints-sidecar.md) with current Rust/Python/TS surfaces.
  Compare hints off/on for task outcomes, next-call usefulness, bytes/tokens and overhead; preserve pure hints,
  budget accounting and precedence. Stage-2 carrier implementation itself needs reviewed placement first.
- Conditional overflow probes require a recorded [tripwire](../../../../design/overflow-chunking-reevaluation.md)
  disposition: paired answer-level original/rotated past-cut evidence and current latency/row-count estimates.
  Move byte-identical twins together and count twin gold; compare within question. No chunk vectors are needed
  for T1. Chunking remains parked; a fired tripwire reopens review, not enablement. No ANN work is authorized.
- Planned primary-source research: original alias-expansion/MMR/coverage methods and model evaluation protocols;
  pinned native batching/provider and queue semantics. Record exact versions, assumptions and fit gaps in design.
  Database-owned decomposition, synthesis, consolidation, verification and abstention remain outside this slice.

## Prerequisites, resources and ordering

- Hard execution prerequisites: Slice 9 study authorization, approved Slice 10 owned setup, current Slice 30
  baseline, reviewed preregistration, accessible frozen fixtures and approved compute/cost ceilings per study.
- Design/preregistration may begin early. Freeze a current native baseline before executing; if Slice 50 changes
  fusion identity, requalify affected retrieval/answer/attribution claims and baseline comparisons on integrated 50.
  Do not block unrelated product slices on an unselected study; report whether 40/60/70/90 changes affect it.
- Newly explicit prerequisites: fixture/gold access, qualified native artifact, stage-2 placement if needed,
  declared coalescing isolation key, queue bottleneck evidence, and any paid judge/answerer authorization.
- Reserve an owned runner, storage/runtime window and per-study money/token/GPU caps; do not assume availability.
  Follow [GPU policy](../../../../design/gpu-eval-activities-policy.md): real embedding/rerank work on available
  RTX3090, verify actual engagement; queue if busy. Keep R580/primary CUDA12.6 fixed and never use K620.
  Same-backend fidelity/targeted CPU compatibility exceptions remain distinct from answer/relevance studies.
- Priced/long runs need tested incremental checkpoint/resume, bounded retry/backoff and window-fit before spend.
  Serialize shared-file/product landings and runner contention; isolate any later code writer in its own worktree.

## Tasks, validation and stop decisions

1. Assemble the disposition matrix and challenge each proposed consumer value against current implementation.
2. Review requirements/design/test mapping and per-study preregistration independently; obtain missing rulings.
3. Implement only separately authorized harness work with failing tests first for metric math, gold isolation,
   paired counts, resume and real-database adapter behavior. Retain rerunnable tools/configuration in the repo,
   indexed by experiment README; preserve fixtures, outputs, receipts, raw logs, resource costs and negative results.
4. Execute only approved cells within caps; publish typed retrieval versus answer conclusions and limitations.
5. Independent review audits baseline identity, leakage, paired calculations, eligibility and decision-rule adherence.
   Accept/reject/inconclusive is a study verdict; product promotion requires requirements/AC/design, ADR/interface
   changes where needed, RED→GREEN and code review in a newly approved slice or explicit reviewed placement.
6. Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md): real-DB boundaries,
   meaningful measurement tests and resource release; defer exhaustive fuzz/MC/DC/CUDA matrices and tamper tests.
   Planning artifacts use scoped Markdown checks; executable/source work later requires the full agent gate.

- Stop on missing approval/setup/baseline, leakage, invalid completeness, budget exhaustion, eligibility or safety
  violation; do not expand cells or repair a failed hypothesis through tuning. Preserve and report the evidence.
- Close only when every inventory item is completed or explicitly moved/Parked and 160 has the resulting scope.
  No automatic SDK default flip, ranking/profile/API promotion, chunking enablement or publication follows.
