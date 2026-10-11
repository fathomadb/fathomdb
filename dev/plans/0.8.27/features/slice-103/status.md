---
title: FathomDB 0.8.27 Slice 103 completion status
status: COMPLETE
target_release: 0.8.27
---

# Slice 103 completion status

The integrated **code commit** is
`c2e80ff7683fe856a4cf372a088897c3450b0b9a` on `release/0.8.27`.
Track T (Tegra), Track W (Windows WAL), and Track R (owed erasure recovery)
are integrated. Exact-code installed Windows and Jetson wheels, independent
review, and the repository gates qualify that code commit. Slice 110 is next.

## Acceptance and evidence

| Acceptance | Current disposition |
| --- | --- |
| AC27-103A | PASS: the five late 0.8.26 Tegra commits have a changed-file carry/adapt/exclude inventory; Candle pins, lockfile, and override gate agree. See the [Track T receipt](../../../runs/0.8.27-slice-103-tegra/receipt.md). |
| AC27-103B | PASS: the final classic Jetson Orin wheel was rebuilt from integrated `c2e80ff`, installed into a fresh venv, and passed glibc, static linkage, CPU, auto, forced CUDA, and verified in-process allocation checks. |
| AC27-103C | PASS: the guarded Pages route and negative operator tests pass. No Pages workflow was dispatched. |
| AC27-103D | PASS: the exact installed 0.8.26 Windows control produced 6 first checkpoint refusals in 100 trials. The frozen unfixed 0.8.27 candidate reproduced the defect; the [Track W receipt](windows-evidence-receipt.md) distinguishes those controls from the fixed candidate. |
| AC27-103E | PASS: a genuine Windows RED test led to the local-drive SQLite WAL fix. The exact-code installed Windows wheel passed 40/40 no-read, 40/40 read, 20/20 page, 5/5 purge, and operator excision checks with zero first refusals and the required physical-absence, survivor, closure, and write-fence assertions. True network-share WAL remains outside the qualified support claim. |
| AC27-103F | PASS: all three tracks and the combined source passed independent review. Both exact-code installed platform wheels, affected regressions, strict repository verification, and the broader release check passed. |
| AC27-103G | PASS: the [Track R receipt](../../../runs/0.8.27-slice-103-recovery/receipt.md) records the accepted successor ADR, CLI/recovery contracts, eight real-database tests, safe telemetry refusal, and the offline single-connection recovery action. |

## Integrated code qualification

- Independent combined source review: PASS at the integrated code commit.
- Strict `./scripts/agent-verify.sh`: PASS, 182/182 suites, zero skips or
  exclusions, strict security zero violations, blockers, and downgrades.
  Exact [log](../../../runs/0.8.27-slice-103-integration/evidence/agent-verify-c2e.log), SHA-256
  `62f0ef19dd8881b32b8ed460548fe7fd67fc09b53cfbfffea3642fcdf6f49a17`.
- Broader `./scripts/check.sh`: PASS on a ptrace-capable executor with
  `CARGO_PROFILE_TEST_OPT_LEVEL=3 AC013_VECTOR_DIM=384`. These are the
  documented optimized AC-013 performance settings. The 25-test long-run
  group passed 17 tests, with 8 intentionally ignored, in 562.94 seconds;
  TypeScript typecheck and strict MkDocs build passed. Exact
  [log](../../../runs/0.8.27-slice-103-integration/evidence/check-optimized-c2e.log),
  SHA-256 `ba6da3810696f2f1d01d4fc13420171e45de442a1c153fc4fb50d20e4e680cbd`.
  The plain debug-profile invocation exceeds the pre-existing AC-013 latency
  budget on both Slice 103 and pre-Slice-103 source; the canonical isolated
  release-profile AC-013 run passed at 10,000 vectors and 384 dimensions.
- Final Windows wheel built from this code commit after metadata-only
  `0.8.27` staging: SHA-256
  `c372e8fd1fdcd5d46cd80d85443efa2c245a638a85add65dc2d4344fb79348dd`.
  Its installed native module and raw trial log hashes are in the
  [Track W receipt](windows-evidence-receipt.md).
- Final classic Jetson Orin wheel built from this code commit after
  metadata-only `0.8.27+tegra` staging:
  `fathomdb-0.8.27+tegra-cp310-abi3-linux_aarch64.whl`, 9,063,350 bytes,
  SHA-256 `91bed0411962741b9443afb4bd7df5506893d1c2fc06dae8ca1d2afaeb4c3ffb`.
  A fresh installed runtime selected CPU under `cpu`, CUDA under `auto`, and
  CUDA under `cuda:0`; the forced run's verified allocation delta was
  143,622,144 bytes against a 67,108,864-byte floor. The clean installed
  import used `python -I` outside source with `LD_LIBRARY_PATH`,
  `LIBRARY_PATH`, and `PYTHONPATH` removed. The host-native wrapper,
  glibc 2.35 floor, raw `nm`/`readelf`, and fail-closed linkage checks passed.
  The retained [final Tegra manifest](../../../runs/0.8.27-slice-103-integration/evidence/tegra/qualification-manifest.json)
  SHA-256 `ea13d5ca9548959be2d0b7ea3123f7a0ece7e5a1bf4a6e5519cfe841ae9d5e23`
  binds 50 evidence-file hashes and 15 passing checks. The independent
  artifact review passed and the approved remote scratch checkout and bundle
  were removed, as shown by the
  [cleanup proof](../../../runs/0.8.27-slice-103-integration/evidence/tegra/remote-cleanup-proof.log).
  This host
  had a CUDA driver, so driverless import is supported by linkage inspection
  and stripped-path installed import, not measured on a driverless Jetson.
  The witness's sole-GPU-consumer condition could not be externally attested
  through the Jetson's `nvidia-smi` PID list.

The source commit remains the qualification anchor. Later receipt and
release-state commits must be documentation-only; any change to source,
tests, scripts, dependencies, or build inputs requires requalification.
The Slice 103 closeout record was committed at
`a4eb9f6e2a25adefc41d10c6f1cb3f38f3ba5b5c`; the release state records
that bookkeeping SHA separately from the code SHA.
No Pages dispatch, tag, push, registry publication, or deployment belongs to
this slice closeout.
