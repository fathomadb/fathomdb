---
title: 0.8.24 Slice 40 — research and evidence boundaries
status: PROPOSED
target_release: 0.8.24
---

# Slice 40 research and evidence boundaries

## Settled repository facts

- The ordinary Windows Python and N-API matrix is CPU-oriented. CUDA must not
  be inferred from `windows-latest`; the current release workflow's restricted
  CUDA route is Linux self-hosted.
- ADR-0.8.22 fixes the Windows CPU npm package as
  `fathomdb-native-win32-x64-msvc`. It is a release prerequisite, not an alias
  available for a new CUDA package.
- The npm loader resolves a single Windows x64 CPU package. Adding a CUDA npm
  artifact requires an explicit selection algorithm and tests; optional
  dependency installation alone does not choose a native binary.
- The local Windows VM is useful for native Windows filesystem/CPU validation
  but its virtual display and absence of NVIDIA host-device passthrough make it
  unsuitable for Windows CUDA compilation or runtime proof.

## Research required after P24-09/P24-10

| Question | Required evidence | Consumer |
| --- | --- | --- |
| CUDA/MSVC compatibility | NVIDIA Windows CUDA installation guidance matched to observed driver/toolkit, MSVC, Windows SDK, and GPU. | Build design and executor manifest. |
| Native dependencies | PE/DLL inspection and applicable redistribution/licensing facts for the selected bytes. | Artifact manifest and clean smoke. |
| Python distribution | Exact project/version/index selection semantics; no collision with the CPU wheel. | Python design and install docs. |
| npm selection | Package name, optional-dependency topology, loader precedence, force/auto behavior, and upgrade/co-install rules. | ADR/interface/docs and loader tests. |
| Publisher boundary | Registry OIDC/environment constraints and artifact digest verification from builder to hosted publisher. | Workflow design; no publisher credential on the builder. |

Only primary vendor/registry documentation should settle these post-decision
questions. Artifact provenance is necessary but does not replace a real
installed GPU lifecycle smoke.

## Sources read

- Slice 0 executor inventory and publication topology.
- Slice 3 product and architecture drafts.
- Slice 4 architecture alignment and Slice 5 verification adequacy review.
- ADR-0.8.22, `src/ts/src/platform.ts`, relevant package manifests, and the
  current release workflow.
- `dev/design/0.8.23-windows-local-environment.md`, used only for the local
  CPU-validation boundary.
