---
title: FathomDB 0.8.27 Slice 80 - independent verification
status: PASS
target_release: 0.8.27
candidate: 3e60cc5dd37c8771d607285985337a7f35223aa1
---

# Slice 80 independent verification

An independent, read-only `gpt-5.6-terra` subagent returned **PASS** at clean
candidate `3e60cc5dd37c8771d607285985337a7f35223aa1`. No required evidence was
unavailable.

| Gate | Result |
| --- | --- |
| `scripts/agent-verify.sh` | PASS: 127/127 registered suites. |
| Workspace Clippy with warnings denied | PASS. |
| Workspace Cargo all-target check | PASS. |
| Candidate-bound Python receipt | PASS. Native module SHA-256: `28135e3e51407bb48e6f4d7768221c02dd35ebd65c6c91a7ec2bbf9e0958da41`. |
| Public surface | PASS: 13-row exact capture. SHA-256: `cecec249980a1c57b8a1ad82cfe47b4e33acb7452b7dba0bfad79b4bdf7d3d84`. |
| Hidden surface | PASS: 33-row capture; against the tracked baseline, 261 additions, 0 changes, and 0 removals. SHA-256: `ba06574c78eb78e5fde005ed9ca1d2eb462f4346e3c4a6b17f3e6cc63068951c`. |
| Strict security | PASS: 0 violations, 0 blockers, 0 downgrades, including live AC-037. |
| Feature-complete | PASS: 21 runs, 357 planned, 349 passed, 0 failed, and 8 documented ignores. Summary SHA-256: `833d22cfb694fad97ee7cf9ac7e0a68a161e093c050207c2eb1bfd2fa11d6814`. |
| Release-state views and plan anchors | PASS. |

## Python environment and cleanup

The first Python-receipt attempt found no checkout-owned `.venv` and did not
produce a receipt. It was superseded by a disposable Python 3.12 environment
using a non-editable install. The successful receipt above is the evidence of
record.

The verifier removed the disposable environment and generated artifacts,
restored the tracked `src/python/fathomdb/_fathomdb.pyi`, and confirmed the
candidate tree was clean.

## AC-037 capability ownership

The live AC-037 layer used an already-active host capability. The verifier did
not install, replace, or remove an AppArmor profile it did not own and did not
alter external AppArmor state. Strict security nevertheless exercised the live
layer and passed with 0 violations, 0 blockers, and 0 downgrades.

## Verdict

Slice 80 satisfies AC27-80A through AC27-80H at the reviewed candidate. The
implementation commit remains `8e4499637e9d40ac6fcb9579f352b9f643e86709`;
`3e60cc5d` adds the closed review record without changing production or test
code. Slice 90 may proceed only when separately commissioned.
