---
title: FathomDB 0.8.27 Slice 110 qualification status
status: COMPLETE
target_release: 0.8.27
---

# Slice 110 qualification status

The reviewed NAPI product-code commit is
`87670f61d48e6552edcb2a64f7ddb2dacf2863e1` on `release/0.8.27`.
The release-note gate fix at `06e34759f55dbede47b9b7ccb2c000dd5edac9b4`
changes no product code. The Tegra allocator and early-`cuInit` changes merged
at `a25d063cd`. Slice 110 is **complete**: the clean amd64 repository gate
passed 184 / 184 suites at `825f31cc5`, and the merged installed Jetson
package passed its allocation witness 20 / 20 on a quiet host. Installed
x86_64 Node and Python CUDA artifacts passed the merged-source runtime checks
below. Hosted
Linux arm64 GNU and both macOS package rows have passed. The intermittent
Jetson forced-CUDA refusal was root-caused and fixed on
`llm/slice110-tegra-allocator-fix` (see AC27-110E): an aarch64-Linux
synchronous allocation fallback in the vendored cudarc 0.19.7 plus early
`cuInit` at Node addon load, both ruled into 0.8.27
(`slice-110-early-cuinit-with-allocator-fallback`). Every review round on that
branch passed or closed its findings, and an x86_64 round on candidate
`8b76f6115` passed (see the [integration intake](tegra-integration-pending.md)).
That branch is merged into `release/0.8.27`. The HITL authorized Slices 114
and 115 to proceed before Slice 110 closed; both are complete under those
sequencing exceptions. Slice 120 is next.

> **Revisit obligation (user-directed).** The aarch64-Linux-only cudarc
> allocator workaround, and the early `cuInit` added in this slice,
> must be revisited at the **next micro release** and **loudly at the next
> minor release**. Check four things: whether NVIDIA has fixed the
> default-pool/`cuInit` address-space behaviour in a newer L4T/CUDA; whether
> upstream cudarc gained a fallback or pool API, so the vendored copy can be
> dropped; whether V8/Node changed the ARM64 mmap hint mask; and the planned
> explicit-pool work to recover stream-ordered speed. Tracked as todos-ledger
> `TC-9fef1b7c-4442-4c77-b925-992f338c9aac` (seq 270).

## Acceptance

| Criterion | Evidence and disposition |
| --- | --- |
| AC27-110A | PASS locally: Terra reconciled all 465 frozen source items with no unexplained omission. `CounterSnapshot` moved to the frozen `types` owner. The root registers one Engine and keeps feature-gated test exports separate. |
| AC27-110B | PASS for the tested routes: production native runtime has the frozen 17 exports and 44 Engine prototype names; the generated declaration differs only in the accepted subscriber callback signature and removal of `AttachSubscriberOptions`. Test-hook generation adds exactly its expected hooks; a later production build removes them from declarations and runtime. The fresh Linux and Windows installed pairs preserve the package loader and consumer type surface. |
| AC27-110C | PASS locally: the accepted [subscriber ADR](../../../../adr/ADR-0.8.27-typescript-subscriber-delivery.md) is implemented. RED witnesses preceded the callback delivery, replacement race, queue overflow and Windows wrapper fixes. The native suite passes 7/7; NAPI Rust unit tests pass 22/22. A production-artifact writer callback ran on the JS thread before its 8 MB write settled; concurrent close and repeated close settled. Existing FFI panic, conversion and lifecycle suites passed in the full gate. |
| AC27-110D | PASS locally: existing native/SDK validation and FFI tests cover numeric bounds, invalid strings, panic/error conversion and no-mutation refusals. The no-default-embedder artifact rejected `useDefaultEmbedder: true` without creating a database, then opened and closed normally without that option. |
| AC27-110E | **PASS:** the Tegra GPU runtime fix is reviewed, and the merged installed package passes the witness on a quiet host (20 / 20). Exact-source Linux x64 GNU, Windows x64 MSVC, Linux x64 CUDA/reranker, hosted Linux arm64 GNU, and macOS x64/arm64 installed Node package pairs passed. On the Jetson AGX Orin the intermittent forced-CUDA refusal was traced to the driver's default memory pool, which needs one contiguous 20960 MiB range of process address space inside [8 GiB, 128 GiB); Node heaps fragment it, so stream-ordered allocation failed with `CUDA_ERROR_OUT_OF_MEMORY` while synchronous allocation worked. A vendored cudarc 0.19.7 (`third_party/cudarc-0.19.7`) now falls back to synchronous allocation when that pool cannot be obtained and handles zero-length synchronous buffers. A fragmented-layout regression test failed 6 / 6 before the fix and passed 11 / 11 after. After the first review's fixes it asserts that the default pool is unavailable and that the context allocates synchronously, and it passed 12 / 12. The allocator decision is now made once per device per process, and unrelated pool-query errors are propagated rather than downgrading the device. After the second review the test asserts only on the measured AGX Orin 64 GB (12 / 12). A cached decision was shown to stay valid on the measured Orin: the default pool survived context teardown with an explicit pool kept alive (40 / 40), never created (40 / 40) or destroyed first (20 / 20), and a post-`cuInit` control detected a missing pool (10 / 10); other Jetsons, non-Tegra aarch64 hosts and a co-resident `cuDevicePrimaryCtxReset`/`cudaDeviceReset` are unmeasured. At the user's direction, the fallback now compiles only for aarch64 Linux; every other target compiles upstream's allocator logic unchanged. Fresh forced-CUDA open/embed/rerank/witness processes then passed 100 / 100 (Node 25: 30 in-tree + 30 installed; Node 24 and 26: 10 + 10 each); forced CPU passed 6 / 6. After each review round, a rebuilt artifact passed 10 / 10 in-tree and 10 / 10 installed on Node 25, with forced CPU 3 / 3 in each form. The final Tegra Python wheel passed 10 / 10. `cuInit` itself fails once a large JavaScript heap leaves no 4 GiB hole, so the Node addon built for aarch64 Linux with CUDA now calls `cuInit` while Node registers the module (once per process; skipped when every component built with CUDA has an exact `cpu` policy, or with `FATHOMDB_CUDA_EARLY_INIT=off`), and a later forced-CUDA refusal may name an out-of-memory `cuInit` and remedy when the latest same-kind probe record still holds that outcome. A heap-growth check failed 3 / 3 before and passed after; behind an early import, 50 / 50 heap-heavy processes passed on Node 25, 24 and 26, `node --import fathomdb` rescued a late import 8 / 8, and a late import without it was refused 5 / 5 with the new message. Fix round 3 moved the call from a shared-library constructor to module registration, made the embedder items `#[doc(hidden)]` and unstable, and records the latest `cuInit` outcome per probe kind for a best-effort refusal hint; on the rebuilt addon 50 / 50 heap-heavy processes passed again, 5 / 5 first loads inside a worker passed, `--import` rescued 5 / 5, a late import was refused 5 / 5 with the hint, and the rebuilt Tegra Python wheel passed 10 / 10. The hook adds about 12 ms per import and about 61.5 GiB of reserved virtual address space. Open limitations: a late import into a large heap still fails; the synchronous path is about 1.8–2.8 times slower per steady embed, and early `cuInit` does not avoid it; only the AGX Orin 64 GB was measured. The [Tegra receipt](../../../runs/0.8.27-slice-110-tegra/receipt.md#allocator-root-cause-and-fix) holds the hashes and counts. |
| AC27-110F | **PASS:** the gpt-6-sol high code review and independent Terra verification cover the NAPI source, installed artifacts and final Tegra fix; the source inventory has no unexplained Slice 110 item. The clean amd64 `agent-verify` gate passed 184 / 184 suites at `825f31cc5`, with zero skips or exclusions and security 0 violations/0 blockers/0 downgrades. Merged-source installed packages and hosted platform receipts cover the required routes, including the 20 / 20 installed Jetson witness repeat on the same addon digest. The earlier CI heavy-verifier missing-tool preflight and queued self-hosted Windows job remain historical limits, not green CI claims. Slice 120 receives the final native declarations and package receipts. |

## Candidate-bound artifacts and checks

### Merged-source amd64 qualification (2026-10-05)

The merge commit `a25d063cd` is the tested source. On the local x86_64 host
(CUDA 12.6.68, Node 25.9.0, two RTX 3090s), the official CUDA NAPI build
produced a binary with SHA-256
`b1be1554c9c0c96445d78878eb5f264c8cc5e8226909250a505e064566fd0f42`.
The thin main and Linux x64 platform packages were packed and installed
offline in a separate consumer; the installed binary matched the build.
Five fresh forced-CUDA processes passed open, 384-dimensional embedding,
reranking and close on the selected RTX 3090, each with a 134,217,728-byte
allocation-witness delta above the 67,108,864-byte floor. The main tarball
SHA-256 was `296a4b4157418e3a66fd194e8be45c882d7035d4d06cf9463f7e9d8d1a1092c6`;
the platform tarball was
`daf6912fe267ae9317fcef675053ef3d2dffa7a7c542c5c427800fce9720f1ef`.

The same source built a host-only CUDA Python wheel, SHA-256
`c33cd61d4dea483736060f799c39c90ea5f372f7c4288d65dca158b60c617dc6`,
installed into an isolated non-editable environment. Five fresh forced-CUDA
processes passed embedding and reranking on the same RTX 3090, each with the
134,217,728-byte allocation-witness delta. The installed Node and Python
packages also passed forced-CPU execution and forced-CUDA refusal with no
visible GPU; neither silently fell back to CPU. The merged x86_64 vendored
cudarc allocator suite passed 11/11, including its live GPU round-trip check.
The merged-source `scripts/test-feature-complete.sh` gate passed on this CUDA
host: 21 runs, 361 planned tests, 353 passed, zero failed, eight
allowlisted ignores and zero gate failures. These checks establish
installed-package amd64 behavior and the extra feature combinations. A first
full `agent-verify` attempt passed 182/184 registered suites, including the
Rust workspace suite, but its two Python artifact suites rejected
documentation edits made during the run. On clean commit `33f7601a`, the
Python suite then passed 1,579 tests with 27 skips, and the nonce-bound native
candidate receipt verified against that exact commit and extension SHA-256
`db2f1613947b6e8ab422518e2d02cf25bab5b6cbe9bc1e8946aa7e171167b003`.
The two originally failed suites have clean rerun evidence. The full strict
`CARGO_PROFILE_TEST_OPT_LEVEL=3 AC013_VECTOR_DIM=384 ./scripts/agent-verify.sh`
then passed 184 / 184 registered suites on the clean `825f31cc5` amd64
checkout, with zero skipped or excluded suites. Its security leg reported
zero violations, blockers and downgrades. The later `68f4af943` commit added
only merged-Jetson evidence and status wording; it did not change the tested
product or gate source.

On the AGX Orin, the merged-source CUDA Node addon rebuilt and its installed
package passed five fresh forced-CUDA open/embed/rerank/close processes without
the optional allocation witness. In an earlier witness-enabled repeat, the
first process passed and the second had a negative measured GPU-memory delta
and failed the witness floor. That does not show a CUDA execution failure.
The native addon
SHA-256 was `f66a394c699e1dc7a7894b6e66a995223eff2e2b728007ca0e1de3e1ee9da635`.

The witness row was resolved on 2026-10-06 with the same addon digest, staged
in the installed form. Under the shared GPU lock on a quiet host, 20 / 20
fresh forced-CUDA open/embed/rerank/close processes passed with the witness
on. Their deltas were 140,099,584–171,958,272 bytes, at least 2.09 times the
67,108,864-byte floor. Forced CPU passed 5 / 5. An interference control ran
another process that touched and released 2 GiB of host memory during 5
witness runs. It moved the deltas by about 200 MB, to 338–373 MB. The witness
reads `cuMemGetInfo`, which on this integrated GPU is a system-wide counter.
Its documented precondition is that the run is the sole GPU consumer. So
memory that another process releases inside the window lowers the delta and
can make it negative. The earlier negative delta is attributed to such
concurrent host activity. This is inferred: the activity during that run was
not recorded. The receipt is
[merged-witness](../../../runs/0.8.27-slice-110-tegra/merged-witness/results.md).

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
  Those attempts predate the root cause; the allocator fix, its regression
  test and the 100-process repeat are summarized under AC27-110E.
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
`Promise<void>`. Slice 120 should use these final declarations and installed
package receipts.

The active data-plane architecture, binding and lifecycle designs now explain
the as-built NAPI ownership split, separate Promise and subscriber paths, and
the reasons for the queue, wakeup, close and heartbeat decisions. The
post-implementation design review and its wording correction are recorded in
[design-review.md](design-review.md). All required platform rows are closed.

## Closeout and handoff

The merged-source x86_64 code, installed packages and feature-complete gate
have passed, as has the clean 184 / 184 repository gate. The AGX Orin
allocation-witness measurement is resolved on the merged installed package
(20 / 20 on a quiet host). The accepted operational limits remain: forced
CUDA refuses rather than silently using CPU when unavailable; a late import
into a very large Node heap can still fail at `cuInit`; and the synchronous
allocation path is about 1.8–2.8 times slower per steady embed on the measured
Orin. The final native signatures and installed package receipts hand off to
Slice 120. No tag, publication or deployment was performed.
