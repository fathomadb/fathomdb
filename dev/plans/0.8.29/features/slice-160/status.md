---
title: Slice 160 local qualification status
status: IN_PROGRESS
target_release: 0.8.29
---

# Slice 160 local qualification status

The first local batch used `d259d84c59e38861de11192cb8764979458fb127` on
`llm/0.8.29-s160-local-20261116`; later rows retain their actual producer revisions. The [design](design.md) selects the
existing two required CPU long-test variants and offline verified leaf packages.
Independent Astra-medium design review accepted this batch before execution.
No source behavior or test oracle changed.

## Completed independent local batch

All commands used Rust1.95, the unchanged locked source above, an owned offline
Cargo cache/target and ordinary CPU execution. Raw logs and archives remain under
`/mnt/data/tmp2/fathomdb-0829-continuation/slice160-local-20261116/`.

| Existing check | Actual result |
| --- | --- |
| Q10 AC021, `--features operator`, `AGENT_LONG=1`, exact selector |1test passed;60-second window,59 projection add/drop cycles,59 rebuilds, zero `SQLITE_SCHEMA` events. `logs/ac021.log`. |
| Q10 cursor race, `AGENT_LONG=1`, exact selector |1test passed;1000 searches,401 committed writes including400 during reads,999 advancing hits; zero row-count/cursor and hit-cursor violations. `logs/cursor.log`. |
| Offline leaf packages, `cargo package --offline --locked --allow-dirty` |All3 archives verify-compiled: embedder-api0.8.0, schema/query0.8.29. Cargo's unused optional fork-patch warnings are retained in `logs/package-*.log`; no verification bypass. |
| Q4 missing AC037 live components |Current locked `security_cycle` rebuilt; both canonical netns engine and demonstrate-the-catch scripts exited0, including the live deliberate-egress catch. `logs/security-build.log`, `logs/check-netns-deny-egress.log`, `logs/check-netns-deny-egress-catch.log`. No host settings changed or optional downgrade selected. |
| Q14 current Linux x64 production consumers |At `b07a161317ff753b86b5eb706e7c5e761adecb9a` (unchanged runtime source), the existing owned Python setup built an abi3 .29 wheel with `pyo3/extension-module,default-embedder`; the existing production N-API route and TS build passed. No test-hooks. The existing local native smoke passed isolated offline Python and matched Node installs, strict installed graph types with TS6.0.3, write/search/dependency/frozen-read and Node graph-evidence controls; Python3/Node3 runtime-configuration cases passed, zero skipped. `logs/production-{python-build,napi-build,typescript-build,installed-consumers}.log`. |
| Q14 current Windows x64 production consumers and WAL |Exact source `50780514e6ba37fff147351eeea077ba65933431` passed production Python/Node installs, strict TS6.0.3 and all13 retained N-API modules:186/186, zero skips. The cp310-abi3 Windows wheel is6876529 bytes and passes the size gate. All10 unique WAL engine controls, real held-reader parent/child and installed baseline/current serial/binding/retained controls pass. Both existing receipts validate actual bytes. Independent Astra-medium final ACCEPT; raw logs/artifacts/WAL results remain under `/mnt/data/tmp2/fathomdb-0829-continuation/slice160-windows-20261116/`. |

| Q14 current Linux ARM64 CPU consumers and floors |Exact clean `fd47561ad7ad778eb8a5125d239d1c438cd19483` source in an owned Orin checkout produced fresh CPU/no-hook Python and N-API artifacts. The existing native ARM manylinux image built the actual2.28 wheel/addon; auditwheel and both symbol-floor checks pass. Unchanged isolated installed Python/Node/strict-TS6.0.3/runtime/dependency/frozen/graph consumers pass, Python3/Node3 runtime cases with zero skips. Existing pinned BGE auto/explicit-CPU384-vector/write/search/close controls also pass inside the guarded networkless container. Raw logs/operators/current package subjects are retained in `arm-results/` under the evidence root above. |
| Q15 current Linux CUDA12.6 distribution floor and driverless controls |At `abcc528944e58b5ef20cb03c27a741bf29bc13b1` (unchanged runtime source), the existing attested manylinux image built the canonical no-hook Python `embed-cuda,rerank-cuda` tuple; matching host N-API tuple built with the pinned GCC13/CUDA12.6 route. Actual `auditwheel show` confirms manylinux2.28; N-API's declared2.39 floor passes. The existing archive inspector checked every packaged ELF member without forbidden CUDA/NVIDIA payload or linkage. Canonical driverless Python/Node stages pass auto/explicit CPU384-vector forwards, real write/search/close and reranker CPU forwards. All4 forced embedder/reranker captures have actual exit1 and pass the existing exact typed-JSON/stderr validators. `logs/{manylinux-cuda-python-build,production-cuda-napi-build,production-cuda-auditwheel,production-cuda-napi-floor,production-cuda-driverless,production-cuda-forced-validation}.log`. |
| Q14 external Rust source-archive SDK and CLI |The archive from `abcc528944e58b5ef20cb03c27a741bf29bc13b1` retains manifests, original lock, Rust sources and active local override configuration. The unchanged existing persistent SDK consumer passes actual `cargo run --locked --offline` write and separate-process reopen, including body/missing-record/close/exit assertions. Locked metadata resolves every consumed product crate inside the archive at W.29/E.8.0. CLI `cargo install --path` with `--locked --offline` passes; unchanged existing version0.8.29, legacy integrity JSON and immutable data-plane/status/file-set checks pass. Subjects/consumer lock/metadata remain in `rust-archive/`; `logs/archive-sdk-minimal-{write,reopen}.log`, `logs/archive-sdk-source-paths.log`, `logs/archive-cli-{install,smoke}.log`. |

The long variants close the missing Q10 observations on these inputs. AC037 closes
the missing live component alongside applicable accepted unchanged security
controls; no aggregate battery or full-workspace green run is claimed. The three
leaf archives are local package preparation, not all shipping artifacts,
Python/Node/platform consumers, complete rehearsal or registry readiness. Reassess
affected runtime/package inputs after90; unchanged observations need no replay.

The Linux wheel is host-native `cp310-abi3-manylinux_2_39_x86_64`, built with
Python3.13.13/Rust1.95; Node25.9 consumed its matched production package. This
runtime observation does not qualify the required manylinux2.28 distribution
floor, CUDA artifacts, all five targets, final90 bytes or registry consumers.
The wheel remains in `tmp/fathomdb-owned-python-3d8ud5k0/` under the raw-log root;
the CPU addon/declarations are preserved in `subjects/cpu-ts/` under that root.

The CUDA wheel remains in `manylinux-out/`; matched main/platform tarballs,
installed forced-refusal subjects and archive-linkage output remain in
`driverless/`. The bounded adapter `driverless-existing-stages.sh` copies only
the existing canonical staging/driverless/refusal stages, preserving their
oracles. Container networking was disabled, no GPU devices or host CUDA-library
mounts were supplied, and pinned BGE/TinyBERT cache guards passed. This closes
these actual floor/linkage/driverless components; GPU-forward/UUID/allocation,
final90 applicability and complete rehearsal remain separate. No full v3 GPU
witness or aggregate release pass was fabricated.

The external SDK owns its downstream lock rather than inheriting the producer's
whole workspace resolution. Initial full-config locked attempts stopped before
execution; raw failures are retained. Isolating its actual bare CPU closure from
unused product-wide Git overrides removed only5 `patch.unused` entries, with no
consumed package identity/version change; real runs retained `--locked`. The
archive's original producer lock and active override configuration are preserved.
This source-archive SDK/CLI proof is separate from registry-resolved `.crate`
consumption and still needs final90 source applicability assessment.

Windows used actual Python3.12.10, Rust1.95 and Node25.9/npm11.12.1.
The verified archive retained all14478 source files and its original lock;
qualified fork Git objects/configuration supplied the unpublished selected pins.
The initial PowerShell5 native-stderr capture failure remains in
`windows-wal-console.log`; the scoped fourth-control and remaining resumption
is in `windows-wal-resume-console.log`. Selectors and marker assertions were
unchanged, with no replay of the first three accepted controls. Existing scratch
was reattached without formatting; selected inactive compiler-cache cleanup
restored the approved C: floor. No A6 replay, product repair, GPU or CI occurred.
This qualifies the named current Windows scope, not final90 or other targets.

ARM execution owns `/home/coreyt/fathomdb-0829-cpu-s160-20261116/` on the existing
Orin. Rust1.95/Node25.9/npm11.12.1 and a new Python3.13.12 venv first produced a
host-native2.34 wheel and passing consumers. The later CPU image
`sha256:a35fc98ecd505e7dde58e3fe6c82f9f77ac6b2bd7e0e95142161ba5e390c73bc`
used Python3.11.15/own maturin1.16, two build jobs and its declared2.28 base;
the successful container supervisor ends with exit0 and no owned actors.
It excludes GPU/nvhost/nvmap devices, host CUDA mounts, Tegra detection files and
all approved GPU-query executables before imports. This is generic ARM CPU
qualification, not a Tegra GPU artifact, non-Tegra ARM+GPU claim or final90 proof.

The earlier host imports have a real execution-constraint exception:
`_coinstall.detect_platform()` automatically launched
`nvidia-smi --query-gpu=name --format=csv,noheader`. Six installed-helper warnings
confirm six classifications between02:32:55.818 and02:33:15.281-0600; the setup
native probe invoked the same hook before02:31:27.623, with stderr captured
internally. Exact successful-query count/timestamps and driver-internal behavior
were not traced. No product CUDA computation/reset was requested; zero GPU probes
is not claimed. E source/environment/native/lease were not modified; final ARM/E
applicability review owns the exception, with no automatic soak retry. The
pre-build supervisor startup KeyError and all host warnings remain in raw logs;
they are distinct from the successful later container execution.

## Remaining qualification

- 20's four Windows rows and the assembled238-suite source gate are complete;
  ordinary smoke/skip behavior remains distinct from the actual long runs above.
- 60/70/80 and100 have accepted selected exits;110/140's selected .29
  source/healthy/OOM/consumer scope is independently accepted.30's remaining
  G/M/D campaigns and50's paired observations remain incomplete; the
  [release state](../../../release-state-0.8.29.json) owns their current progress.
- 40 awaits the actual owner choice; the approved72 model episodes have not run.
  Both revised130E pool-on SDK arms pass; the pool-off arms remain active.90 remains behind the actual Block1 exit.
- Final integrated qualification, five-target installed packages, selected
  CUDA12.6 consumers and the complete local rehearsal still require applicable
  actual inputs. Earlier accepted observations retain their scope; no automatic
  full replay follows document edits.
- After local passes, review CI scripts and execute CI. The earlier blanket CI
  exclusion is superseded. No publishing tags, registry uploads or published
  registry-consumer proof is authorized; readiness and publication remain distinct.

Owner dispositions in [requirements](requirements.md#owner-dispositions) retain
format37 with no migration, reset work in .30, CUDA13 including Tegra in .31,
and non-Tegra ARM+GPU excluded from .29 gates/known-working claims. No private
Memex population census or new capacity/admin ritual blocks this local batch.
