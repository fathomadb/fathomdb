---
title: 0.8.25 Slice 10 implementation review — cycle 2
status: FAIL_FIXED_IN_FIX_3
reviewed_commit: 0d5044f4
reviewer: independent subagent
---

# Slice 10 implementation review — cycle 2

## Verdict

FAIL. The final permitted implementation FIX cycle was required.

## Findings and disposition

1. **P1 — Physical record quarantine broke the canonical run contract.**
   FIX-3 restores the byte-original v1 record at
   `experiments/runs/<run_id>/record.json`. Policy quarantine now changes only
   evidence eligibility and pins the original record hash. The logical-artifact
   test recognizes only this closed, policy-named legacy exception, and a new
   regression requires a canonical record for every index row.
2. **P2 — The accepted native witness predated FIX-2.** FIX-3 requires a new
   successful witness after the final implementation commit so its git-blob
   artifact binds the completed classifier and receipt-finalization code.
3. **P2 — Negative coverage omitted arm ownership and blocked continuation.**
   FIX-3 adds an arm/witness mismatch test and preregisters a real, local,
   expected-hit-missing run. Its committed receipt must validate as blocked and
   remain ineligible as successful evidence.

FIX-3 is the third and final implementation correction cycle. Any remaining
P1/P2 finding blocks Slice 10 rather than opening another correction cycle.
