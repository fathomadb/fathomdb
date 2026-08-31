# REASON-01 compact-ledger v2 plan

## Outcome

Correct the evidence-empty and malformed-cell behavior exposed by v1, then
determine descriptively whether `protected_evidence_ledger_v2` preserves the
frozen protected retrieval gain without losing answer correctness,
groundedness, or attribution against corrected A0.

This is a new preregistered protocol over the consumed 109-case cohort. It does
not repair or resume v1 and cannot authorize promotion.

## Requirements and acceptance criteria

- Reuse only the frozen retrieval candidates. Generate all answer and scorer
  cells in a new v2 checkpoint.
- Validate that every requirement is either covered by selected evidence or
  listed missing, never both.
- When every requirement is missing and no evidence is selected, make no
  reader call. Emit the exact canonical abstention: `The available memories do
  not contain enough information to answer this question.`
- Build the answer JSON schema per cell. Citation items are restricted to the
  exact selected IDs; an empty evidence set permits zero citation items.
- After five schema or semantic failures, persist a terminal quality failure
  for that cell and continue. Correctness, groundedness, attribution, and
  citation validity are false. Transport, spend, checkpoint, source-binding,
  and context-limit failures still stop execution.
- A deterministic all-missing abstention is grounded because it makes no
  positive factual claim beyond bounded-memory insufficiency. It is attributed
  to the recorded all-requirements-missing ledger state, not to a source
  citation. Report this basis and valid-abstention rate separately.
- Score abstention correctness with the official LongMemEval abstention prompt.
  An all-missing abstention on a non-abstention case is deterministically
  incorrect even though its bounded-memory grounding remains valid.
- Report terminal malformed counts by stage and arm. They count as false in
  paired quality metrics and cannot disappear from completeness accounting.
- Preserve atomic response-first checkpoints, full `Retry-After`, exact model
  routes, stage cost/latency, and the existing $9 local/$10 provider caps.

The descriptive quality boundary remains no worse than corrected A0 on answer
correctness, groundedness, and attribution, with complete bindings and cells.
Passing permits only a plan for an untouched confirmation.

## Design

The existing runner selects behavior from the typed configuration schema. V1
retains its historical checkpoint and artifact names; v2 uses
`reason01.compact-ledger-checkpoint.v2` and v2 receipt/summary names.

For each v2 treatment cell:

1. Validate exact strips and complete evidence-or-missing requirement coverage.
2. If every requirement is missing, persist the canonical abstention and its
   ledger-state basis without reader or evidence-judge calls.
3. Otherwise, construct a strict answer schema whose citation enum is the
   sorted selected-ID set, then call and validate the reader.
4. If any model stage exhausts semantic repair, persist its stage-specific
   false result. Skip impossible downstream calls and continue the arm/case
   rotation.

Provider, timeout, checkpoint, binding, context, and cost-cap errors are not
quality observations and remain run-stopping failures.

## Plan

1. Freeze this contract and `compact-ledger-run.v2.json`.
2. Add failing end-to-end tests for all-missing abstention, per-cell citation
   schemas, retry exhaustion, and continuation to the next case.
3. Implement the smallest compatible v2 path in the existing runner.
4. Run focused tests, type/lint checks, and a zero-spend input/config preflight.
5. Update PROGRAM and the status board. Return before paid execution.

## Stop

Do not change prompts, models, retry counts, abstention text, scoring semantics,
or decision boundaries after a paid v2 checkpoint begins. Do not interpret a
partial checkpoint or use this consumed cohort for promotion.

## Readiness

- Focused tests: 15 passed, including all-missing no-reader execution and
  malformed-cell continuation to the next case.
- Config SHA-256:
  `8aed0276235401c4836cf179178777585d1811795caae9cad3775cf26692b527`.
- Runner SHA-256:
  `a6e49dcb41d972f21d60997f98f4304502b362c07cb23443047536021ce4ab31`.
- Zero-spend preflight receipt:
  `data/performance-benchmarking/locomo-multihop/runs/reason01-compact-ledger-v2-preflight-20260830-01/reason01-compact-ledger-receipt.v2.json`
  (`sha256:0dacfc71597374d0e95cdabd14ae6f624209d5256de6fb95d002ea694f8c0a76`).

## Closeout

The paid dynamic-schema route probe passed, and the fresh v2 diagnostic
completed all 109 cases. The registered decision was `descriptive_fail`:
answer accuracy increased 2.75 points versus A0, while groundedness decreased
9.17 points and attribution decreased 5.50 points. Close the offshoot and
retain A0. See the [result](2026-08-30-reason-01-compact-ledger-v2-result.md)
and [receipt](../../experiments/runs/reason-01-compact-ledger-v2-20260830T2156Z-572f51ea/record.json).
