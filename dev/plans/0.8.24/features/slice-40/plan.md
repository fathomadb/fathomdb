---
title: 0.8.24 Slice 40 — Windows x64 CUDA distribution draft plan
status: DRAFT
target_release: 0.8.24
---

# Slice 40 — Windows x64 CUDA distribution

## Planning boundary

This document plans Slice 40. It does not choose the Python/npm matrix, operate
a Windows host, compile locally, register a runner, upload an artifact, configure
a publisher, or publish a package. The accepted direction requires a remote
Windows x64 CUDA executor. **No local Windows compilation is required or
authorized.**

## Goal and outcome

Design and, after explicit decisions, implement prebuilt Windows x64 CUDA for
the owner-selected Python, npm, or combined SDK surface while preserving every
existing Windows CPU artifact and unsupported-route behavior. The result must:

- require no end-user or release-operator local Windows compilation;
- bind bytes to current source, MSVC/Rust/Python/Node/CUDA/driver/GPU facts;
- distinguish CUDA artifact identity/selection from the existing CPU wheel and
  `fathomdb-native-win32-x64-msvc` npm package;
- move immutable artifacts through a reviewed trust boundary to hosted
  OIDC publishers where required; and
- supply Slice 60 with a real Windows GPU candidate-installed smoke contract.

## Authority and inputs

- P24-09/P24-10, R24-2/R24-9, A24-3, draft
  `REQ-TARGET-WINDOWS-CUDA`/`AC-TARGET-WINDOWS-CUDA`, and Slice 6 decisions.
- `dev/plans/0.8.24/prework/executor-inventory.md` and publication topology.
- ADR-0.8.22 for the existing Windows **CPU** npm package name.
- Current Windows CPU jobs, Python/N-API package metadata, `platform.ts`, CUDA
  feature forwarding/build helpers, witness schemas, and installed smokes.
- `dev/design/0.8.23-windows-local-environment.md` as a CPU Windows validation
  environment description, not proof of a CUDA GPU executor.
- NVIDIA's current Windows CUDA installation/compiler compatibility guide.
- npm trusted-publishing documentation: OIDC trusted publishing currently
  requires a supported cloud-hosted runner, so a self-hosted GPU builder cannot
  silently become the npm publisher.

Primary references begin with:

- <https://docs.nvidia.com/cuda/cuda-installation-guide-microsoft-windows/>
- <https://docs.npmjs.com/trusted-publishers/>
- <https://docs.github.com/en/actions/reference/security/secure-use>
- <https://docs.github.com/en/actions/concepts/security/artifact-attestations>

## Scope

### In scope

- Obtain the owner-selected Python/npm support matrix.
- Observe and approve a remote Windows x64 CUDA executor and its trust boundary.
- Choose artifact/distribution identity and runtime selection for each selected
  SDK without replacing the CPU artifact.
- Define remote build, link/dependency inspection, GPU runtime proof, artifact
  sealing/transfer, hosted publication, and installed smoke.
- Define clear behavior for unsupported, CUDA-unavailable, and forced-CUDA
  routes.
- Add/update an ADR/design, public compatibility/install docs, and interface
  docs if loader or error behavior is user-visible.

### Non-goals

- Local Windows compilation, hosted `windows-latest` CPU jobs as CUDA proof,
  Windows ARM64/32-bit, macOS CUDA, or Tegra.
- Renaming or replacing the existing Windows CPU npm package by implication.
- Requiring both SDKs if the owner selects only one.
- Giving a self-hosted/public-repository runner publish credentials or trusting
  a label as access control.
- Claiming a compile, artifact upload, or `nvidia-smi` output alone is an
  installed-package smoke.

## Slice prep — planned first phase

Create under this directory:

- `prep.md` — goals, SDK/executor inventory, current-main SHA, and decision state;
- `draft-contracts.md` — slice-local needs/requirements/acceptance drafts;
- `design.md` — selected artifact, loader, executor, transfer, and smoke design;
- `research.md` — primary-source Windows CUDA/npm/GitHub findings; and
- `decision.md` — owner-selected SDK matrix and approved executor contract.

### Prep tasks

1. Record the chosen SDK surface: Python, npm, or both. For each, enumerate the
   current CPU package, native module filename, loader/import path, version,
   optional dependencies, and release job.
2. Observe the proposed remote executor: selector, online state, Windows build,
   GPU/compute capability, driver/CUDA toolkit, Visual Studio/MSVC/SDK, Rust,
   Python, Node/npm, storage/cache, service identity, isolation, and transfer.
3. Restate and refine the drafts:
   - **N40-DRAFT:** a Windows x64 user can explicitly install a supported
     prebuilt CUDA SDK artifact without compiling it locally;
   - **R40-DRAFT-1:** the selected SDK matrix and CPU/CUDA package-selection
     behavior are explicit, versioned, and fail clearly when unsupported;
   - **R40-DRAFT-2:** remote build bytes are bound to immutable source and
     executor/toolchain/GPU evidence before a hosted publisher consumes them;
   - **AC40-DRAFT:** each selected installed artifact completes
     open/write/search/close/exit with selected CUDA and retained process/device
     evidence; CPU artifacts remain separately installable.
4. Read bindings/release architecture, Python/TypeScript interfaces, ADR-0.8.22,
   and the actual loader/package/workflow/test bodies. Write an
   exists-versus-net-new map.
5. Assign prerequisites: CI route to Slice 10, SDK/executor/identity choices to
   this slice's owner gate, shared publisher/smoke matrix to Slice 60, and final
   evidence/publication readiness to Slice 70.

## Artifact options the design must evaluate

For every selected SDK, compare options without assuming the answer:

### Python

- a separately named Windows-CUDA distribution with explicit mutual exclusion
  if it provides the same `fathomdb` import package;
- a separate CUDA plugin/native-provider distribution loaded by the CPU package;
  or
- another explicit identity that preserves the CPU wheel tag and import path.

Two different CPU/CUDA wheels with the same project/version/platform tag are
not a viable immutable-registry distinction.

### npm

- a separately named CUDA platform package selected explicitly by a stable
  loader policy;
- a separate top-level CUDA distribution; or
- another owner-approved topology that never changes the accepted CPU package
  identity from ADR-0.8.22.

Installing two matching OS/CPU optional dependencies is not sufficient; the
loader selection and unsupported behavior must be deterministic and tested.

## Draft design and design review

### Required design content

- selected SDK matrix and exact package names;
- user selection/configuration and unsupported/unavailable/forced behavior;
- CPU package preservation and co-install/upgrade semantics;
- remote executor trust, default-branch workflow ownership, candidate SHA, and
  no-candidate-controlled privileged script boundary;
- pinned Windows/CUDA/MSVC/Rust/Python/Node toolchain and dependency inspection;
- artifact manifest/digests, transfer, retention, and hosted publisher jobs;
- OIDC/environment/repository/workflow claims for PyPI/npm as applicable;
- real Windows GPU lifecycle smoke and selected-device/process evidence;
- docs/interfaces/ADR/release-note consequences; and
- Slice 60/70 handoff schema.

### Challenging aspects and research plan

1. Verify NVIDIA's supported Windows/compiler/CUDA matrix for the proposed
   target and the project's Candle/native dependency chain.
2. Inspect PE/DLL dependency behavior and decide which CUDA runtime libraries
   may be redistributed versus host-provided; use NVIDIA and dependency
   license/docs as primary sources.
3. Verify npm/PyPI OIDC runner and workflow/environment constraints. In
   particular, keep self-hosted build separate from cloud-hosted npm publish.
4. Evaluate artifact attestations or an equivalent signed manifest; an
   attestation proves provenance, not runtime correctness, so retain the GPU
   smoke separately.
5. Verify how Python and npm installers behave when CPU and CUDA artifacts have
   overlapping files or platform selectors.

### Architectural-fit review and revision

Check the design against actual CPU packages/loaders, bindings architecture,
the Windows CPU ADR, public interfaces, and current release workflow. Revise to
remove any implicit CPU replacement, unproved hosted-runner capability, or
self-hosted publication credential. A changed package identity or public loader
policy receives an ADR/successor and matching interface/docs before code.

## Planned implementation sequence after decisions

1. Land the approved ADR/design and failing local contract tests first.
2. Implement selected package metadata/loader/build-contract changes under TDD.
3. Prepare a main-owned remote build harness with immutable source input and
   fail-closed manifest sealing.
4. Execute compilation only on the approved remote Windows CUDA host. Retain
   toolchain, dependencies, digests, and GPU runtime evidence.
5. Transfer sealed artifacts to a reviewed hosted publisher job; never pass
   publishing credentials to the build host.
6. Run the clean candidate-installed smoke on the remote GPU host.
7. Update public install/compatibility docs and hand exact artifacts/smokes to
   Slice 60. Publication remains separately authorized.

## Verification and evidence

- Local RED/GREEN contract tests for package identities, loader selection,
  unsupported paths, workflow graph, manifests, and negative evidence.
- Existing Windows CPU package/runtime tests stay green unchanged or are
  extended only to assert preservation.
- `actionlint`, PowerShell/shell checks, package dry-run/inspection, and
  applicable release-contract tests.
- Remote build witness includes immutable SHA, compiler/toolchain, DLL
  dependencies, artifact digests, and CUDA/GPU facts.
- Candidate-installed smoke uses a fresh environment and installed artifact,
  not source/editable state; it performs open/write/search/close/exit and proves
  selected CUDA with device/process evidence.
- Registry-installed proof is handed to Slice 60/70 and happens only after a
  separately authorized publication.

## Risks and recovery

| Risk | Control / recovery |
| --- | --- |
| CPU package is silently replaced | Separate identity/selection and explicit preservation tests. |
| Remote host is CPU-only or misconfigured | Observe GPU/toolchain before ready status; hosted Windows CPU is not evidence. |
| Self-hosted runner can publish | Split build from hosted OIDC publication and verify manifest/digests. |
| Candidate-controlled code reaches privileged host | Default-branch-owned harness, immutable SHA, reviewed trust route. |
| Missing DLLs fail only for users | Inspect dependencies and run clean installed smoke on the target host. |
| Published bytes are immutable | Fail before publish; recover through deprecation/yank and a corrected version, never replacement. |

## Decisions and prerequisites for the next reviewer

Ready status requires:

1. owner-selected Python/npm/both matrix;
2. exact CPU/CUDA package and loader topology for each selected SDK;
3. named, observed, approved remote Windows CUDA executor;
4. build-to-publisher trust/transfer design; and
5. trusted-publisher/environment claims for each selected registry.

If no suitable executor or safe artifact identity is available, Windows CUDA is
deferred rather than weakened into local compile or CPU-only evidence.

## Definition of done

Slice 40 closes only after the selected prebuilt Windows CUDA surface is
implemented under TDD, existing CPU artifacts remain intact, remote build and
clean installed GPU smoke evidence are retained with immutable provenance, and
Slice 60 receives the exact artifact/smoke matrix. It does not close on a
hosted CPU build, source compilation, or artifact upload alone.
