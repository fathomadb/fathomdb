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

## Typed open-admission fix and current artifacts

Astra-medium source FINAL ACCEPT at `e3762147295cb0e3db3331cedbe04c0c714a7e2c`
preceded this refresh. All four reviewed RED→GREEN commits remain in order;
actual native/source-archive producer is `d2267d1e7bbccf55c543ecb10359df0cf578d817`.
Raws, exact subjects, existing collectors and full digests are under the existing
raw root's `openfix-20261116/`; no prior1349/f668 proof is retagged.

Each payload below passes3 installed Python cases (11 deselected) and3 selected
N-API cases (zero skips): real stored name/revision BLOB and negative-dimension
corruption must preserve CorruptionError kind/stage/hint/anchor, release the
lock on repeated refusal, then reopen/read the persisted body after exact profile
restoration. Maintained selectors are in `test_typed_errors.py` and
`typed-errors.test.ts`; source REDs remain the actual defect evidence.

| New d226 payload | Wheel SHA prefix / addon SHA prefix | Actual artifact boundary |
| --- | --- | --- |
| Linux x64 CPU | `f5b2c7e0b660` / `ff836c4d4ba2` | ABI3 wheel2.28/addon2.39 floors, strict installed TS/package/native digest and existing collector pass. |
| Linux ARM64 CPU | `459e5b2d3d4f` / `4d9d0685e560` | ABI3 wheel/addon2.28 floors, existing isolated lifecycle/strict TS/package checks and collector pass in guarded runc; GPU/Tegra devices, files and query tools excluded before imports. |
| Windows x64 CPU | `81e273ac6a4d` / `178ef3a690d0` | Exact archive/source-hash guards, ABI3/strict installed TS/package/digest/collector pass. Wheel6894398B; delta−505602B versus7400000B, within20971520B budget. |
| Linux x64 CUDA12.6 | `0ad140afdc97` / `3d4478c1089e` | Canonical embed/rerank wheel2.28/addon2.39, ABI3/all-member forbidden-linkage/floors/strict installed TS/digest pass. Driverless auto/CPU BGE lifecycle passes both bindings;2 forced embedder captures actually exit1 and pass exact typed validators. |

Valid counts/token capture still uses the same earlier snapshot; only token-error
observation is deferred behind existing typed profile admission. Writer/ranking/
vector math and shipped Python/TS fronts are unchanged. Accepted focused engine
controls and this successful-path equivalence retain original1349 healthy62/210
and unchanged reranker/provider applicability; those bodies were not replayed.
Historical WAL and earlier rehearsal proofs retain their source/scope. After the
foreign-consumer hold cleared, one fresh exclusive900s+20s GPU batch exits0 on
these exact new digests. Self16GiB/250GiB, physicalFD9/inode8532374/flock,
UUID `GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`, driver580.178.04/24576MiB and
compute-empty entry checks pass. Allocation Python628408→namespacePID1 and
N-API637651→PID1 each witness134217728B>=67108864B. AC082d Python positive645195/
CPU646763 and N-API positive647172/CPU648806 all close/exit0 with certain cleanup;
positive PIDs bind the selected UUID, CPU negatives report compiledtrue/effective
CPU/selectednull with no GPU PID/CUDA access across full42/73 trace files. Positive
traces21/52 retain their separate controls. All owned actors/containers are gone,
final compute is empty and independent same-inode flock/release passes. Actual
proof is in `gpu-allocation-openfix/`, `ac082d-openfix/` and corresponding logs;
prior foreign observations and old1349 GPU inputs remain separate. No sharing,
reset, foreign-actor action or shared D allocation/ledger change occurred.

The new source archive retains its producer lock; the locked downstream SDK
resolves all6 product crates inside it at W.29/E.8.0. Actual write/separate-process
reopen/body/missing/close/exit and locked offline CLI install/version/integrity/
immutable file-set controls pass0. `rust-archive/`, `logs/archive-*` and retained
new executables supply actual identities. Windows' initial uninvoked extraction
attempt remains ineligible; wrong-name tar failures were recovered by copying
the actual platform tarball and validating required bytes, without test replay.

The retained7d release binary SHA `d854615cc2235410bcb361ee80445a92bf2cfe32a3aa608cc2007f26152a1fc1`
first failed760.621/s and its single initially authorized resumption failed
993.996/s<1000 (59640 commits/60.000239089s, exit101). Both remain recorded.
Following the owner's busy-host question, one fixed OLD686d→NEW7d comparison
used unchanged SHA-verified executables, one common environment/TMPDIR/serial
arguments, no debugger/code/rebuild/affinity change, and contemporary1s host/
process context. OLD `d34f6487dbe1d8f4b4aa7d84febcf551493e7a802491e77e4d3082101a2077b4`
passes1495.233/s; current NEW passes1482.090/s>=1000. Complete process CPU/wall
is OLD39.20user+19.15system/65.13s and NEW39.49+18.96/65.13s. The observed pair
is within1%; one pair does not establish statistical causality or explain the
historical993 result. Historical per-run load/IO/cgroup snapshots were absent;
later10-minute system-SAR context must not be presented as exact run context.
The instrumented765 diagnostic remains attribution-only. No statement-cache
production optimization is selected; d226 shipping bytes remain the candidate.
Raw unchanged invocations/time-v/1s CPU, queue, IO, pressure, frequency/temperature,
TID placement/scheduler observations and actual UTC/local bounds remain in
`ac011-fixed-host-pair/`; all recorder/native actors terminated.

Separate independent still-unrun rows ran once: b429.322/s>=100 PASS; registered
AC081 seven-process PASS with sequential324.699516–345.138117ms<=500 and concurrent
47.880991–50.739633ms<=100, retaining all7 sequential200ms warnings and no retired
ratio verdict. Retained operator/AGENT_LONG subjects pass AC02160s/60 cycles/
60 rebuilds/zero schema warnings and cursor1000 searches/212 writes/211 during
reads/999 advancing hits/zero both violations. Actual7d test-binary inputs remain
explicit; valid-profile paths do not execute the admission-error correction.
See `logs/ac011a-resume.log`, `logs/{ac011b,ac021,cursor}-first-current.log` and
`ac081-seven/`. Original1538.787/760.621/993.996 observations are not replaced.

Owner-approved temporary24GiB live ballooning of the shared Windows VM left
persistent32GiB configuration/services/disks unchanged; settled guest free was
20606368KB with no active workers/builds. Actual host memory was reported without
an inferred8GiB gain. **32GiB live restoration remains due** in
`openfix-20261116/runtime-state.json`; parent holds restoration until remaining
native work ends. Mac/hosted/registry and combined readiness remain separate;
the ordinary current throughput pass retains earlier failures. Any subsequent production remedy needs affected new bytes;
these d226 observations retain their actual input scope.

## Earlier full90 artifacts and original throughput failure

Independent Astra-medium design ACCEPT at `e1323ba0c` preceded this batch.
Actual no-hook native producers are
`1349e675c4f82a7589a05ca2043577c6bf92e1a2`, including accepted full90 source.
The reviewed operator-only test cfg correction was cherry-picked at
`f6686e783c0f37ea23e0892cd8593b7d4b9392b5`; it changes no production native input.
The archive SDK/CLI source includes that correction. Pending-publication notes
and the subsequent local rehearsal use `7d03502255d2ea50ee43cc845a85ba47354a706e`.
Native subjects retain their actual producer identity; none is retagged.

Raw commands, original failures and subjects remain under
`/mnt/data/tmp2/fathomdb-0829-continuation/final-artifacts-20261116/`.

| Existing obligation | Actual earlier1349/f668/7d observation |
| --- | --- |
| Q14 Linux x64 CPU | Fresh production/no-hook manylinux2.28 Python wheel and declared2.39 addon pass auditwheel/symbol floors, isolated installed Python/Node/strict-TS6.0.3/runtime/dependency/frozen/graph controls. Unchanged reporting/snippet oracles pass62 installed Python tests and15 N-API modules/210 tests, zero skips, including real pinned BGE UTF-8 aggregate/scored properties. Guarded networkless CPU auto/explicit forward/write/search/close controls pass both bindings. |
| Q14 Linux ARM64 CPU | Fresh production/no-hook wheel/addon both meet actual manylinux2.28 floors; the same isolated consumers,62 Python and210 N-API tests and pinned BGE CPU forwards pass. The guarded runc image excludes GPU/nvhost/nvmap devices, CUDA mounts, Tegra detection files and GPU-query executables before imports. No repeat of the historical ARM host-query exception occurred. Actual image a35fc98e uses Python3.11.15/Rust1.95/Node25.9/npm11.12.1. Own remote source is clean; raws/subjects are retained in arm-results. |
| Q14 Windows x64 CPU | Exact transferred source passes production wheel/addon builds, isolated installed Python/Node and strict TS6.0.3. Original13 retained modules pass187 tests;2 reporting/snippet modules add23, all210 pass with zero skips. Installed Python62 tests, including pinned offline BGE properties, pass. Wheel6893894B is506106B below the7400000B baseline and passes the20971520B delta budget; the existing collector passes. Python3.12.10/Rust1.95/Node25.9/npm11.12.1. Raws/subjects are in windows-results. Historical WAL mechanism proof retains its original scope/source; it was not replayed or relabeled. |
| Q14 archive SDK/CLI | Fresh exact-source archive includes the reviewed test cfg correction and original producer lock. A separate downstream SDK lock resolves all consumed product crates inside the archive at W.29/E.8.0; real write, separate-process reopen/missing-record/close/exit and the unchanged public SDK coverage test pass. Actual installed process exits are0. CLI locked offline installation and unchanged version/legacy-integrity/immutable data-plane/file-set assertions pass. Qualified override configuration is retained separately in cargo-home. No registry-consumer claim. |
| Q15 CUDA12.6 | Fresh canonical embed/rerank wheel2.28/addon2.39 pass actual floors and every packaged-member forbidden-linkage check. Driverless auto/explicit CPU BGE/write/search/close and TinyBERT forwards pass; all4 forced embedder/reranker captures have actual exit1 and pass exact typed-JSON/stderr validators. Current subjects are in cuda-manylinux-out and driverless. |
| Q15 allocation and AC082d | Fresh exclusive FD9 batch exits0. Python/N-API allocation witnesses each bind actual host PID→namespacePID1 to the selected3090 UUID and measure134217728B above67108864B floor. Both separate CUDA-positive/explicit-CPU-negative pairs pass full trace/loader/lifecycle checks; CPU42 Python/73 Node traces prove no CUDA access or GPU PID, compiledtrue/effectiveCPU/selectedUUIDnull. Positive PIDs are observed on the selected UUID. All4 consumers close/exit0 with certain cleanup; owned containers are gone, final compute PIDs empty and independent physical-inode8532374 flock/release succeeds. No reset or shared measurement allocation change. |
| Q17/Q18 notes and local rehearsal | Substantive .29 notes explicitly retain pending qualification/publication, format37/no migration and actual support limits. The first missing-heading failure is preserved. At7d035022 steps1–4 pass; step5 originally exits101 because Cargo was offline. Only its3 canonical leaf publish dry-runs resume with registry reads enabled and exit0, each explicitly aborting upload due to dry run. This is assembled local validatable-subset completion, not an uninterrupted wrapper pass, hosted rehearsal, dependent-crate publication or registry consumption. |
| Q5 original AC011a failure | Current release binary/7d035022 canonical5s warmup/60s measurement fails at760.621 commits/s<1000,45638 commits/60000ms, actual exit101. Strict stop leaves AC011b/AC081/current long controls unrun. Original raw final-ac011.log and binary identity remain. No product cause or waiver is established; see below. |

The failed AC011a wall interval is11:28:38–11:29:44 CST. This lane's last needed
compile ended11:21:39; the parent's owned workspace SQLite tests were still
active. Static call-site inspection finds new capture only in open (before the
warmup) and explicit generation/mutation-status reads, neither called by the
timed unconfigured-kind write loop. Direct writer/worker/commit files are
unchanged from686d. This does not attribute the failure or qualify the current
throughput gate. The initially authorized resumption failed993.996/s; the later owner-directed
fixed comparison and current unchanged1482.090 pass are recorded above. No
altered threshold or production remedy is selected.

The initial ARM shallow-history bundle prerequisite mismatch and interrupted
oversized Windows pre-build cache copy remain operational failures, not product
REDs. Only inactive owned Windows static cache was removed to restore the
existing8GiB C: entry floor; subsequent caches/builds use private R: paths.
No runtime test oracle, product behavior, database format or model input changed
in this qualification batch. Additional Q3 feature joins and the final broad
source gate retain their parent-owned actual outcomes.

## Historical independent local batch

All commands used Rust1.95, the unchanged locked source above, an owned offline
Cargo cache/target and ordinary CPU execution. Raw logs and archives remain under
`/mnt/data/tmp2/fathomdb-0829-continuation/slice160-local-20261116/`.

| Existing check | Actual result |
| --- | --- |
| Q3 normal feature campaign and metadata correction |Actual source686d ran30 feature groups/230 planned:207 passed,0 runtime failures,23 ignored. Original EXIT1 contains2 matrix drifts+8 exact manual-ignore metadata errors. Reviewed5176860 corrected canonical derived sets/manual classifications and exact real-library test-ID validation with genuine RED/GREEN; existing scanners revalidated the same immutable30 logs without replay. Current root static coverage passes403 targets/44 gate-only.12 unchanged opt-in exclusion listing-freshness checks retain original no-stale evidence only. Independent Astra SOURCE+ASSEMBLEDQ3 ACCEPT; original EXIT1/interrupted metadata EXIT143 retained. Normal feature scope, no ignored-as-pass/manual fault or final90 claim. |
| Q5/Q6 current release throughput and latency |Clean686d/binary `perf_gates-368e3e9a107a2e61` (actualdefault/test-hooks) passes AC011a1538.787/s>=1000 andb268.069/s>=100 with unchanged5s warmup/60s measurement. AC012N10000/1000samples p50/p991/3ms; AC013N10000/dim768/1000samples63/73ms. Independent Astra ACCEPT; full raw `logs/ac011-helper-red.log` and `logs/ac01{2,3}-release-binding-current.log`. Planned-RED filename/expectation wrapper is not a failed scientific gate; helpers were correct and unchanged. DCPUload disclosed; no concurrent builds or CPU-isolation claim. |
| Q9 current release seven-process reader observations |Same686d/binary368: all7 fresh processes pass existing500/100ms hard limits, each1600 sequential+1600 concurrent/eight-reader calls. Sequential337.851121–344.381150ms retains7 warning outcomes; concurrent46.958327–52.513726ms. Independent Astra ACCEPT; original AC081/AC020 raw logs retained. Separate AC081c unchanged-debug correctness scope is reused, never relabelled a release timing. |
| Q7 real-corpus stress with original EU7 failure retained |Clean686d/CUDA12.6 compute86 binary `eu7_real_corpus_ac-29e89ed460e117ce` uses provided13JSONLs/96704727B and defaultN1000/7667,100queries/1000bootstrap/1000latency/8x250stress. Stress p9943ms<=150 and46ms<=316 pass independently reviewed AC073 scope. Original overall exit101 and legacy recall ci_hi.897/.801<.90 remain failed/unqualified; no waiver or replay. Current TC5F/Q8 recall remains separate. `eu7-release-cuda86-current.json` and `logs/eu7-release-cuda86-current.log`. |
| Q10 AC021, `--features operator`, `AGENT_LONG=1`, exact selector |1test passed;60-second window,59 projection add/drop cycles,59 rebuilds, zero `SQLITE_SCHEMA` events. `logs/ac021.log`. |
| Q10 cursor race, `AGENT_LONG=1`, exact selector |1test passed;1000 searches,401 committed writes including400 during reads,999 advancing hits; zero row-count/cursor and hit-cursor violations. `logs/cursor.log`. |
| Offline leaf packages, `cargo package --offline --locked --allow-dirty` |All3 archives verify-compiled: embedder-api0.8.0, schema/query0.8.29. Cargo's unused optional fork-patch warnings are retained in `logs/package-*.log`; no verification bypass. |
| Q4 missing AC037 live components |Current locked `security_cycle` rebuilt; both canonical netns engine and demonstrate-the-catch scripts exited0, including the live deliberate-egress catch. `logs/security-build.log`, `logs/check-netns-deny-egress.log`, `logs/check-netns-deny-egress-catch.log`. No host settings changed or optional downgrade selected. |
| Q14 current Linux x64 production consumers |At `b07a161317ff753b86b5eb706e7c5e761adecb9a` (unchanged runtime source), the existing owned Python setup built an abi3 .29 wheel with `pyo3/extension-module,default-embedder`; the existing production N-API route and TS build passed. No test-hooks. The existing local native smoke passed isolated offline Python and matched Node installs, strict installed graph types with TS6.0.3, write/search/dependency/frozen-read and Node graph-evidence controls; Python3/Node3 runtime-configuration cases passed, zero skipped. `logs/production-{python-build,napi-build,typescript-build,installed-consumers}.log`. |
| Q14 generic Linux x64 CPU wheel2.28 |At `84d8f4514a8fdef034a3cf31a1353f65a7dbdaa4` (unchanged runtime source), the existing attested manylinux image built a separate CPU-only/no-hook wheel with `pyo3/extension-module,default-embedder`. Actual auditwheel and extracted-module2.28 symbol-floor checks pass. Paired with the preserved CPU addon bytes, unchanged isolated Python/Node/strict-TS6.0.3/runtime/dependency/frozen/graph consumers pass, Python3/Node3 runtime cases with zero skips. Existing pinned BGE auto/explicit-CPU384-vector/write/search/close controls pass in clean networkless/runc images without GPU devices or CUDA-library mounts. `logs/cpu-x64-manylinux-{build,floor,installed,forwards}.log`. |
| Q14 current Windows x64 production consumers and WAL |Exact source `50780514e6ba37fff147351eeea077ba65933431` passed production Python/Node installs, strict TS6.0.3 and all13 retained N-API modules:186/186, zero skips. The cp310-abi3 Windows wheel is6876529 bytes and passes the size gate. All10 unique WAL engine controls, real held-reader parent/child and installed baseline/current serial/binding/retained controls pass. Both existing receipts validate actual bytes. Independent Astra-medium final ACCEPT; raw logs/artifacts/WAL results remain under `/mnt/data/tmp2/fathomdb-0829-continuation/slice160-windows-20261116/`. |
| Q14 current Linux ARM64 CPU consumers and floors |Exact clean `fd47561ad7ad778eb8a5125d239d1c438cd19483` source in an owned Orin checkout produced fresh CPU/no-hook Python and N-API artifacts. The existing native ARM manylinux image built the actual2.28 wheel/addon; auditwheel and both symbol-floor checks pass. Unchanged isolated installed Python/Node/strict-TS6.0.3/runtime/dependency/frozen/graph consumers pass, Python3/Node3 runtime cases with zero skips. Existing pinned BGE auto/explicit-CPU384-vector/write/search/close controls also pass inside the guarded networkless container. Raw logs/operators/current package subjects are retained in `arm-results/` under the evidence root above. |
| Q15 current Linux CUDA12.6 distribution floor and driverless controls |At `abcc528944e58b5ef20cb03c27a741bf29bc13b1` (unchanged runtime source), the existing attested manylinux image built the canonical no-hook Python `embed-cuda,rerank-cuda` tuple; matching host N-API tuple built with the pinned GCC13/CUDA12.6 route. Actual `auditwheel show` confirms manylinux2.28; N-API's declared2.39 floor passes. The existing archive inspector checked every packaged ELF member without forbidden CUDA/NVIDIA payload or linkage. Canonical driverless Python/Node stages pass auto/explicit CPU384-vector forwards, real write/search/close and reranker CPU forwards. All4 forced embedder/reranker captures have actual exit1 and pass the existing exact typed-JSON/stderr validators. `logs/{manylinux-cuda-python-build,production-cuda-napi-build,production-cuda-auditwheel,production-cuda-napi-floor,production-cuda-driverless,production-cuda-forced-validation}.log`. |
| Q15 actual current CUDA pair GPU allocation and explicit CPU non-use |Retained abcc Python/N-API artifacts pass both existing GPU allocation/PID/UUID stages:128MiB model delta above64MiB floor and valid raw allocation controls. Tool686d then passes both AC082d CUDA-positive/explicit-CPU-negative pairs: complete42Python/73Node CPU trace files plus loader output, no CUDA access, CUDAcompiledtrue/effectiveCPU/selectedUUIDnull; positive children prove their own selected3090 PID. All four BGE384/write/search/TinyBERT/close lifecycles exit0, cleanup certain, final compute PIDs empty and physical FD9 released. Independent Astra-medium final ACCEPT; `gpu-allocation-abcc-current/`, `ac082d-abcc-current/` and corresponding logs remain under the evidence root. Final90 applicability remains open. |
| Q14 external Rust source-archive SDK and CLI |The archive from `abcc528944e58b5ef20cb03c27a741bf29bc13b1` retains manifests, original lock, Rust sources and active local override configuration. The unchanged existing persistent SDK consumer passes actual `cargo run --locked --offline` write and separate-process reopen, including body/missing-record/close/exit assertions. Locked metadata resolves every consumed product crate inside the archive at W.29/E.8.0. CLI `cargo install --path` with `--locked --offline` passes; unchanged existing version0.8.29, legacy integrity JSON and immutable data-plane/status/file-set checks pass. Subjects/consumer lock/metadata remain in `rust-archive/`; `logs/archive-sdk-minimal-{write,reopen}.log`, `logs/archive-sdk-source-paths.log`, `logs/archive-cli-{install,smoke}.log`. |

The long variants close the missing Q10 observations on these inputs. AC037 closes
the missing live component alongside applicable accepted unchanged security
controls; no aggregate battery or full-workspace green run is claimed. The three
leaf archives are local package preparation, not all shipping artifacts,
Python/Node/platform consumers, complete rehearsal or registry readiness. Reassess
affected runtime/package inputs after90; unchanged observations need no replay.

The initial Linux wheel is host-native `cp310-abi3-manylinux_2_39_x86_64`, built
with Python3.13.13/Rust1.95; its runtime observation retains that scope. The later
CPU-only wheel above supplies the missing2.28 qualification without substituting
the separately qualified CUDA wheel. The original wheel remains in
`tmp/fathomdb-owned-python-3d8ud5k0/` under the raw-log root;
the CPU addon/declarations are preserved in `subjects/cpu-ts/` under that root.

The generic x64 CPU validation wheel remains in `cpu-x64-manylinux-out/`, built
with Rust1.95/maturin1.14.1/Python3.11; the preserved addon/declarations were reused
unchanged through `cpu-x64-ts/`. The combined clean-container consumer used
Node25.9/npm11.12.1 and the existing immutable Python3.13.13 base. Model-control
packages/caches remain in `cpu-x64-forwards/`. The contract in
`scripts/release/glibc-floor-contract.sh` and public compatibility docs requires
Python2.28 on both architectures and ARM npm2.28; x64 npm delivery remains the
CUDA host family at2.39. No generic x64 addon2.28 gate or new publication route is
invented. These current CPU proofs do not establish all five targets, final90
bytes or registry consumption; ARM `fd475` subjects and CUDA artifacts remain
separate and unchanged.

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
is not claimed. E source/environment/native/lease were not modified; independent final ARM/E
review accepted the disclosed scope under pre-run seq390, with no soak retry. The
pre-build supervisor startup KeyError and all host warnings remain in raw logs;
they are distinct from the successful later container execution.

## Remaining qualification

- Earlier20 Windows rows and the assembled238-suite source gate retain their
  actual source scope. Current integrated Q2/Q3 outcomes remain parent-owned;
  ordinary smoke/skip behavior stays distinct from the actual long runs above.
- 60/70/80 and100 have accepted selected exits;110/140's selected .29
  source/healthy/OOM/consumer scope is independently accepted.30's remaining
  G/M/D campaigns and50's paired observations remain incomplete; the
  [release state](../../../release-state-0.8.29.json) owns their current progress.
- 40 conditional disposition is complete with no new treatment; the exact approved72 model episodes remain parent-owned
  integrated exits. Selected130 D/E is independently accepted; full90 source is
  independently accepted under seq392 and is integrated.
- Current affected d226 three-target CPU, CUDA12.6 driverless and source-archive
  proofs and exact new CUDA GPU witnesses are recorded above. Mac targets
  remain open; current Q2/Q3/model status is parent-owned. Original
  throughput failures remain alongside the current ordinary pass. Combined
  local readiness is not claimed. The local
  rehearsal is assembled-qualified; its distinct hosted counterpart remains.
  Earlier observations retain original scope; no automatic full replay follows
  document edits.
- After local passes, review CI scripts and execute CI. The earlier blanket CI
  exclusion is superseded. No publishing tags, registry uploads or published
  registry-consumer proof is authorized; readiness and publication remain distinct.

Owner dispositions in [requirements](requirements.md#owner-dispositions) retain
format37 with no migration, reset work in .30, CUDA13 including Tegra in .31,
and non-Tegra ARM+GPU excluded from .29 gates/known-working claims. No private
Memex population census or new capacity/admin ritual blocks this local batch.
