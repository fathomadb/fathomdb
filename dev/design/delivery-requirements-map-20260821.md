---
title: Delivery requirements map — languages, platforms, packaging, fragile deps
date: 2026-08-21
status: PROPOSED
desc: >
  Ground-truth map of FathomDB's actual multi-language/multi-platform delivery
  and release surface, read directly from the Rust workspace, the binding
  packages, and every GitHub Actions workflow — companion to
  `ci-challenges-review-20260821.md`. Analysis only; no CI config, script, or
  GitHub setting is touched by this doc.
blast_radius: >
  read-only: Cargo.toml (workspace root); src/rust/crates/*; src/python/pyproject.toml;
  src/ts/package.json; .github/workflows/{ci,release,aarch64-release-preflight,
  jetson-tegra-cuda-evidence,corpus-freeze,perf-canonical}.yml;
  scripts/release/{cuda-artifact-contract,glibc-floor-contract}.sh;
  docs/compatibility/index.md; dev/design/{release,bindings}.md
---

# Delivery requirements map — languages, platforms, packaging, fragile deps

**Status: PROPOSED, analysis only.** This doc changes no workflow, script, or
GitHub setting. Purpose: separate *inherent* delivery complexity (a real
requirement of shipping FathomDB across languages/platforms/GPU vendors) from
*avoidable* CI ceremony, so `ci-challenges-review-20260821.md`'s findings can
be judged against the real requirement surface rather than assumed. Every
claim below is grounded in a file opened during this review; file paths are
cited inline.

## 0. The closest existing "support contract" doc

`docs/compatibility/index.md` is the one doc in the repo that already tries
to state the full delivery contract for users (supported Python/Node/Rust
versions, published prebuilt platforms, SQLite/sqlite-vec linkage, schema
version, versioning axes, breaking changes, open perf gates). It is the
right spine to extend, not re-derive — but it is **stale against the actual
release pipeline**: its own header says "the published **0.8.21** release"
while the workspace is at **0.8.23** (`Cargo.toml:26`), and its platform
table claims

> Do not expect the published npm package to install on macOS, Windows, or
> Linux musl. [...] macOS x86_64 / arm64 / Windows x64 — no, unsupported
> published artifact

— but `release.yml` demonstrably builds *and publishes* npm platform packages
and PyPI wheels for macOS x64/arm64 and Windows x64 on every real release
(`publish-npm-platform-darwin-x64`, `-darwin-arm64`, `-win32-x64-msvc`,
`publish-pypi` consuming `python-dist-*` for all five `build-python` matrix
rows — see §2). Multi-platform prebuilts have existed since `v0.1.0`
(`git log` on `.github/workflows/release.yml` shows "Multi-platform
prebuilts and OIDC npm provenance" at `1b945652`), so this isn't new
release-engineering outrunning old docs by days — the *documented* support
contract has been narrower than the *shipped* one for a long time. This is
itself a concrete instance of the general problem this doc exists to
illuminate: nobody has a single place that states, and keeps current, what
FathomDB actually promises to build, test, and publish.

`dev/design/release.md` (version axes, tiered publish order, registry
idempotency) and `dev/design/bindings.md` (cross-language protocol
invariants — governed SDK surface, error mapping, build/packaging path per
binding) are the two locked design docs that come closest to a delivery
contract, but neither states the platform/GPU/glibc matrix; that detail
lives only in `docs/compatibility/index.md`, `scripts/release/
glibc-floor-contract.sh`, and the workflow YAML itself.

## 1. Languages & bindings

Three consumer-facing surfaces, one shared Rust core:

| Surface | Package | Built via | Publish target |
|---|---|---|---|
| Rust (facade + 6 libs) | `fathomdb`, `fathomdb-cli`, `fathomdb-embedder`, `fathomdb-embedder-api`, `fathomdb-engine`, `fathomdb-query`, `fathomdb-schema` | `cargo build --release --workspace` / `cargo publish` | crates.io — 7 crates |
| Python | `fathomdb` (PyO3 binding crate `fathomdb-py`, `publish = false` in Cargo.toml — never itself goes to crates.io) | maturin, `pyo3 = "0.29"` with `abi3-py310` (`src/rust/crates/fathomdb-py/Cargo.toml:53`) — one wheel covers Python 3.10+ per platform | PyPI — 1 package, 5 platform wheels/release |
| TypeScript/Node | `fathomdb` + 5 per-platform `fathomdb-<triple>` packages (binding crate `fathomdb-napi`, also `publish = false`) | napi-rs (`napi_build::setup()` in `fathomdb-napi/build.rs`) | npm — 6 packages/release |
| CLI | `fathomdb-cli` binary (`fathomdb doctor`, `fathomdb recover`, `fathomdb export`, ...) | `cargo build --release -p fathomdb-cli` (`dev/design/bindings.md:230`) | ships inside the wheel/npm smoke path + `cargo install fathomdb-cli` (crates.io) |

10 Cargo workspace members total (`Cargo.toml:2-12`); 7 are published crates
(the T1–T7 tiers in `dev/design/release.md`'s publish order), 3 are
deliberately unpublished glue (`fathomdb-py`, `fathomdb-napi` — binding
cdylibs, ship compiled, not as source crates; `fathomdb-tc5-benchmark` —
internal benchmark harness).

`fathomdb-embedder-api` runs on an independent version axis ("Axis E",
currently `0.6.1` vs. workspace "Axis W" `0.8.23` — `Cargo.toml:16-44`)
because it's the semver-stable plugin trait; every other crate lockstep-bumps.
This means a release can require bumping 8 version fields across 2 axes
before a tag is even cut, each checked by `scripts/set-version.sh
--check-files`.

## 2. Platforms, architectures & GPU variants — the real matrix

**Not a full cross-product.** Five platform/arch targets recur across every
build-shaped job, but which artifact family is built where, and with what
GPU backend, differs per job. This table is the ground truth, read from
`.github/workflows/ci.yml`'s `wheel-size-gate` (L776-861) and
`native-artifact-runtime-validation` (L881-956) matrices and `release.yml`'s
`build-python` (L409-465), `build-napi` (L474-566), and
`build-cuda-linux-x64-gnu` (L839-895) jobs:

| Platform / arch | Python wheel | napi `.node` | GPU backend | Runner | glibc floor |
|---|---|---|---|---|---|
| Linux x86_64 (gnu) | yes | yes | **CUDA 12.6** (baked into the "normal" shipped artifact) | self-hosted `[Linux,X64,gpu,cuda-12]` — the `windchill3` RTX 3090 box | 2.39 (`cuda-napi-host` family — built host-natively, not in the manylinux container) |
| Linux aarch64 (gnu) | yes | yes | CPU only | `ubuntu-24.04-arm` (GitHub-hosted) | 2.28 (`manylinux` family; napi row is containerized in `manylinux_2_28` specifically to match — L529-538) |
| macOS x86_64 | yes | yes | CPU only (Metal feature exists but is not wired into any CI/release job) | `macos-15-intel` | n/a |
| macOS arm64 | yes | yes | CPU only | `macos-14` | n/a |
| Windows x64 (MSVC) | yes | yes (named `fathomdb-native-win32-x64-msvc`, not scoped `@fathomdb/`) | CPU only | `windows-latest` | n/a |
| Linux aarch64, Jetson/Tegra | yes (built, **not published**) | not built | CUDA (JetPack-bundled) | self-hosted `[Linux,ARM64,jetson,aarch64]` — a Jetson Orin box | 2.35 (`tegra` family — highest of the three, host-native) |

So: **5 platform targets are actually built+published for both PyPI and
npm on every real release** (linux-x64-gnu, linux-arm64-gnu, darwin-x64,
darwin-arm64, win32-x64-msvc). A 6th (Jetson Tegra CUDA) is built and smoke
tested on real hardware but deliberately never published — it's
"real-hardware evidence," not a release artifact
(`jetson-tegra-cuda-evidence.yml:1-5`). CUDA is real on exactly **one** of
the five published platforms (Linux x64) plus the unpublished Tegra build;
every other platform ships CPU-only. There is no macOS Metal or Windows GPU
build anywhere in CI despite `embed-metal`/`rerank-metal` Cargo features
existing (`fathomdb-embedder/Cargo.toml:97,107`) — those features are
declared but currently dead in the pipeline.

Three **different, independently measured glibc floors** exist for what
looks like "the Linux build" — 2.28 (manylinux, most artifacts), 2.35
(Tegra), 2.39 (the CUDA-enabled Linux x64 npm binary, because it's built
outside the manylinux container to get access to the CUDA toolkit) — all
declared in one file (`scripts/release/glibc-floor-contract.sh:1-56`) and
enforced per-family by `scripts/check-glibc-floor.sh`, cross-checked against
`docs/compatibility/index.md` by `scripts/check-glibc-floor-doc-truth.sh` so
the *claim* can't silently drift from the *measurement*. This machinery
existing at all is itself evidence of how easy it is for "Linux" to
silently mean three different minimum-OS promises.

### Self-hosted runners

Two distinct self-hosted GPU boxes, both gating real release artifacts:

1. **`windchill3`** (RTX 3090, x86_64, labels `[self-hosted, Linux, X64,
   gpu, cuda-12]`) — the CUDA-12.6 build/publish host for the "normal"
   Linux x64 Python wheel and npm package (`release.yml:842`), plus the
   `cuda-contract-preflight` / `cuda-package-rehearsal` /
   `cuda-reranker-package-rehearsal` dry-run rehearsal jobs
   (`release.yml:313,574,716`). Referenced across `dev/design/
   0.8.23-gpu-artifacts.md:55`, `dev/tegra-platform-reference.md:541`, and
   several 0.8.23 slice runbooks.
2. **Jetson Orin runner** (ARM64, labels `[self-hosted, Linux, ARM64,
   jetson, aarch64]`) — the Tegra CUDA evidence host
   (`jetson-tegra-cuda-evidence.yml:27,53`), workflow-dispatch-only,
   `concurrency: cancel-in-progress: false` because "one integrated GPU
   cannot give attributable allocation evidence to concurrent runs"
   (`jetson-tegra-cuda-evidence.yml:17-21`).

Both are the "org-scoped self-hosted GPU runner group" that motivated the
2026-08-10 `coreyt/fathomdb` → `fathomadb/fathomdb` org transfer per
`ci-challenges-review-20260821.md` §2. Every other job in `ci.yml` and most
of `release.yml` runs on GitHub-hosted runners (`ubuntu-latest`,
`ubuntu-24.04-arm`, `macos-14`/`macos-15-intel`, `windows-latest`).

An `aarch64-release-preflight.yml` workflow (69 lines, one job) additionally
rehearses the Linux ARM64 wheel+napi build on **every push** that touches
release-relevant paths (workflow files, `src/{python,rust,ts}/**`,
`Cargo.toml`/`Cargo.lock`) — not just at release time — because that
workflow validates an unmerged branch, which `release.yml` (tag-only) can't
do (`aarch64-release-preflight.yml:1-6`).

## 3. Fragile / native dependencies — and why

| Dependency | Where | Why fragile |
|---|---|---|
| **Candle** (`candle-core-fathomdb`, `candle-kernels`, `candle-nn-fathomdb`, `candle-transformers-fathomdb` `=0.10.2`) | `fathomdb-embedder/Cargo.toml:35-37`; patched in root `Cargo.toml:71-74` via `[patch.crates-io]` to a pinned git rev of a FathomDB-maintained fork (`github.com/coreyt/candle-fathomdb`) | Not upstream Candle — a fork pinned to one exact commit, maintained to preserve a Linux AArch64 CPU Gemm/F16 fallback and to control CUDA linkage. `candle-kernels` owns the static CUDART link boundary; the published dual-runtime Linux artifact must have no dynamic `libcudart.so.12` or driver dependency, so CPU mapping/operation on a CUDA-free host is a product requirement, not an image choice. Different Cargo features compile genuinely different backends per GPU vendor (`embed-cuda`, `embed-metal`, `rerank-cuda`, `rerank-metal` — `fathomdb-embedder/Cargo.toml:96-107`), so a change here can silently affect only one platform's numerical output (see the retired "CPU↔CUDA byte-identity" ULP finding in program memory). |
| **CUDA toolkit / nvcc / host gcc** | `scripts/release/cuda-artifact-contract.sh:17-26,114` | Pins nvcc to an exact string (`'Cuda compilation tools, release 12.6, V12.6.68'`) and the host C compiler to `/usr/bin/gcc-13` specifically, with a comment explaining Ubuntu 22.04 "carries stock gcc 11.4.0 and has no gcc-13" while CUDA 12.6 needs it — i.e. the release host's stock OS toolchain is not sufficient and a specific compiler must be side-installed and pinned, or nvcc/host-compiler ABI mismatches break the build in ways that only show up on that one self-hosted box. |
| **glibc floor divergence** | `scripts/release/glibc-floor-contract.sh` | Three different floors (2.28 manylinux / 2.35 Tegra / 2.39 CUDA-linux-x64-napi) exist because CUDA support requires building *outside* the manylinux container that gives the low, portable floor — building for GPU support silently raises the minimum end-user OS/glibc version unless separately measured and gated per artifact family (§2). |
| **rusqlite (`bundled`) + sqlite-vec `=0.1.9`** | `fathomdb-engine/Cargo.toml:19,23`, `fathomdb-cli/Cargo.toml:56`, `fathomdb-schema/Cargo.toml:18,31` | `bundled` compiles SQLite from C source per target platform/arch (not linked against system SQLite); `sqlite-vec` is a native loadable SQLite extension statically linked into every wheel/`.node` binary (`docs/compatibility/index.md`'s "SQLite + sqlite-vec" section) — another from-source native C build gated to an exact pinned version (`=0.1.9`), repeated on every platform target in §2. |
| **onnxruntime (`ort` crate, `=2.0.0-rc.10`)** | `fathomdb-embedder/Cargo.toml:38-51` | Pinned to a release *candidate*, not stable — later RCs (rc.12) have a known `ort`/`ort-sys` VitisAI execution-provider mismatch, so rc.10 is the newest RC that builds cleanly (HITL-accepted 2026-07-08, tracked as follow-up TC-9). `load-dynamic` means the actual native ONNX Runtime library is resolved at *runtime* via `ORT_DYLIB_PATH`, not linked at build — a per-platform runtime provisioning dependency that CI/release currently does not build or test at all, since `onnx-embedder` is a non-default, opt-in feature absent from every matrix in §2. |
| **napi-rs native module + containerized ARM64 build** | `release.yml:501-561` | The one row of the napi build matrix (`linux-arm64-gnu`) that needs to match the manylinux glibc floor is built inside a digest-pinned `manylinux_2_28` Docker container instead of bare-metal on `ubuntu-24.04-arm`, with an assert step (`Assert glibc floor (AC80-1)`) proving the resulting `.node` file's actual symbol floor rather than trusting the image — every other napi row builds bare-metal. |

## 4. Multi-registry publishing — the count

Per real release (`release.yml`, triggered on `push` of a version tag),
independently:

| Registry | Artifacts published | Notes |
|---|---|---|
| crates.io | 7 crates, tiered T1→T7 (`dev/design/release.md` § Tiered publish order), sequential with 60s inter-tier sleeps for index propagation | `fathomdb-embedder-api` (Axis E) must land first so every downstream crate can resolve it |
| PyPI | 1 logical package (`fathomdb`), 5 platform wheel files uploaded together (`publish-pypi`, `release.yml:1159-1183`, pattern `python-dist-*`) | abi3, so 5 wheels (not 5×3 per-interpreter) cover Python 3.10–3.12+ |
| npm | 6 packages: 1 thin main `fathomdb` (dist/ only, no binary — binaries injected as `optionalDependencies` just before publish, `scripts/release/npm-inject-optional-deps.sh`) + 5 platform binary packages | non-scoped names (`fathomdb-linux-x64-gnu`, ..., `fathomdb-native-win32-x64-msvc` — the one exception forced by npm's own name-ownership rules, per `dev/design/release.md:206-209`) |

14 published packages total per release, gated behind an `all-builds-passed`
cross-ecosystem barrier (`release.yml:901-915`) so a partial release (e.g.
crates.io succeeds, PyPI build fails) can't leave the ecosystems at
mismatched versions — every publish job is individually idempotent
(`cargo-publish-if-new.sh` / `npm-publish-if-new.sh` / PyPI `skip-existing`)
so a retried release re-attempt is a no-op on anything already published.
Post-publish, 5 separate registry-installed smoke jobs
(`post-publish-smoke`, `-aarch64`, `-darwin-x64`, `-darwin-arm64`,
`-win32-x64`) install the just-published artifact fresh and exercise
open/write/search/close, because per `dev/design/release.md`'s "Post-publish
smoke" section, "green CI + published wheel" is not considered done.

## 5. Connecting the surface to CI cost

`ci-challenges-review-20260821.md` measured full `CI` runs on `main` at
**19-21 minutes wall-clock, back-to-back**, at a 75-103 runs/day cadence.
This doc's numbers explain most of that steady-state cost independent of any
correctness bug:

- Every non-docs-only push runs **two 5-row platform matrices**
  (`wheel-size-gate`, `native-artifact-runtime-validation` — §2) = 10
  platform-specific jobs, plus **2 more Windows-only jobs**
  (`windows-wal-checkpoint-diagnosis`, `windows-wal-attribution`, one of
  which itself builds a maturin wheel) = 12 platform-specific jobs running
  in parallel on every push.
- Alongside that: 5 heavy Ubuntu jobs (`verify-fast`, `verify`, `security`,
  `default-embedder-tests`, `rust-workspace-race-report`) each bootstrapping
  a Python + Node + Rust toolchain, and roughly 13 lightweight always-on
  governance/doc-integrity jobs (`gitleaks`, `shell-lint`, `board-currency`,
  `ledger-integrity`, `plan-anchors`, `governed-surface-pin`,
  `pinned-override-rot`, `c1-contract-conformance`, `transcript-hygiene`,
  `release-state-views`, `commission-manifest`, `design-status`,
  `steward-orient`, `docs`) that are cheap individually but add scheduling
  and queueing overhead atop the platform matrices.
- That's **~25 jobs defined in `ci.yml` alone**, most running concurrently
  on every non-docs push — a direct, countable consequence of 2 SDK
  bindings × 5 platform/arch targets × (CUDA-or-not) needing independent
  build+validate legs, not a single monolithic "run the tests" step.
- `release.yml` (34 jobs) and the two evidence workflows compound this
  further at actual release time: 4 jobs pinned to the one self-hosted GPU
  box, a 4-row `build-python`/`build-napi` matrix run twice (once each),
  and 14 sequenced/idempotent publish jobs plus 5 post-publish smokes.

None of this is evidence that any *specific* gate documented in the
companion review (gitleaks full-history scan, the broken
`windows-wal-attribution` script, etc.) is justified — those verdicts stand
on their own. What this map adds is the base rate: even a hypothetically
perfect CI config, with zero ceremony and zero bugs, would still fan a
single push out to roughly a dozen platform-specific build/test legs because
of the genuine language×platform×GPU-vendor surface FathomDB ships across.
The judgment call for the HITL is which of the ~25-60 jobs across these
workflows are *inherent* to that shipped surface (§1-§4) versus incidental
process/ceremony layered on top (the subject of
`ci-challenges-review-20260821.md`).

## Sources

- `Cargo.toml` (workspace root) — crate list, version axes, Candle patch
- `src/rust/crates/{fathomdb-py,fathomdb-napi,fathomdb-embedder,fathomdb-engine,fathomdb-cli,fathomdb-schema}/Cargo.toml`
- `src/python/pyproject.toml`, `src/ts/package.json`
- `.github/workflows/{ci,release,aarch64-release-preflight,jetson-tegra-cuda-evidence,corpus-freeze,perf-canonical}.yml`
- `scripts/release/{cuda-artifact-contract,glibc-floor-contract,cuda-preflight}.sh`
- `docs/compatibility/index.md`
- `dev/design/release.md`, `dev/design/bindings.md`
- `dev/design/0.8.23-gpu-artifacts.md`, `dev/tegra-platform-reference.md`
- `ci-challenges-review-20260821.md` (companion doc, same review)
