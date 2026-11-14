# Slice 120 design

Proposed implementation contract, inspected at source commit
`a00bf9b9228318e7c236148551bb188f63ff66ee`. Paths below are
repository-relative unless explicitly marked cache source. No execution claims.

## 1. Existing implementation and actual work

| Seam | Existing behavior | Required delta |
| --- | --- | --- |
| Python `src/python/pyproject.toml`, `fathomdb/__init__.py`, `_coinstall.py` | One `fathomdb._fathomdb`; guard runs before native import. Provider scan deduplicates names, so two same-name dist-info directories are not detected. First matching version can hide stale metadata. | Generate markers for both installed x86 variants and cu130 local version in isolated staging; inspect provider metadata instances and active package origin; reject duplicates/mismatches before native load, then compare native release version and compiled CUDA identity before any provider work. |
| Node `src/ts/src/platform.ts`, `binding.ts` | Local `.node` is loaded first; Tegra host/presence selection is separate; generic package has no explicit selected-version check. | Parse explicit selector before any local require; select package using metadata only; exact selected-version check and no fallback; verify private compiled identity after mapping, before model/device work. |
| `scripts/release/cuda-artifact-contract.sh`, build/rehearsal helpers | 12.6 constants, kernel cap75, manylinux2_28 wheel, host-native Node; reranker tuple is separately explicit. | Add selected cu130 axes without rebinding old 12 variables; extend existing validators/inspection/PTX/rehearsal inputs, not a parallel evidence stack. |
| `fathomdb-embedder/src/cuda_libraries.rs` | Required symbol sets, typed refusal and actual loaded-file reports already exist. Missing/missing-symbol remedy is hardcoded to CUDA12. | Parameterize remedy from compiled cudarc CUDA identity; preserve the exact 12 wording and existing public error kinds. Use compiled identity for diagnostics instead of literal12 fallback. |
| Cargo consumption | Registry Candle fork0.11.0/cudarc-fathomdb0.19.10; only candle-kernels is git-patched. | Keep qualified pins unless a concrete repair requires 140. External Rust root must reproduce the kernels patch; published transitive dependencies do not inherit this repository's patch. |

Cache sources inspected: registry `candle-{core,nn,transformers}-fathomdb-0.11.0`,
`cudarc-fathomdb-0.19.10`; git `candle-kernels` at
`182ceab0547348993b6dbec4b1aed610dcc8b23f`; lock-resolved cudaforge0.1.5 (not
its manifest's caret lower bound0.1.2). The fork checkout's current HEAD is not
the consumed source. Cargo.lock also contains upstream cudarc through a different
closure; use the existing single-cudarc selected-build check, not lock-name count.

## 2. Artifact identities and selection

| Surface | 12 default | Proposed 13 route |
| --- | --- | --- |
| Python | Existing distribution/version/channel | Same distribution `fathomdb`, local version `0.8.29+cu130`, import `fathomdb`; exact URL + SHA256 from approved first-party CUDA channel. No PyPI local-version assumption. |
| Node | Main `fathomdb@0.8.29` and `fathomdb-linux-x64-gnu@0.8.29` | Same main package, manually installed `fathomdb-linux-x64-gnu-cuda13@0.8.29` exact tarball URL/digest. No optionalDependency. |
| Rust SDK | Existing source feature contract | `embed-cuda,rerank-cuda`, explicit qualified toolkit and `CUDARC_CUDA_VERSION=13000`, same single-major source contract. No second crate/API or additive cuda12/cuda13 features. |
| CLI | Existing delivery | Recommended no new public 13 archive; qualify source-built matching diagnostic binary. A later archive requires exact identity/channel ruling. |

Python: installer replaces one same-name version with the other; this is not
side-by-side installation. Install an exact local version/URL; a bare `==0.8.29`
constraint may also match a local build, so never document it as choosing12.
Before native import collect canonical metadata locations for known providers,
not just distinct names. Deduplicate the same path seen twice, reject two distinct
locations even if names/versions agree. Both packaged x86 variants include a small
generated private marker beside the import package: public base release version,
variant, target, features and compiled CUDA version. Require exactly matching
adjacent dist-info for the selected packaged variant. Native identity contains
release version (`env!("CARGO_PKG_VERSION")` in the binding) AND compiled CUDA
version (`candle_core::cuda::cudarc::driver::sys::CUDA_VERSION`); neither access
loads a CUDA library. Compare both after native mapping, before provider work.
For cu130, metadata's local version must be exactly base release plus `+cu130`;
the native base release must match that base, not the local suffix. Generic12
metadata/base/native must agree too. A packaged missing marker, missing native
identity, wrong CUDA identity or stale same-major native release is `ImportError`
with stable `artifact_variant_mismatch` text. Duplicate provider metadata is
`ImportError` with stable `artifact_variant_conflict` text. Test displacement in
both directions, not just a generic extension inside a cu130 package.

The existing documented source/editable development workflow remains distinct:
only an explicitly identified source checkout or metadata-declared editable
installation may omit the packaged marker/identity; ordinary wheel metadata can
never take that exception. Preserve existing development tests and do not claim
installed-artifact qualification for that route. No new environment switch
bypasses identity validation. This source change adds identity to newly built12
and13 release artifacts; it does not modify retained historical bytes or change
the12.6 toolkit/default/channel contract. Do not scan/hash all installed
distributions on import. Identity is not authentication of hostile binaries;
external qualification binds file digests. Keep Tegra provider guard behavior.

Node: `FATHOMDB_CUDA_VARIANT` accepts exactly `12` or `13`, read once at initial
import; unset retains existing12/local-dev route. Explicit12/13 bypasses local-dev
binary discovery, so a stale local `.node` cannot override requested installed
bytes. Explicit13 supports only linux/x64/glibc; it does not enter Tegra probes.
Resolve the selected package manifest before `require`: name/version/main entry
must match the selected coordinate and main package version exactly. Missing or
malformed manifest, invalid selector, unsupported host and failed addon load throw
`NativeArtifactSelectionError` with a stable code/reason and selected package;
version mismatch retains `PlatformPackageVersionMismatchError`. No fallback to
12/CPU package on an installation error. Both installed variants are allowed;
only the selected manifest/version is authoritative. Neither presence nor device
policy selects13. After mapping either installed x86 variant, including
unset/default12 and explicit12, require native release version AND compiled CUDA
identity to match the selected package/main release and expected12.6/13.0 build
identity before returning the binding. Missing identity, wrong-major displaced
member or stale same-major release throws `NativeArtifactSelectionError` with
stable `artifact_variant_mismatch` reason; never try the other package. Manifest
checks occur before native load; native-member checks necessarily follow mapping
but precede any provider/device work. The existing unset/local-development path
may omit packaged identity; the exception applies only to the local-dev candidate,
never a resolved installed package. Explicit12/13 cannot take that exception.
Import again in the same process cannot change the variant; a fresh process is
required. Do not add public application exports for identity.

Alternatives: presence-based Node selection is less typing but silently changes
behavior when a package appears; library probing adds CUDA work before policy and
risks two-major mappings. Replacing the single12 artifact drops supported hosts.
Two bundled native modules increases ABI/loader risk. Explicit selection is the
smallest predictable change. Separate Node13 naming narrowly amends the rejected
alternative in ADR-0.8.25; the normal12 dual-runtime package remains normal-name.

## 3. Runtime compatibility and failure contract

Artifact choice is independent of `FATHOMDB_EMBED_DEVICE` and
`FATHOMDB_RERANK_DEVICE`. Each existing resolver keeps its own report. Within a
selected artifact, explicit CPU succeeds without driver access; auto selects the
lowest visible ordinal only, applies the existing compute floor and real minimal
probe, or falls back CPU with existing typed reason. Forced `cuda:N` fails typed
for absent driver/device, incompatible PTX/SM, library failure or provider failure;
it never changes artifacts. Existing pool failures remain errors under auto too.

| Available host libraries, compatible driver/device | Selected12 auto / forced | Selected13 auto / forced |
| --- | --- | --- |
| Only compatible12 cuBLAS/cuRAND | CUDA / CUDA | CPU with library-unavailable / typed refusal |
| Only compatible13 cuBLAS/cuRAND | CPU with library-unavailable / typed refusal | CUDA / CUDA |
| Both compatible lines | CUDA12 / CUDA12 | CUDA13 / CUDA13 |
| Neither line | CPU with library-unavailable / typed refusal | CPU with library-unavailable / typed refusal |
| Driverless (any library contents) | CPU with observed unavailable reason / typed refusal | CPU with observed unavailable reason / typed refusal |

Explicit CPU is CPU in every row. Missing driver can precede library checks, so
assert the real existing reason rather than requiring library-unavailable there.
Wrong-major decoys and missing symbols remain separate `cuda_library_unavailable`
reasons. cuBLAS must report major12 or13 matching build. cuRAND soname10 is shared:
the exact fork accepts CUDA12 component versions10301–10399 and CUDA13>=10400
(and has an unconfirmed soname11 candidate); never label libcurand.so.10 as proof
of CUDA12 or claim an unmeasured soname11 installation works. cuBLASLt dependencies
and preloaded wrong-major libraries are checked by the fork, not guessed from
filenames. Record actual loaded paths/versions; two directories exposing the same
cuRAND soname must be selected by a process-local library search path. Never
rewrite the host loader config or global LD_LIBRARY_PATH. No attempt to load
both FathomDB majors to test compatibility; a chosen library candidate may still
be rejected by the existing exact-major checker.

Use CUDA13 remedy text naming the qualified CUDA13 component package/line or
approved exact library location, selected from the build major, not cuRAND's own
major10. Verify actual NVIDIA package coordinates before implementing text; no
assumption all13 packages retain a `-cu13` suffix. Existing CUDA12 remedy stays
byte-identical. Reuse existing symbols, refuse before device construction, keep
`CudaDeviceInfo.cuda_libraries` and doctor-v1 schema/error kinds unchanged.

## 4. Compiler6111901: exact exposure and release choice

[Primary NVIDIA13.2.2 notes](https://docs.nvidia.com/cuda/archive/13.2.2/cuda-toolkit-release-notes/index.html)
rechecked2026-11-14 report a reconvergence compiler bug introduced in12.8 and fixed
in13.2.2; it needs nested lane divergence plus elided convergence. This does not
identify its compiler pass, all affected generated code or a safe flag/workaround.
Existing12.6 predates that stated introduction; this is not a blanket safety claim.

The consumed `candle-kernels/build.rs` compiles `.cu` sources to PTX using nvcc
`-O3 --expt-relaxed-constexpr -std=c++17`; it also compiles static MoE objects.
cudaforge0.1.5 finds NVCC env first, then PATH, then fallback locations. Pin both
absolute NVCC and CUDA_PATH, and inspect actual commands; merely setting
CUDARC_CUDA_VERSION controls binding selection, not nvcc. PTX is later JIT-compiled
by the driver, so checking source or only host object files is insufficient.

| Consumed path | Concrete source evidence | Exposure conclusion |
| --- | --- | --- |
| `candle_bge.rs`, `candle_reranker.rs` → Candle `models/bert.rs` | F32 BERT embedding/attention/FFN; index-select, arithmetic/casts/copy, GELU, reductions/normalization, softmax, cuBLAS GEMM. No FathomDB flash-attention or MoE model route. | This is the relevant runtime closure; do not audit unrelated model zoo as product exposure. Static linked objects still belong to artifact inspection. |
| kernels `indexing.cu` index_select/gather | Per-lane grid-tail loop encloses per-ID sentinel branch/assert; noncontiguous index computation uses shared dimension metadata. Valid tokenizer IDs may never take sentinel branch. | Potential nested divergent control in compiled kernel, not proof ordinary BERT triggers it. Source branch count alone cannot clear or convict. |
| kernels `unary.cu`, `binary_op_macros.cuh`, `reduce.cu` | Grid-tail loops, CUDA math helpers; sum/max/softmax and layernorm. Layernorm block-size/pointer tests are block-uniform despite nested syntax. CUDA math helper lowering adds compiler-generated control. | Cannot claim two divergent levels merely from LayerNorm nesting; generated math/reconvergence remains unexamined. |
| cuBLAS/curand shared libraries | Vendor-compiled bytes loaded at runtime; not rebuilt by FathomDB nvcc. | Product-kernel source edits cannot clear vendor code; use selected library releases/known-issue matrix and numerical qualification. No claim that compiler bug is present in these libraries. |

**Result: exposure UNKNOWN, no observed miscompilation and no demonstrated safe
13.0 pipeline.** There is no known source fix to commission today. Before any13
build, obtain an exact13.0 compiler/backport disposition from a primary maintainer
that covers this pipeline, or a separately reviewed technical mitigation with a
sound scope argument and discriminating regression(s). Passing a finite BERT
smoke alone does not clear silent compiler corruption. If that cannot be supplied,
return the concrete choices below; do not create a permanent paperwork gate.

| Choice | Consequence / recommendation |
| --- | --- |
| Keep13.0 policy, resolve technical safety prerequisite | Recommended current action. Implement reviewed source selection after ruling; no13 build/delivery until resolved. Outcome remains open, not silently deferred. |
| Use fixed13.2.2 compiler | New policy/toolkit/PTX/driver-floor ruling and separate capable venue required; R580/windchill3 cannot be upgraded. Not selected here. |
| Compile12.6 kernels with13 bindings/libs | Could avoid the stated compiler introduction while retaining older PTX, but changes the accepted per-major build contract and mixed-runtime provenance. Needs feasibility/ABI proof and owner ruling; not an approved workaround or default. |
| Defer second artifact | Explicit owner scope disposition required; do not mark second delivery complete. |

A concrete kernel or loader defect discovered by the mitigation work is a140-A
repair; qualify it in the owning fork. A speculative rebase, new multiarch
strategy, marker sweep or upstream proposal is not a.29 prerequisite.

## 5. Build and installed qualification

Add cu130 as a distinct selected input axis, not a replacement of global12
constants. Exact13 toolkit patch/nvcc, immutable image/base digests, host compiler,
Rust/maturin/Node/npm, Python build interpreter, static-runtime provenance and
qualified library versions remain unselected. Do not fabricate these values from
historical13 GCP results. Retain x86 cap75, manylinux2_28/glibc2.28 wheel policy,
existing abi3-py310 binding contract and Node N-API8/Node25/host glibc2.39 policy;
validate actual emitted wheel tags/symbols and all ELF version needs. Use separate
build dirs/cache keys per variant. Driver minimum for13.0 follows the selected
patch's NVIDIA table (GA580.65.06/U1 580.82.07/U2 580.95.05), plus actual extracted
PTX ISA/target proof; do not infer PTX support from family minor compatibility.
The historical12.6 PTX8.5/cap75 claim is not evidence of the new13 output.

No CUDA/NVIDIA NEEDED entries or shared-library payload in any member. Keep
cudart_static and lazy driver/library path; no x86 import-time cuInit. Tegra early
cuInit is an existing separately governed exception, not copied into x8613.
Existing12.6/Tegra routes remain unchanged. No Arm13 artifact or claim is selected.

Retain predecessor A6 licence and SBOM acceptance on the exact new
`0.8.29+cu130` wheel and `fathomdb-linux-x64-gnu-cuda13@0.8.29` tarball, plus
any rebuilt12 candidate whose packaging changes. Reuse
`scripts/check-license-consistency.sh`'s SPDX/text assertions against actual ZIP
METADATA and `.dist-info/licenses/LICENSE`, and actual tarball `package.json` and
regular `LICENSE` bytes equal to the authoritative root text. Its ordinary Python
leg is explicitly a source/mechanism proxy; `--with-wheel` builds another wheel
and cannot substitute for inspecting the selected retained local-version wheel.
The existing wheel bytecode-hygiene checker is not a licence checker. Minimally
extend the existing licence gate with `--wheel FILE` / `--npm-tarball FILE`
retained-archive inputs, reusing its predicates instead of rebuilding or creating
a new checker. Missing/wrong licence bytes or SPDX/name/version must fail.

Reuse the existing offline `scripts/sbom-survey/` dependency inventory and
CycloneDX schema validation, then correlate its entries to the selected resolved
Cargo/npm closure, qualified kernel/fork pins and static CUDA-runtime component.
Bind the resulting selected-artifact assessment to the exact candidate hashes and
archive identities in existing rehearsal output. The survey covers tracked
repository manifests and is not by itself a binary/artifact SBOM; unrelated
repository dependencies must not be claimed as shipped components. Confirm actual
archive members against the selected inventory, including any shipped third-party
notices. Do not add an embedded-SBOM package requirement, online version sweep,
new evidence framework or repeat checks that already passed on unchanged bytes.
A changed archive/checker predicate gets a genuine RED; invoking an unchanged
check on new candidate bytes is qualification, not a reason to invent a new test.

Reuse existing package rehearsal and installed consumer machinery. Verify native
file digests against candidate members, import outside checkout, use local pinned
model cache, do actual embedding and reranking, create/write/read a real format37
DB, close, then reopen/read/close in a new process with the same selected bytes.
A fresh external Rust SDK project must declare the exact kernels patch at its
root; the published Candle fork otherwise resolves registry kernels with dynamic
cudart. Resolve one qualified cudarc-fathomdb, no upstream runtime cudarc and no
concrete conflicting version features through existing checks. Compile each major
in a separate process/target directory, never link both into one consumer. No new
public feature changes; document build env instead. CLI diagnostic companion
reports compiled major/library paths and typed failure/exit without adding verbs.

## 6. Existing adoption review: exact remainder

`dev/plans/0.8.29/research/cuda-version-adoption.md` already records current primary
packaging/vendor facts and inaccessible quantitative sources. Preserve it. The
single remaining monthly query is saved in `research/data/clickpy-query.sql`:
September/October2026 `pypi.pypi_downloads_per_day_by_version`, grouped month,
project,version over runtime-cu11/runtime-cu12/runtime/runtime-cu13. Denominator:
real runtime-version downloads per complete month; exclude/report deprecated
cu13 0.0.1 separately. Compute major/minor shares only from complete responses;
one matched-window independent cross-check is still missing. Release-asset counts
for current llama.cpp CUDA12/13 and Ollama JetPack5/6/7 are also inaccessible.
Downloads remain proxies; there is no measured installed-driver/fleet share.

Retain policy rows11 unsupported,12.6 tier1,other12 library cells,13.0 tier1-next,
13.2 low/no new build,13.4+ compile-only. Current13.2 consumer packaging informs a
selected13-library compatibility cell, not a new adoption percentage or automatic
artifact. Owner either accepts these exact quantitative limits with unchanged
ratings or authorizes bounded remaining collection. Do not redo the full research.
