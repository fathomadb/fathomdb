---
title: FathomDB 0.8.27 Slice 50 - independent verification
status: PASS
target_release: 0.8.27
candidate: 1f5b8614813b5a363ec5f81fcb580d48da4a4e8f
---

# Slice 50 independent verification

The independent read-only verifier returned **PASS** at clean candidate
`1f5b8614813b5a363ec5f81fcb580d48da4a4e8f`.

| Gate | Result |
| --- | --- |
| Correction-safe erasure | 7 passed; 0 failed or ignored |
| Fast-tier target coverage | 260 targets; 53 feature-complete-only |
| Slice 30 public surface | Equal; 13 rows / 43,673 entries; empty metadata and row diffs |
| Hidden structural surface | All 8 rustdoc rows equal; 41-item release probe equal |
| Hidden test inventory | 10 reviewed tests added; 0 removals; 0 changes |
| `scripts/agent-verify.sh` | 127/127 suites passed; 0 skipped or excluded |
| Security summary | 0 violations; 0 blockers; 0 downgrades |
| Python candidate receipt | PASS; exact candidate and native module verified |
| Workspace Clippy, warnings denied | PASS |
| Workspace Cargo check | PASS |

The public capture SHA-256 was
`89fec9c58d290471de739c82cd2eb1af78ba0d901acd0b0c640381f9d668ec12`;
the immutable baseline SHA-256 was
`06212f662b2a3897447fe294d65d87ae93255b9b511d9fdd57eab3cf8490a723`.
The hidden capture SHA-256 was
`b082ff7da3f7bb5d4c064bdbfbb5b1d835696367a1c4da1a310237d70ca1deef`.
Its strict comparison differed only through 37 occurrences of 10 approved
additive tests: 2 Slice 50 regression tests, 7 earlier approved calibration
tests, and 1 earlier approved eu8 destination test. The baseline was not
regenerated.

The first broad run exposed the stale split-owner WAL guard, which received its
own reviewed RED/GREEN correction. The first run of the final candidate then
found two temporary-environment import failures because the non-editable wheel
environment lacked the documented `src/python` path. Those exact tests passed
2/2 after correcting only the disposable environment, and the unchanged full
gate passed 127/127. The sandbox ptrace denial was handled by the documented
unchanged unconfined route. No candidate failure was reported as a pass.

GPU/feature-complete execution was not applicable because Slice 50 moved no
accelerator path. Inventory and target-coverage gates still covered every test
target. The candidate-bound Python native module SHA-256 was
`fb69d77d4371ad865ac288b78d9f122db82371398e24ae44fe9b1dfefc5bd311`.

All temporary environments, receipts, native/generated artifacts, captures,
and caches were removed. Final tracked Git status was clean and HEAD remained
the exact reviewed candidate.
