---
title: 0.8.24 Slice 30 — public Tegra CUDA distribution plan
status: READY
target_release: 0.8.24
---

# Slice 30 — public Tegra CUDA distribution

## Status and governing correction

Slice 30 is **ready for TDD implementation**. The package identity is not
open: accepted decision D-80.6-3 keeps the distribution name `fathomdb` and
distinguishes the Tegra build with the exact local version `0.8.24+tegra` on a
first-party PEP 503 index. Generic CPU `fathomdb==0.8.24` remains on PyPI.

The earlier draft's `fathomdb-tegra` / separate-import premise is rejected and
retired. It contradicted `dev/tegra-platform-reference.md` and the revised
D-80.6-3 in `dev/design/0.8.23-aarch64-tegra.md`. A bare
`linux_aarch64` wheel is host-bound and is not PyPI-uploadable under any
distribution name.

The owner also clarified that the repository is this repository:
`fathomadb/fathomdb`. That resolves repository identity, not the undeclared
index endpoint or deployment ownership.

## Goal

Make the already-proven Jetson Orin CUDA wheel publicly installable without
changing the CPU distribution or import topology:

- PyPI continues to serve generic CPU `fathomdb==0.8.24`;
- a first-party PEP 503 index serves
  `fathomdb==0.8.24+tegra` as bare `linux_aarch64`;
- installation guidance is printed only after classic-Tegra detection and pins
  the exact local version;
- the supported matrix is Jetson Orin, L4T R36 / JetPack 6, CUDA 12.6,
  glibc 2.35, AArch64, and Python abi3; and
- Slice 60 receives a registry-installed Jetson smoke contract while existing
  CPU artifacts remain unchanged.

## Resolved inputs

- Distribution/import identity: one `fathomdb` distribution and one
  `fathomdb` import package.
- Version split: CPU `0.8.24` on PyPI; Tegra `0.8.24+tegra` on the alternate
  index.
- Wheel truth: `cp310-abi3-linux_aarch64`, never falsely relabelled manylinux.
- Selection safety: detection-gated command plus exact `==0.8.24+tegra` pin;
  no floating alternate-index instruction.
- Target: classic Jetson Orin (`sm_87`) on L4T R36 / JetPack 6, CUDA 12.6,
  glibc 2.35. Generic AArch64/SBSA and Thor are unsupported.
- Build/evidence route: dedicated real Jetson runner and the existing
  host-native build/witness workflow are proven inputs. The current workflow
  is not yet a 0.8.24 route: both jobs hard-code
  `refs/heads/release/0.8.23` and would skip a 0.8.24 dispatch.
- Repository claim: `fathomadb/fathomdb`.

## Authorized interim publication route

The owner authorized GitHub Pages as the interim first-party PEP 503 host on
2026-08-25. Pages is enabled in GitHub Actions mode for this repository; the
concrete base is
`https://fathomadb.github.io/fathomdb/tegra/simple/`, owned by
`fathomadb/fathomdb`.

The narrow publisher route is credentialless on the Jetson followed by a hosted
GitHub Actions job that consumes only the retained wheel artifact, constructs
the static PEP 503 pages, and deploys through the `github-pages` environment
with `pages: write` and `id-token: write`. It has no PyPI, npm, crates.io, or
long-lived registry credential. The deployment is explicitly opt-in and limited
to the exact 0.8.24 release branch and version. A retry must prove the same
wheel filename and SHA-256 before replacing the Pages deployment.

This is an interim 0.8.24 distribution route, not a permanent hosting ruling.
Before a later Tegra release, revisit artifact retention, multi-version index
generation, domain ownership, and distribution policy. That review must not
silently replace the 0.8.24 endpoint or broaden this publisher.

## Implementation plan after the blocker closes

1. Add RED contract tests for the exact same-name/local-version topology,
   honest wheel tag, detection-gated exact install command, CPU/Tegra
   separation, and publisher route.
2. GREEN the smallest allowlisted packaging/workflow/docs changes. Preserve
   the build wrapper's host-native `+tegra` staging and existing witness. Make
   the Jetson evidence workflow accept the exact authorized 0.8.24 release
   candidate rather than silently skipping it.
3. Run local structural tests and `actionlint`; do not claim target evidence
   from those checks.
4. On the dedicated Jetson only, build the exact candidate and retain source,
   toolchain, wheel digest, installed `cpu`/`auto`/`cuda:0` smokes, and GPU
   witness. Publishing remains separately authorized.
5. On explicit publish input, hand the immutable artifact to the scoped GitHub
   Pages publisher. After the Pages deployment, install from the declared
   endpoint on a clean Jetson and hand that proof to Slice 60.

## Future RED/GREEN tests

- Extend `scripts/tests/test_cuda_release_contract.sh` and
  `scripts/check-cuda-release-contract.py` to fail when the staged wheel is not
  `fathomdb-0.8.24+tegra-*-linux_aarch64.whl` or is labelled manylinux.
- Add a publisher/index structural test that fails unless the concrete endpoint
  is a single declared constant, the requirement is exactly
  `fathomdb==0.8.24+tegra`, and generic PyPI remains the CPU source.
- Extend the `_coinstall.py` tests so only confirmed classic Tegra can receive
  the alternate-index repair command; generic AArch64, SBSA, Thor, missing
  Tier-2 evidence, and generic CPU hosts must not receive it.
- Extend `test_jetson_tegra_cuda_evidence_ci_job.sh` so immutable artifact
  transfer is credentialless and the target-controlled job has no publication
  credential or deployment step.
- First add a RED mutation/contract arm proving the current
  `refs/heads/release/0.8.23` predicates skip a `release/0.8.24` candidate.
  GREEN must replace the stale literal with a bounded release-branch contract
  that permits the exact authorized `release/0.8.24` candidate while rejecting
  unrelated refs and still requiring `candidate_sha == github.sha`.
- Assert both `validate-candidate` and `tegra-cuda-evidence` share the same
  bounded ref predicate; a test that fixes only one job remains RED.
- Add release-workflow mutation fixtures proving the hosted publisher consumes
  only the named artifact/digest, uses the declared environment, and cannot
  route the Tegra wheel to PyPI.
- Preserve the existing real-hardware `cpu`, `auto`, and forced-`cuda:0`
  candidate-installed smokes; add the clean registry-installed lifecycle smoke
  only after an endpoint exists.

## Future implementation allowlist

- `src/python/fathomdb/_coinstall.py` and its focused tests;
- `scripts/release/build-python-cuda-tegra.sh` only if a contract test proves a
  publication-handoff gap;
- `scripts/check-cuda-release-contract.py` and focused release-contract tests;
- `.github/workflows/jetson-tegra-cuda-evidence.yml` only for the bounded
  0.8.24 candidate-ref correction and immutable build output;
- `.github/workflows/jetson-tegra-cuda-evidence.yml` for the bounded 0.8.24
  candidate-ref correction and its opt-in hosted GitHub Pages publisher;
- public compatibility/install documentation, the release design, maintained
  indexes, and Slice 30 local records.

Changes to npm, generic ARM64 CPU artifacts, Rust target triples, public SDK
shape, platform-capabilities schema, crates.io, or PyPI CPU publication require
separate justification and are not implied by this plan.

## Verification after implementation

- Focused Python and release-contract tests, `actionlint`, Markdown/plan/anchor
  lint, and `git diff --check`.
- Real Jetson host contract, wheel metadata and digest, glibc 2.35 assertion,
  clean candidate install, all three device policies, and validated in-process
  witness.
- After separately authorized publication, exact-version clean install from the
  declared PEP 503 endpoint and open/write/search/close/exit proof.
- Generic CPU `fathomdb==0.8.24` install remains sourced from PyPI and unchanged.

## Definition of done

Slice 30 closes only after the authorized route's design passes review,
RED/GREEN implementation is complete, a real Jetson candidate proof is
retained, and the immutable artifact/Pages publisher handoff is ready for the
explicit publication input. This plan does not itself dispatch a workflow or
publish an artifact.
