---
title: FathomDB 0.8.27 Slice 85 commissioning baseline verification
status: COMPLETE
target_release: 0.8.27
candidate_sha: 4c75bfec2985f4001690673cc5b38dfdce2081bf
verified_on: 2026-09-28
---

# Slice 85 commissioning baseline verification

AC27-85F's entry gate passed on the clean exact candidate
`4c75bfec2985f4001690673cc5b38dfdce2081bf`. This receipt commissions the
reviewed Slice 85 implementation; it does not claim that Slice 85 is
implemented or that the final release candidate is qualified.

## Host prerequisites

- The capture filesystem had 114 GB free after reclaiming generated Cargo
  artifacts; the gate's 100 GB minimum therefore passed.
- Two NVIDIA GeForce RTX 3090 devices reported 24,576 MiB total and 24,123 MiB
  free each. `nvidia-smi` and the feature inventory ran outside the restricted
  sandbox.
- Public capture used Node `v25.9.0`, TypeScript `6.0.3`,
  `cargo-public-api 0.52.0`, and `nightly-2026-04-24` (`rustc 1.97.0-nightly
  (36ba2c771 2026-04-23)`) on `x86_64-unknown-linux-gnu`.
- The candidate worktree had a checkout-owned Python 3.12 virtual environment.
  It used declared development dependencies, a non-editable wheel built from
  the exact candidate, and a candidate-local `.pth` entry for the separate
  `src/python/eval` package. No editable FathomDB install was made from the
  worktree.

## Canonical and native receipts

`./scripts/agent-verify.sh` passed unconfined on the exact candidate. The test
harness reported 125 of 127 registered suites passed, 2 documented skips, 0
failures, and 0 exclusions. The security gate reported no violations,
blockers, or downgrades. This is commissioning evidence only; under AC27-85G
it is not an exact-final-candidate AC-037 qualification, which remains owned
by Slice 150.

The candidate-bound native receipt passed and recorded:

```json
{
  "candidate_sha": "4c75bfec2985f4001690673cc5b38dfdce2081bf",
  "module_sha256": "60be05b662c3502216abdfa0326e71fd9cceaec6e1807d7632ed001a90298790",
  "schema": "fathomdb.python-test-hooks-receipt/v1"
}
```

The ignored receipt's worktree-specific module path and nonce are deliberately
not promoted to durable identity; the candidate and module digests are.

## Official surface captures

The official public comparator captured 13 rows from the exact candidate.
Comparison with `features/slice-30/baseline.json` was exactly equal: no
metadata differences and no row differences.

- Capture SHA-256:
  `002edebe5025ff03ffa50f2fb0cb4cac4a6c2ba249769cf1ecb2600ae5ad7791`.
- Disposable path:
  `/tmp/fathomdb-s85-public-4c75bfec.json`.

The official hidden-surface oracle captured all 33 rows from the same exact
candidate. Relative to the current successor baseline
`features/hidden-surface/baseline-8e2afb29.json`, metadata was equal and the
diff was additive only: 287 test-target or test-inventory entries, 0 removed,
and 0 changed. All eight structural rustdoc rows and the release probe were
unchanged.

- Capture SHA-256:
  `3c4de75e6885666f3e9fd927f597c70923089a970bf0e5e915fb34ece47c0ae2`.
- Disposable path:
  `/tmp/fathomdb-s85-hidden-4c75bfec.json`.

The implementation must compare its final candidate with these pre-move
manifests. If either disposable file is unavailable, recapture this exact Git
candidate with the official tool and require the recorded SHA-256 before any
comparison; do not substitute a later pre-move candidate.

## Commissioning disposition

The independent design review is PASS, owner decision `seq-294` authorizes
execution after these receipts pass, and these receipts passed. Slice 85 is
therefore commissioned and may begin its reviewed RED/GREEN batches. Slice 90,
tagging, registries, and publication remain unauthorized.
