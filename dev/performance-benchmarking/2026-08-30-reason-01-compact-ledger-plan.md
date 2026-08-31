# REASON-01 compact-ledger offshoot plan

## Outcome

Determine whether `protected_evidence_ledger_v1` can retain the accepted
protected retrieval gain while restoring answer correctness, grounding, and
attribution. This is a diagnostic reuse of the consumed 109-case cohort, not a
new held-out acceptance result.

## Requirements and acceptance criteria

- Reuse the frozen REASON-01 checkpoint; do not rerun or alter retrieval.
- Regenerate A0 raw, protected raw, and protected compact answers in a
  deterministic interleaved order under one current model route.
- Select at most ten exact source-linked evidence strips from the protected
  candidate pool; validate every quote against its canonical body and enforce
  an answer-input limit no larger than both 12,000 characters and that case's
  A0 answer-input length.
- Keep the ledger ephemeral; preserve source IDs and never write derived memory.
- Score answer correctness without retrieval context using the official
  LongMemEval multi-session prompt.
- Score grounding and attribution from the answer plus cited evidence only.
- Persist every paid response atomically in a new offshoot checkpoint, resume
  only unfinished stage cells, honor full `Retry-After`, and stop at a dedicated
  isolated Airlock instance's $10 incremental OpenRouter provider cap.
- Report exact answer-turn recall, non-empty rate, answer correctness,
  grounding, attribution, citation validity, cost, and paired uncertainty.
- The offshoot succeeds descriptively only if the compact arm's point estimate
  is no worse than corrected A0 on answer correctness, grounding, and
  attribution while the protected retrieval result remains unchanged. Paired
  uncertainty is reported but is not an acceptance gate on this consumed
  cohort.

## Plan

1. Freeze the [design](2026-08-30-reason-01-compact-ledger-design.md) and obtain
   an independent review.
2. Write focused failing tests for strict ledger parsing, quote/source
   validation, corrected scoring prompts, checkpoint resume, and decision rules.
3. Implement only enough runner behavior to make those tests pass.
4. Perform a zero-spend checkpoint/config/Airlock preflight and one non-benchmark
   shape probe. Verify the isolated state and authoritative provider cap.
5. Run the compact ledger and answer cells, then corrected scoring for all three
   arms.
6. Preserve a blinded disagreement audit queue, write a receipt and result, and
   update PROGRAM and the status board. The queue remains a limitation until
   independently adjudicated.

## Stop

Stop on checkpoint drift, invalid source binding, quote fabrication, incomplete
arms, unresumable state, repeated malformed model output, or the cost cap. Do
not select thresholds from the consumed outcomes or continue to native
HippoRAG-2 from this diagnostic alone.

## Result

Closed incomplete. The frozen run stopped after 47 complete cases when the
compact answer stage exhausted its five semantic attempts on an evidence-empty
ledger. See the [result](2026-08-30-reason-01-compact-ledger-result.md) and
[content-free receipt](../../experiments/runs/reason-01-compact-ledger-20260830T2002Z-ab15e1aa/record.json).
