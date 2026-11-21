---
title: Slice 60 completion status
status: COMPLETE
target_release: 0.8.30
slice: 60
---

# Slice 60 completion status

The whole [plan](plan.md) and [design](design.md) were implemented and qualified.
Final public-registry delivery passed under owner seq-413; parent owns
canonical closure, integration and worktree cleanup. No work in50/70/75/80 is
claimed here.

## Source and decisions

- Cudarc CODE `3509cd6bd83b44204be1880e833474ab0f75e060` supplies
  `cudarc-fathomdb0.21.0`: remove only the unused marker and preserve the owning
  Rust/build, private allocator, monotonic typed loss and balanced lifetime.
- Candle CODE `952edb7dcd1e29563acedd485a3c1ce8ef07a85c` supplies the four
  `candle-{kernels,core,nn,transformers}-fathomdb0.13.0` packages. It rebases onto
  exact upstream0.11.1 `59f8e903ca80a32ea7f8514b39748fc979e69e82`, preserves
  reviewed K1–K9/K7t deltas and coalesces the consumed kernel source. The old
  separate kernel branch retains its historical baseline, not a consumed pin.
- Adopt exact cudaforge0.1.6, with actual compiler-selection/header invalidation
  RED→GREEN. Preserve prior0.1.5 runs and their narrow selected-payload
  equivalence; quantized/MoE/native75 inspection is not launched support.
- Product and standalone SDK locks, mirrors and sixteen CPU/CUDA target-feature
  graphs name the same final sources: one active cudarc, renamed kernel,
  no old ug driver edge. F32, format37 and support/security policy persist.
- Native-default treatment is **NOT ADOPTED**. The3090 matched campaign passed;
  Orin had seven validated children, eighth body0 but protocolNOTPASS
  (unknown PID and136.911ms sample gap), then32 measured+4 prewarm NOTRUN.
  The prospective rule retains PTX0;75 inherits no native-producer flip.
  Actual size ratios stayed below2x and x86 clean kernel build ratio1.367
  stayed below3x, without rescuing the inconclusive Orin result.

## Verification and limits

Direct Astra plan/design PASS followed one consolidated FIX-1; focused design
changes passed before implementation. Direct Sol source review and Terra actual
runtime verification passed; parent independently reviewed final production
source. Semantic REDs cover actual loader/compiler/graph/tool defects; compile
setup failures, interruption143 and preservation controls are not product RED.

The single strict full product gate on clean frozen runtime source
`f020b84af7b89df4588b69b4f7e558aafeae2f89` exited0 after4263.969s:
241registered,238ran+passed,0failed,3explicit live-controls skipped,0excluded.
Raw receipt/logs are `/tmp/fathomdb-0830-slice60-final-gate/`; passing-run Rust
spill cleanup removed its raw spill, so only available aggregates are claimed.
Later parent metadata merges and qualification documentation do not change this
source result. No broad replay was performed for those changes.

Fresh mandatory final-lock owning CPU controls passed323x86 and322nativeARM,
zero ignored, with exact lock/features recorded under `terra-final-exact-lock/`.
The final normalized no-root-patch external CUDA consumer executed exact F32
`[1,4,9,16]`, tracked pool and post-drop on3090 and driverless CPU. Its staged
archives retain their own hashes; public downloads are a separate delivery proof.

Final installed Python/Node PTX0 consumers exercised real persisted two-row DBs,
model/reranker numerical controls, close and fresh-process recovery on both
venues. CLI proved its real read-only doctor, integrity and dump-count contract,
including the exact retained-terms finding/exit65 on independently confirmed
normal sanitation-dirty fixtures; it has no model-forward credit. Driverless
x86 and maintained masked-signal Tegra linkage tests retain their distinct
scope. Tegra-only artifacts do not prove genericARM thin-SDK lifecycle.
These are actual0.8.29 development candidate artifacts, not future0.8.30
shipping bytes or50's unimplemented module-release API.

Under actual owner411/412, each venue passed seven process-primary reset
children and seven immediate fresh healthy recovery children. Root models
proved2→0 owners, SDK modules3→2 with bounded opaque globals until process exit,
and original guarded clone tests proved final-owner destruction. Orin used its
real3GiB private pool. Inactive-primary synthetic709 remains typed provenance,
not a native709 driver return. All children exited normally and leases freed.
Orin visibility is observed-only; cross-user absence and total-device hard-cap
enforcement remain unproven. No host reset, host configuration change, partial
construction reset or same-process recovery was used.

Ordinary Orin private-pool OOM and the owner-requested single full-card3090
Default OOM retry passed native status2, same healthy generation, cleanup and
retry. Preserve the original3090 8GiB probe breach/NOTPASS and healthy clone
body0/protocolNOTPASS238.325ms. Owner411 accepted that limited healthy witness
as the fault prerequisite; it was not relabelled protocolPASS. The approved
3090 retry observed15,026,159,616GPU bytes,35samples/max32.857ms, exit0.

## Evidence and handoff

Owning requirements, delta inventory, runnable probes and qualification tooling
live in each fork's `fathomdb/` tree. Local raw evidence remains under
`/mnt/data/tmp2/fathomdb-0830-slice60/`; Orin evidence is under
`/tmp/fathomdb-0830-slice60/product-final/` on the established venue.
Selected old failures/skips and original subject identities are retained.
Final source gate, both7+7 fault sets, OOM, installed and graph scopes are done;
no further hardware/full-source run is required. All mandatory selected rows
are satisfied, with the explicit nonadoption and observation limits above.

## Actual public delivery

Owner seq413 approved the Q2/new-kernel-namespace successor and exact five
uploads. Normal `cargo +1.95.0 publish --registry crates-io --locked`
verification and uploads exited0 in dependency order (cudarc used its required
cuda-12060 feature). Existing owned verification caches were reused. Actual
public SHA256 values, separately downloaded and checked against crates.io:

| Package | Version | Public archive SHA256 |
| --- | --- | --- |
| cudarc-fathomdb | 0.21.0 | `6eaa637ddf017eee725e50219745293994400c7b214886bcd5a8b10a4dc18413` |
| candle-kernels-fathomdb | 0.13.0 | `b51dd9c1d22e2c176da122c08e9c674c9548a6d9fdf17834cc5318636cdc8a3f` |
| candle-core-fathomdb | 0.13.0 | `8003006e771c7bb82eb2e1da063feb7d641478ef73a185a238a9576ee3f10346` |
| candle-nn-fathomdb | 0.13.0 | `c197cbfba5f694f177bf8ce6cfd566fd158ec3c66500bfae79ec9b8add1a8072` |
| candle-transformers-fathomdb | 0.13.0 | `93a6665f3454338eb255450bd82e68cf0287fe4469ad00458564b4a48dcf8536` |

Cudarc/kernel public archives equal staged bytes. Other public archives differ
only in generated Cargo.lock; all source and normalized manifests are identical.
Old staged hashes are retained. The isolated public consumer resolves five
normal crates.io packages, one cudarc and no upstream kernel/ug on x86/ARM.
Its200 dependency versions and edges match the qualified staged consumer; only
five sources/checksums changed. CUDA-enabled compilation exited0 (96.821s),
then actual CPU and networkless read-only driverless execution both produced
`[1,4,9,16]` and post-drop exit0. Public binary SHA256 is
`298cf6743b573d42ce1532d320fb78e735feeab049a50aaa9db89d8c1a6ff109`.
Matching earlier GPU source/closure proof remains applicable preservation, not
a relabelled public-binary GPU run. Files: `evidence/public-registry-downloads.json`,
`evidence/public-registry-consumer-*` and `public-registry-consumer/` under the
local evidence directory. Product0.8.30 publication remains outside this grant.
