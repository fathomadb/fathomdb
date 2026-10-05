---
title: FathomDB 0.8.27 Slice 110 qualification status
status: IN_PROGRESS
target_release: 0.8.27
---

# Slice 110 qualification status

The reviewed product-code commit is
`87670f61d48e6552edcb2a64f7ddb2dacf2863e1` on `release/0.8.27`.
The release-note gate fix at `06e34759f55dbede47b9b7ccb2c000dd5edac9b4`
changes no product code. Slice 110 remains **in progress** because required
Tegra forced-CUDA runtime evidence is not yet qualified. Hosted Linux arm64
GNU and both macOS package rows have passed. A freshly rebooted Jetson repeat
still fails forced CUDA intermittently on the release code. The HITL
authorized Slices 114 and 115 to proceed before Slice 110 closes; those
exceptions do not qualify the Tegra row or unblock Slice 120.

The separate Tegra allocator branch has since advanced to `77d742153` and
reports a successful fallback on the Jetson. It is not merged or qualified on
this release branch. The [integration intake](tegra-integration-pending.md)
records branch checks, review follow-up and remaining merge, CI, ledger and
release-gate work. Slices 114 and 115 are now complete under their separate
sequencing exceptions; Slice 110 remains the live blocker.

## Acceptance

| Criterion | Evidence and disposition |
| --- | --- |
| AC27-110A | PASS locally: Terra reconciled all 465 frozen source items with no unexplained omission. `CounterSnapshot` moved to the frozen `types` owner. The root registers one Engine and keeps feature-gated test exports separate. |
| AC27-110B | PASS for the tested routes: production native runtime has the frozen 17 exports and 44 Engine prototype names; the generated declaration differs only in the accepted subscriber callback signature and removal of `AttachSubscriberOptions`. Test-hook generation adds exactly its expected hooks; a later production build removes them from declarations and runtime. The fresh Linux and Windows installed pairs preserve the package loader and consumer type surface. |
| AC27-110C | PASS locally: the accepted [subscriber ADR](../../../../adr/ADR-0.8.27-typescript-subscriber-delivery.md) is implemented. RED witnesses preceded the callback delivery, replacement race, queue overflow and Windows wrapper fixes. The native suite passes 7/7; NAPI Rust unit tests pass 22/22. A production-artifact writer callback ran on the JS thread before its 8 MB write settled; concurrent close and repeated close settled. Existing FFI panic, conversion and lifecycle suites passed in the full gate. |
| AC27-110D | PASS locally: existing native/SDK validation and FFI tests cover numeric bounds, invalid strings, panic/error conversion and no-mutation refusals. The no-default-embedder artifact rejected `useDefaultEmbedder: true` without creating a database, then opened and closed normally without that option. |
| AC27-110E | **OPEN only for Tegra GPU runtime:** exact-source Linux x64 GNU, Windows x64 MSVC, Linux x64 CUDA/reranker, hosted Linux arm64 GNU, and macOS x64/arm64 installed Node package pairs passed. The Linux x64 CUDA pair computed on a named RTX 3090 and passed driverless CPU fallback and forced-device refusal. On the freshly rebooted Jetson AGX Orin, fresh CPU and CUDA-capable package pairs install and pass their CPU, subscriber and type-surface checks. Forced CUDA succeeded only 1/5 complete clean-process runs and 3/10 focused opens; a later idle-host repeat passed 3/5. Temporary diagnostics locate `CUDA_ERROR_OUT_OF_MEMORY` at Candle's first `Tensor::zeros` after `Device::new_cuda` succeeds. Driver accounting showed about 53.8 GB free across a failure; same-process raw synchronous allocation succeeded while stream allocation/zero failed. The runtime is still intermittent and not qualified. |
| AC27-110F | **OPEN with code/repository gates passing:** gpt-6-sol high code review passed at the product commit; Terra independently verified local source, artifact and Linux CPU package evidence and the later Tegra investigation. Strict local `agent-verify` passed 182/182 suites, zero skipped or excluded, and security 0 violations/0 blockers/0 downgrades at `06e34759`. Hosted platform job receipts are retained. The CI workflow's separate heavy verifier failed its missing-tool preflight before testing, and its self-hosted Windows row remained queued; neither is represented as a green CI conclusion. Tegra GPU runtime and final native handoff still control exit. |

## Candidate-bound artifacts and checks

- Linux x64 GNU: official Node v25.9.0 archive SHA-256
  `1d8db7d6e291d167e8c467ae4094be175e1a0b3969c7ae1f8955b9f7824f7b2e`
  supplied npm 11.12.1; Rust 1.95.0. Production `default-embedder`
  binary SHA-256
  `134812f0189a64a622591a2d54e98c6faaa36f93a9513c0c1b39f98a0a627d27`.
  Fresh main tarball SHA-256
  `de89808dad936a7bfcb3cfb31d5759f79f0a60e7ee722b374807f51f3043c560`;
  Linux platform tarball SHA-256
  `bd58f2849958354eac018e66477e16178f945a5616cd84f4bc424f1562d52c78`.
  External install, subscriber event, reopen, typecheck, exact 17/44 surface,
  no test hooks, and packed file checks passed. The installed binary matched
  the built binary byte-for-byte.
- Windows x64 MSVC: source archive for `87670f61d` SHA-256
  `cb8a2caaae58b76074a7899c55d7deba6723e9786e34f92c387ef25552449ca5`;
  Node v25.9.0, npm 11.12.1, Rust 1.97.1. Production binary SHA-256
  `8df68b6105e7d72b21b3b9d770e1ff02db66bd9d159a87e225b00a7f478d9022`;
  main tarball SHA-256
  `39ea386346a354e020604f52df9a0e81215b543d33624c3770814b5e67780db6`;
  platform tarball SHA-256
  `8ce7ce56f619e055e3327c34c9520893d60762e6ac468f31b1fec7cf9aaa9268`.
  The external installed pair delivered events, typechecked, matched 17/44,
  omitted hooks and passed packed-file checks. The Windows build first exposed
  the `spawnSync npm.cmd EINVAL` defect under pinned Node; its focused RED
  print-plan test and direct local CLI invocation fix are in the product commit.
- The native no-default-embedder release artifact SHA-256 is
  `c4300724b6a95c3c048c4cf5cc28e6a27a7535737453919a9b3f82dd32a6a58a`;
  its opt-in refusal and ordinary open/close passed. The CPU
  `default-embedder,default-reranker` feature typecheck passed.
- Linux x64 CUDA/reranker: the docs-only release HEAD
  `d0551b5466935fe54c1fdc7d2d2e4aa5a09da9cc` built with
  `embed-cuda,rerank-cuda` using CUDA 12.6.68, GCC 13.3.0, Rust 1.95.0,
  Node v25.9.0 and npm 11.12.1. The CUDA native binary SHA-256 is
  `6d3295f6baa3206103d412b3b161aace76a249998a65b3f33c0a625999ff9a92`;
  its platform tarball SHA-256 is
  `8a0fe982d676f671c9c471eb8d53968197a738b3eeb29a6afb8023bc22e9ee3c`.
  Installed-package embedding produced a 384-vector and reranking produced a
  non-null score on selected RTX 3090
  `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b` (driver 580.178.04).
  The allocation witness measured 134,217,728 bytes against a 67,108,864-byte
  floor; the live Node PID appeared in `nvidia-smi` at 476 MiB. Runtime surface
  remained 17/44 with no test hooks and installed binary bytes matched the
  built artifact. The same installed pair passed network-isolated driverless
  CPU embedding/reranking and both forced-CUDA refusal cases. Only the exact
  pinned embedder and reranker model directories were mounted read-only;
  all six model file hashes matched the release contract.
- Hosted Linux arm64 GNU, macOS arm64 and macOS x64: the exact-candidate
  nonpublishing [CI run](https://github.com/fathomadb/fathomdb/actions/runs/37220605280)
  at `366e1bc3d4e4df2a8bb8cd9268ccfdae0e02b08a` passed the native
  build, fresh thin-main/platform-pair install and runtime smoke for all
  three rows. The retained [CI receipt](../../../runs/0.8.27-slice-110-ci/receipt.md)
  links each job and its JSON artifact hash. Native SHA-256 values are Linux
  arm64 GNU `a33f0b0b26df365c45ef1aedf494e3c4836edf35f3609eb30ef3d16680c8966e`,
  macOS arm64 `3679d3d4b4b299748d9e2270fb8395dd7db04d2daf26317298ece4371cd51ac8`,
  and macOS x64 `d156a09c2db0976c57fc109482978b1459924538a3419899e0a715f6b19d9e83`.
- Classic Jetson AGX Orin: the exact candidate's CPU and CUDA-capable NAPI
  artifacts built and installed as matching package pairs. The
  [Tegra receipt](../../../runs/0.8.27-slice-110-tegra/receipt.md) records
  artifact hashes, 17/44 production surface, external typecheck and
  subscriber, CPU embedding/reranking, and the forced-CUDA blocker. A
  temporary diagnostic exposed `CUDA_ERROR_OUT_OF_MEMORY` at the minimal
  Candle probe; sequential fresh-process attempts passed 5/10 on Node 24
  and 3/10 on pinned Node 25, while the prior Python wheel passed 5/5.
  Standalone Candle probes passed 10/10 on main and spawned Rust threads.
  After the host update and reboot, a new exact-head CPU pair and a
  CUDA/rerank pair again built and passed installed-package checks. Forced
  CUDA succeeded 1/5 complete fresh processes and 3/10 open-only processes;
  direct module loading passed 10/10. The successful process produced a
  111,144,960-byte GPU allocation delta above the 67,108,864-byte floor,
  a 384-dimensional embedding, and two rerank scores. A temporary diagnostic
  build with reranking forced to CPU showed `Device::new_cuda` succeed, then
  `Tensor::zeros(1, DType::F32, ...)` fail with
  `CUDA_ERROR_OUT_OF_MEMORY`. Its diagnostic source was restored before the
  receipt was committed. Host RAM was available and no GPU process was
  visible. `nvidia-smi` does not report Orin GPU memory, but a later valid
  CUDA Driver API probe measured about 53.8 GB free across a failed open.
  After Memex CI finished, a controlled idle-host installed-package repeat
  passed 3/5 complete forced-CUDA runs and failed 2/5 at open. No named GPU
  compute process or accessible GPU-device owner was found; system and
  contiguous-memory counters were stable across failures. Same-process raw
  four-byte synchronous CUDA allocation succeeded while Candle's four-byte
  stream allocation/zero path failed. The latter still combines allocation
  and memset, so the exact failing substage is unresolved. Constraining the
  V8 heap did not improve the earlier pass rate. The linked receipt binds
  all three attempts to exact candidate, artifact, and toolchain hashes.
- The complete `./scripts/agent-verify.sh` used
  `CARGO_PROFILE_TEST_OPT_LEVEL=3 AC013_VECTOR_DIM=384` on a ptrace-capable
  executor. The prior 0.8.27 AC-013 plain-debug latency condition is not
  represented as a Slice 110 pass. The generated production native declaration
  SHA-256 is
  `2e8e9f4cb9e64c967ad9bde344aeb3285b3d4fd006416d31083f6d74023d6db2`.

The retained [entry inventory](entry-inventory.md),
[source-owner map](entry-source-owner.tsv), [design review](design-review.md),
and [execution plan](plan.md) bind the planned contract. The final native
signature delta is the one in the subscriber ADR; `close()` still returns
`Promise<void>`. Slice 120 should use these final declarations and the
installed package receipts after the remaining platform rows pass.

The active data-plane architecture, binding and lifecycle designs now explain
the as-built NAPI ownership split, separate Promise and subscriber paths, and
the reasons for the queue, wakeup, close and heartbeat decisions. The
post-implementation design review and its wording correction are recorded in
[design-review.md](design-review.md). This documentation reconciliation does
not change the open platform acceptance rows.

## Remaining action

Obtain reliable repeated installed Node forced-CUDA opens and GPU allocation
witnesses on a reserved Orin. Split the CUDARC stream allocation from its
asynchronous memset, then review any contract-preserving repair against the
actual model path; forced CUDA must continue to refuse rather than fall back
to CPU when unavailable. Hosted Linux arm64 GNU
and both macOS rows are complete. Only after the Tegra row passes may release
state mark Slice 110 complete and advance its `next_slice` to Slice 115,
because Slice 114 completed under the 2026-10-04 HITL sequencing exception.
No tag, publication or deployment was performed.
