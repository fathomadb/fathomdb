---
title: FathomDB 0.8.27 Slice 103 qualification status
status: QUALIFICATION_PENDING
target_release: 0.8.27
---

# Slice 103 qualification status

The integrated **code commit** is
`c2e80ff7683fe856a4cf372a088897c3450b0b9a` on `release/0.8.27`.
Track T (Tegra), Track W (Windows WAL), and Track R (owed erasure recovery)
are integrated. The final Windows wheel and repository gates qualify that
code commit. The final host-native Tegra wheel is still required before the
slice can close and the release state can advance to Slice 110.

## Acceptance and evidence

| Acceptance | Current disposition |
| --- | --- |
| AC27-103A | PASS: the five late 0.8.26 Tegra commits have a changed-file carry/adapt/exclude inventory; Candle pins, lockfile, and override gate agree. See the [Track T receipt](../../../runs/0.8.27-slice-103-tegra/receipt.md). |
| AC27-103B | PENDING: the branch-local Jetson wheel passed CPU, auto, forced CUDA, installed import, linkage, and allocation checks, but it was built from `a8c7e2f12`, not the integrated code commit. An exact-code rebuild is required. |
| AC27-103C | PASS: the guarded Pages route and negative operator tests pass. No Pages workflow was dispatched. |
| AC27-103D | PASS: the exact installed 0.8.26 Windows control produced 6 first checkpoint refusals in 100 trials. The frozen unfixed 0.8.27 candidate reproduced the defect; the [Track W receipt](windows-evidence-receipt.md) distinguishes those controls from the fixed candidate. |
| AC27-103E | PASS: a genuine Windows RED test led to the local-drive SQLite WAL fix. The exact-code installed Windows wheel passed 40/40 no-read, 40/40 read, 20/20 page, 5/5 purge, and operator excision checks with zero first refusals and the required physical-absence, survivor, closure, and write-fence assertions. True network-share WAL remains outside the qualified support claim. |
| AC27-103F | PENDING: all three tracks have reviewed evidence and the combined source review passed; the exact-code Windows wheel and integrated Linux gates passed. The exact-code Jetson wheel and its installed checks remain. |
| AC27-103G | PASS: the [Track R receipt](../../../runs/0.8.27-slice-103-recovery/receipt.md) records the accepted successor ADR, CLI/recovery contracts, eight real-database tests, safe telemetry refusal, and the offline single-connection recovery action. |

## Integrated code qualification

- Independent combined source review: PASS at the integrated code commit.
- Strict `./scripts/agent-verify.sh`: PASS, 182/182 suites, zero skips or
  exclusions, strict security zero violations, blockers, and downgrades.
  Exact log `/tmp/fathomdb-s103-integrated-verify-c2e.log`, SHA-256
  `62f0ef19dd8881b32b8ed460548fe7fd67fc09b53cfbfffea3642fcdf6f49a17`.
- Broader `./scripts/check.sh`: PASS on a ptrace-capable executor with
  `CARGO_PROFILE_TEST_OPT_LEVEL=3 AC013_VECTOR_DIM=384`. These are the
  documented optimized AC-013 performance settings. The 25-test long-run
  group passed 17 tests, with 8 intentionally ignored, in 562.94 seconds;
  TypeScript typecheck and strict MkDocs build passed. Exact log
  `/tmp/fathomdb-s103-integrated-check-optimized-unconfined-c2e.log`,
  SHA-256 `ba6da3810696f2f1d01d4fc13420171e45de442a1c153fc4fb50d20e4e680cbd`.
  The plain debug-profile invocation exceeds the pre-existing AC-013 latency
  budget on both Slice 103 and pre-Slice-103 source; the canonical isolated
  release-profile AC-013 run passed at 10,000 vectors and 384 dimensions.
- Final Windows wheel built from this code commit after metadata-only
  `0.8.27` staging: SHA-256
  `c372e8fd1fdcd5d46cd80d85443efa2c245a638a85add65dc2d4344fb79348dd`.
  Its installed native module and raw trial log hashes are in the
  [Track W receipt](windows-evidence-receipt.md).

The source commit remains the qualification anchor. Later receipt and
release-state commits must be documentation-only; any change to source,
tests, scripts, dependencies, or build inputs requires requalification.
No Pages dispatch, tag, push, registry publication, or deployment belongs to
this slice closeout.
