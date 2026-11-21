---
title: Slice75 final candidate qualification
status: IN_PROGRESS
slice: 75
target_release: 0.8.30
---

# Slice75 candidate qualification

Nonpublishing qualification executions passed. The final documentation-only main
package and coherent handoff are being finalized. Parent owns canonical closure, final documentation integration,
publication and registry/Pages readback. Astra-medium approved the reconciled
whole plan/design after FIX1 at `c1e55adc6`; direct Sol6.2-high and parent Codex
passed the implementation and scoped recovery controls. Terra independently
inspected actual executions, artifact members, kernel images and applicability.

## Candidate and input convergence

Product/package freeze is `53400f69d29cbc5029f0c759d58a5edb1f76e874`.
The five-target source subject is `6d0e6c37b6cf35e3fecfe93ae3859a32d1b94a8f`;
the intervening delta contains seven packed README corrections, reviewed
qualification controls and parent release records, with no product code,
producer, manifest or dependency/lock change. Later control-only fast-gate
recoveries and parent plan/docs finalization do not replace those subjects.
AxisW is0.8.30; AxisE stays0.8.0. Cudarc
`3509cd6bd83b44204be1880e833474ab0f75e060`, Candle/kernel
`952edb7dcd1e29563acedd485a3c1ce8ef07a85c` and PTX0 remain unchanged.

The full npm main declares all five exact0.8.30 optional packages and
`engines.node = ">=25 <27"`. Held release/Tegra producers are Node25.9.0 /
npm11.12.1; native consumers are Node26.13.1, with Orin26.10.0 separately
measured. Graph contracts/public/generated docs describe actual eligible
bodyFTS/BM25/write-cursor seeding, limit before logical dedup and separate walk;
no embedding/vector/CE or public API change was introduced by those corrections.

## Actual platform, archive and hosted results

| Scope | Actual result and subject |
| --- | --- |
| Five native targets | [CI44920128162](https://github.com/fathomadb/fathomdb/actions/runs/44920128162), exact6d. Linux x64/ARM64, macOS x64/ARM64 and Windows x64 all PASS. Every target executed SDK3/0, CLI0.8.30 diagnostic/immutable integrity, named isolated wheel, strict installed TypeScript, full optional main/platform and separate-process write/reopen with exact body and missing record. All five wheel-size jobs and receipt collector PASS. Linux x64 hostCPU is narrower than the shipping CUDA pair. |
| Native identities | Retained `native-artifact-matrix.json` SHA256 `c1937663580fa7aad222123341d5140289ea08a663f4aa6d1fe3fc663b8583a0` binds all wheel/native hashes, candidate, target and tools. Actual job logs and wheel/tarball/CLI bytes are retained and independently inspected. Windows main LICENSE differs by line endings; remaining43 payloads match. |
| Source archive | Exact6d archive SHA256 `802b116773552702fe3f87461d34bd2ceb30ae794808b53974104b7bd0aa5dfd`. Fresh external SDK and CLI build consumed the archive's lock/configuration, SDK write/reopen PIDs2413012/2413025 passed, CLI exact version and immutable integrity passed; source file-set/lock/configuration stayed unchanged. This proves source consumption, not public registry resolution. |
| Windows WAL/path | Same CI actual `local_canonical_path_does_not_strand_idle_wal_readers` and `unpatched_sqlite_refuses_unsimplifiable_local_verbatim_paths` each1/0; cross-process reader/BUSY controls PASS. Current installed test-hook0.8.30 wheel SHA256 `402b506f35d65d5f6f754d577638e31b7d03c9086fa6f41942cf12008ce84d3c` PASS. The old0.8.22 baseline's exit66 remains a separate observation. SQLite floor assertion and bundled dependency identity are recorded; no invented runtime source ID. |
| Hosted cancellation | [Run44920465232](https://github.com/fathomadb/fathomdb/actions/runs/44920465232) was actually cancelled after READY. Runtime retained250 complete failure lines plus final251, redaction, prior PASS0 and FAIL101; later block started without a result, last suite remained unrun and no final summary was fabricated. Public synthetic literals in GitHub's echoed script prelude are separate from the redacted runtime transcript. |

Raw evidence is retained under `/tmp/fathomdb-0830-slice75-evidence`, including
`native-matrix`, `native-matrix-receipts`, `native-job-logs`, Windows logs,
`archive-consumer.log` and `cancellation-hosted.log`. Runnable controls are
indexed in the existing [experiment README](../../../../experiments/slice70-qualification/README.md).

## Final CUDA and Orin artifacts

The single protected nonpublishing
[rehearsal44922568609](https://github.com/fathomadb/fathomdb/actions/runs/44922568609)
completed SUCCESS at534. Base and additive reranker cohorts consumed their
same-run sealed preflight package bytes; no duplicate producer was substituted.
Normal preflight and rehearsal exercised driverless auto/CPU, typed forced
CUDA refusal, installed positive GPU forwards and CPU nonuse with PID/UUID
binding. Reranker Python and Node each recorded an actual embedder and reranker
forward on the selected RTX3090. The v3 embed+rerank tuple matches the canonical production
`build-cuda-linux-x64-gnu` route, which invokes `--rerank-cuda`. The base cohort
is the rehearsal baseline and is never substituted for shipping-v3 byte proof.

| Subject | SHA256 |
| --- | --- |
| Base manylinux2.28 wheel | `848d8e8fe33fa8f1983388a8c7222dada7b4b7eb426bf4f44b670ed60790e061` |
| Base x64 CUDA npm platform | `1a57dbeaf6b430c6b137ee24757d80e8607ee2884b7d3ee37443a41e388d1816` |
| Base CLI archive | `979b7ae7badfafcb8af50f33ab5d145d02e037ea84167301836970c864f353ec` |
| Shipping-v3 manylinux2.28 wheel | `577a1ff5ba72b405567b9600bbd30f240edd56d3ca687743bf3e3ae9ad0e3b19` |
| Shipping-v3 x64 CUDA npm platform | `170228211cb33b89e27bb3c33a9a1550345a2b9348b03f9da0e2c1c12121dca2` |
| Reranker CLI archive | `f925ad9606c90117ea65a175b23bcd508225e544514efbc66777951a88f72c80` |
| Full release-shaped npm main | `68482cce3e1756224c19685bb66665fad35a7804e7f20f8d6f321b1878166755` |
| Orin0.8.30+tegra wheel | `ac27fdacf7a9398d80bd4b10b64087dafc83dfc858d83c478e2fe9850656afdb` |
| Orin0.8.30 Tegra npm platform | `68bd782a30ab8e58f3d2b9c713de936f533067b3e7290ffff9160ff98ae51648` |

Base Python native member SHA256 is
`65e908441efb7874ac816397719895d530cab693eda46593e8b8a95639827f23`;
base addon is `cdbb932e358de720c064ca5c7f834d4ac9ac857fe9079e47437af87af52e53e2`.
Both contain ten sm75 PTX markers and15 sm75 static cubins. Wheel glibc floor is
2.28; no dynamic CUDA NEEDED dependency is present. Retained CLI host bytes have
actual glibc floor2.34 and15 sm86 static GGML cubins; the doctor path performs
library/device/context/zero-allocation diagnostics, not model/GGML inference.
No CLI model-forward or sm75 kernel-execution claim follows from those bytes.

The original preflight main is an intentionally narrower fixture. Its sealed
witness is untouched. The full main above adds the packed README and all five
optional entries;43 executable/declaration/manifest payloads match the native
matrix main. Actual Node26.13.1 strict installed graph types, networkless
no-driver auto/CPU384-vector/write/search/close, typed forced cuda:0 exit1 and
separate-process persisted/missing checks PIDs16/60 PASS with that full main and
the exact base CUDA platform. A real Node26 GPU forward and both retained CLI
compatible-device doctors selected GPU0 successfully; the owned process then
stopped at18.15seconds when host RAM crossed the6GiB floor. Natural GPU close is
not credited from that stopped run. Its separate2-second close recovery passed
exit0 with an independently observed PID3331004 and empty GPU0 postflight.

The actual shipping-v3 Node26.13.1 consumer separately installed the retained
`170228...` platform above with the full main. Strict installed graph types,
networkless driverless CPU embed/CE, persisted/missing separate-process reopen,
and typed forced embed/rerank CUDA failures (each exit1) PASS. The single positive
v3 row produced384 dimensions and two finite CE scores0.99997683/0.99997595,
explicitly selected CUDA for reranking, bound PID3385933 to independently
observed UUID0, awaited natural close and exited0 in3.79seconds. Postflight had
no compute/graphics actors and unchanged driver580.178.04/CUDA12.6; the lease
was released. Its raw subject is `node26-shipping-v3/gpu-1795280463639093147`.
The earlier base Node26 result remains baseline-only, not shipping-v3 credit.
Terra independently matched the installed shipping-v3 addon
`afe6eab14c2344fe82378b267985a342da3b90f8ec12cbd042f48cd7d0062403`
to the retained tar member: ten sm75 PTX text markers,15 sm75 static cubins,
no dynamic CUDA NEEDED and actual linked GLIBC2.39. That host-addon floor is
separate from the shipping Python wheel's manylinux2.28 floor. Observed samples
showed488.2MiB peak ownedRSS and at least18.98GiB availableRAM; no continuous
between-sample peak or absence claim follows.

Actual Orin maintained local-wheel and Node25 policy consumers PASS, including
CPU/auto/cuda:0 embed/rerank, generic coinstallation selecting Tegra and
Tegra-only selection. Node26.10.0 four real CPU/CUDA embed/rerank rows PASS,
loading addon `1a237950e450c136693dad0288992ee5bf112e66b770ccc0786a5f450f476a62`.
Selected Orin UUID is `GPU-bbbe9f37-7028-556a-930b-54e5f3b67a82`;3GiB private
pools, observed-only peer inventory, owned child RSS and natural close are
scoped controls. No total integrated-GPU memory or between-sample absence claim.
Retained `orin-final-evidence.tar.gz` SHA256
`f2624a7b3cc70eb432d604b829e71006c9e17aecec2dc2bb60d5a9998a8e4651`
matches remote and local. These local packages do not prove public Pages delivery.

Current-pin45-CLS calibration and real shared BGE/CE basic/overlap rows each2/0
completed once under the existing GPU0 lease and sm75/PTX0 subject. Parent
closed70 with those exact results. They are source/numerical evidence, not final
shipping byte credit. No reset/OOM campaign was repeated. GPU1 is foreign.

## Original nonpasses and scoped recovery

Two demonstrated tool defects retained RED commits and separate GREEN with
unchanged behavioral oracles: hidden hosted readiness marker (`83a6c4c5f`)
was repaired using fixed inheritedFD3; mixed genuine0.8.29 wheel/0.8.30 npm
admission (`4c3e42ab5`, original exit0) now fails expected0.8.30/resolved0.8.29
before npm or retention. Windows retention uses the validated wheel collection.

Original renderer audit4high/1moderate/2low became zero with supported
CLI12.2.0, Mermaid12.3.0/parser2.0.1, Puppeteer25.18.1,
chromium-bidi157.0.8090-0 and Zod4.6.7, without overrides. Official paired Chrome
for Testing157.0.8103.78 archive203266906bytes SHA256
`9223832977a13db3646201b1341c1f9cf08afa3ceae92540116d7b0c062364fb`;
actual browser render21/21 and sanitizer PASS. Original sandbox setsockopt
failure remains; unchanged unconfined controls recovered only that scope.

The five-target CI's fast gate ran220 suites:216PASS/fourFAIL. Existing ABI3
negative controls exposed npm metadata reading before wheel validation; moving
that read restored both original diagnostics. Two stale assertions were updated
for approved Node25/26 support and the added artifact retention. Hosted real
preflight lacked its canonical local release ref; CI now materializes only the
exact already-fetched remote commit, refusing a missing baseline. All four
failed scopes PASS locally, including unchanged real preflight12/12; Sol and
parent reviewed the changes. Original aggregate and scoped recovery logs remain.

The heavy job reached its60-minute limit:21 planned,10PASS (including full
workspace Rust36minutes), one declared live-cgroup SKIP, measurement CLI started
without a result and nine later scopes unstarted. It is not a whole hosted green
claim. Unchanged measurement/control/feature-boundary/Python/TS/hermetic/early
cuInit scopes retain50's strict gate and exact recoveries,70's affected Rust/MCP
checks and the fresh five-target installed binding/Orin controls.75 changes no
Python runtime or Rust product implementation; Node metadata/generated-doc and
consumer/workflow changes have their own scoped passes. No broad replay is used
to erase the timeout or earlier original failures.

Other original setup-only failures remain: renderer sandbox startup, absent
owned TS compiler, owned npm cache EROFS pack, Orin maturin mismatch and missing
hardcoded SMI path. Their scoped setup corrections passed. Final Node26 GPU
preflight refused13.43GiB host RAM against the inherited40GiB source-calibration
start floor before dispatching any body. A second admission-only refusal at7.49GiB
against8GiB and subsequent read-only waits are retained; no positive body ran.
A separately reviewed short-consumer
budget uses2GiB ownedRSS and8/6GiB host admission/stop, retaining source70's
40/24GiB and16GiB unchanged; no further relaxation or foreign cleanup is allowed.
An analysis copy of the base CLI initially lacked its archive0755 mode, causing
exit126 before any doctor or Node body; restoring only that exact mode preserved
ELF bytes. The next run completed both doctors and the Node forward, then stopped
with exit-15 at18.15seconds as host RAM fell below6GiB (owned RSS426MiB,
GPU delta445MiB). The parent-approved close-only recovery shortens the unnecessary
observer keepalive20→2seconds; it changes no inference, allocation or PID oracle.

## Handoff to80

Preserve the above subjects, raw original failures and accepted scoped reuse.
Production protected routing, public registry consumers and Pages installation
remain80-owned. The normal Tegra Pages publisher dispatches one same-run Jetson
producer/payload and public wheel+Node readback; it cannot silently consume local
packages as if they were that hosted payload. Retention currently ends at0.8.29;
add the actual published0.8.30 pair after normal delivery. Orin transfer authority
is session-only. Existing Fathom runner installation is
`/home/coreyt/actions-runner/fathomdb`, with its own `_work`; the registered
Fathom runner was offline and no Fathom listener was observed. The unrelated
Memex service is foreign. Existing warm Cargo cache is
`/tmp/fathomdb-0830-slice60/product-artifacts/cargo-home`, Rustup is
`/home/coreyt/.rustup`, Node25.9.0/26.10.0 are under `/home/coreyt/.nvm`, and
CPython3.12 is under `/home/coreyt/.local/share/uv/python/`; the owned source
venv contains maturin1.14.1.80 may use its normal temporary user-process runner
route without changing services or these qualified local subjects. Unsupported physical-device classifier observations remain
explicitly pending in rehearsal manifests; only the available GPU0 is authorized
for current positive qualification, and no unsupported-device or CUDA13 claim is
added. Broad coverage-guided fuzzing and power-loss proof remain0.8.31 work.
