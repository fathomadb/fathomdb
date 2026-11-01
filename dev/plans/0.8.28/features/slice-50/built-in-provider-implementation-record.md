---
title: Built-in provider — implementation and verification record
status: complete provider subunit; Slice 50 scan Unit S remains open
target_release: 0.8.28
observed_on: 2026-11-01
---

# Narrow provider implementation

Owner approval: steward `seq-359`. Reviewed source: `db2baf1ec420dae6680315fcb911ca30e94ddc32`,
from approved contract `2d63cee3c1071dd14f3e1deb44adfddc21672f19`.
Separate [Astra-medium implementation review](../../../runs/0.8.28-provider-contract-review-20261101.md):
APPROVE, no remaining P1/P2 findings. [Closure evidence](../../../runs/0.8.28-provider-contract-output.json)
contains original-to-integrated commit provenance, behavior-specific RED/GREEN
pairs, author checks, corrective history and root verification results.

## Result

- Production engine admission exposes Default/None only. String projection
  provider selectors are rejected; historical metadata stays truthful under
  explicit None. Test/evaluation injection remains behind a separate test API.
- Actual numeric or provider verification refusal returns a classified failed
  open with no Engine, preserving original causes across SDK/bindings and the
  existing cleanup ownership and production budget. There is no automatic fallback.
- Registered first references are confirmed before atomic publication. Explicit
  None validates stored profile metadata, including structured corruption errors.
- Existing cache settings and behavior are preserved: fingerprint inputs,
  persisted marker authority/error semantics, probe gate, restart reuse and
  ordinary 90 population / 45 fresh / 0 cached probe counts. Five fingerprint
  and marker helper bodies match the approved baseline byte for byte.
- Existing Mean pooling, device policy, loader/runtime behavior and deferred
  graph/work semantics remain. Obsolete refused-handle APIs/reports were removed;
  interface docs and ADR supersession reflect the approved contract.

## Verification

| Check | Result |
| --- | --- |
| Strict full run at `c1ac0748cb555420feb86c0cc7860e0ed4e42fff` | 205 suites ran, 195 passed, 10 failed; no excluded/skipped suites. Lint, typecheck and security passed. |
| Seven failed structural/contract-proof suites | Corrected and rerun unchanged: all pass. C1 checker, pins and proof assertions unchanged. |
| Failed Rust compatibility target | 9/9 pass on final candidate with original workspace features; author also ran default 9/9. Original mismatch assertions retained against actual attached providers. |
| Failed Python suite | Final candidate: 1,707 pass, 36 skips; fresh native receipt verified against `db2baf1ec420dae6680315fcb911ca30e94ddc32`. |
| Failed Slice 135 harness | 287 pass, one skip, seven subtests pass after installing this checkout's own noneditable test wheel. |
| Native feature lint and compile closure | Exact PyO3/NAPI feature Clippy, workspace Clippy, default/release compile matrix and external production-consumer isolation pass. |
| Independent review | Separate Astra-medium APPROVE on exact final source, no remaining P1/P2. |

The original whole run returned nonzero. Validation is the completed full run
plus scoped repairs of every failed suite, not a new whole-gate pass at final
HEAD. No full rerun was made for the narrow fixture/manifest/import corrections,
under the owner's gate-frequency policy and independent review assessment.
The original workspace already unified engine test-hooks through binding dev
dependencies; the added engine self dev-dependency keeps mandatory default-test
proofs runnable while normal consumers still cannot import test injection.

## TDD and corrections

The closure evidence records 13 core and 11 binding behavior-specific RED/GREEN
pairs. Two invalid initial core fixture setups were corrected to reach their
intended failure before GREEN; neither initial failure is counted as contract
RED. The withdrawn graph-vocabulary removal pair was reverted and excluded.
Additional passing property/regression coverage is identified explicitly.

Review found one P2: None-profile row decoding lost structured corruption.
Its three real-SQLite RED tests preceded the classification fix. Full verification
also exposed default-test proof gating, a stale root inventory, legacy
provider-free compatibility fixtures and frozen Python surface expectations.
Their corrections retain the original behavioral assertions, exact inventory
checks and immutable Python baseline/hash. Ten Python failures and Slice 135
collection errors were checkout package-installation issues, corrected through
the sanctioned noneditable wheel in this checkout's own environment.
Native-only unused imports were reproduced with strict Clippy and moved into
existing unit-test modules. No checker, contract pin or behavioral oracle was
relaxed. Supplementary gate repairs do not implement a new production policy.

## Remaining scope and limits

Slice 50 remains IN_PROGRESS: scan Unit S and its review/scaling qualification
are not complete. D50-1/D50-2 remain held, not implemented. Slices 60 to 90,
R8/eu7 qualification, release final verification and the 0.8.29 handover have
not been advanced by this subunit. No tag/publication is authorized.

The existing same-identity cached-backend-drift residual remains accepted;
cache admission is not a continuing runtime guarantee. The known provider-free
dense-rebuild invalidation defect remains deferred for separate owner disposition;
this work does not certify that recovery path safe or redesign rebuild.
