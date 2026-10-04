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
platform and CUDA package rows are not yet qualified. Slice 120 is not unblocked.

## Acceptance

| Criterion | Evidence and disposition |
| --- | --- |
| AC27-110A | PASS locally: Terra reconciled all 465 frozen source items with no unexplained omission. `CounterSnapshot` moved to the frozen `types` owner. The root registers one Engine and keeps feature-gated test exports separate. |
| AC27-110B | PASS for the tested routes: production native runtime has the frozen 17 exports and 44 Engine prototype names; the generated declaration differs only in the accepted subscriber callback signature and removal of `AttachSubscriberOptions`. Test-hook generation adds exactly its expected hooks; a later production build removes them from declarations and runtime. The fresh Linux and Windows installed pairs preserve the package loader and consumer type surface. |
| AC27-110C | PASS locally: the accepted [subscriber ADR](../../../../adr/ADR-0.8.27-typescript-subscriber-delivery.md) is implemented. RED witnesses preceded the callback delivery, replacement race, queue overflow and Windows wrapper fixes. The native suite passes 7/7; NAPI Rust unit tests pass 22/22. A production-artifact writer callback ran on the JS thread before its 8 MB write settled; concurrent close and repeated close settled. Existing FFI panic, conversion and lifecycle suites passed in the full gate. |
| AC27-110D | PASS locally: existing native/SDK validation and FFI tests cover numeric bounds, invalid strings, panic/error conversion and no-mutation refusals. The no-default-embedder artifact rejected `useDefaultEmbedder: true` without creating a database, then opened and closed normally without that option. |
| AC27-110E | **OPEN:** exact-source Linux x64 GNU and Windows x64 MSVC package pairs passed installed runtime, typecheck, surface and contents checks. Linux x64 CUDA/reranker, Linux arm64 GNU/Tegra, and macOS x64/arm64 candidate-bound installed Node rows remain unrun. The local CPU reranker feature compiled, but that is not CUDA qualification. |
| AC27-110F | **OPEN with local gates passing:** gpt-6-sol high code review passed at the product commit; Terra independently verified local source, artifact and Linux package evidence. Strict `agent-verify` passed 182/182 suites, zero skipped or excluded, and security 0 violations/0 blockers/0 downgrades at `06e34759`. Platform completion and final native handoff still control exit. |

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
  `default-embedder,default-reranker` feature typecheck passed. Neither result
  substitutes for the remaining CUDA/reranker installed package row.
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

## Remaining action

Run candidate-bound native/package checks for Linux arm64 GNU on Tegra,
macOS x64 and arm64, and Linux x64 CUDA with selected GPU and reranker, then
record their exact artifacts, toolchains and results. Only after those rows
pass may release state mark Slice 110 complete and advance its `next_slice`.
No tag, publication or deployment was performed.
