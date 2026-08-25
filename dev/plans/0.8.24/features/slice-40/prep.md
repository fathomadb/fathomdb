---
title: 0.8.24 Slice 40 — preparation record
status: PROPOSED
target_release: 0.8.24
---

# Slice 40 preparation record

## Assessment basis

**Reviewed 2026-08-25 at release branch `d369280058dd3f2b888a2bdc5a8a6bc803047762`.**
This is an evidence-backed planning record, not an authorization to operate a
runner or alter a release route.

## Goals

- Declare exactly which Windows SDK surface can receive CUDA support.
- Preserve all existing Windows CPU package contracts.
- Bind new bytes to an observed remote Windows GPU executor and a credential
  split between builder and publisher.
- Give Slice 60 a target-native candidate smoke, not merely a build artifact.

## Existing versus net-new

| Surface | Evidence | Disposition |
| --- | --- | --- |
| Python CPU | `build-python` in `.github/workflows/release.yml` includes `windows-latest` / `x86_64-pc-windows-msvc`; `src/python/pyproject.toml` has project `fathomdb`. | Existing CPU build only. |
| npm CPU | `platformPackageName` in `src/ts/src/platform.ts` maps Windows x64 to `fathomdb-native-win32-x64-msvc`; the package is constrained to `win32`/`x64`. | Existing accepted CPU identity only. |
| Existing Windows checks | Release workflow has Windows build, platform publish, and post-publish smoke jobs. | Hosted CPU evidence only. |
| CUDA implementation | No Windows CUDA package, loader choice, CUDA job, or target artifact contract is present. | Net new. |
| Remote builder | Slice 0 recorded Linux self-hosted CUDA only and no approved Windows CUDA builder/facts. | Owner input P24-10 required. |
| Local VM | `gh-runner-wonl-win11` is local Windows validation; current assessment observes a virtual display, no NVIDIA hostdev, and CPU-only validation. | Explicitly excluded as CUDA proof. |

## Decision inputs

| Decision | Required owner record | Why it cannot be inferred |
| --- | --- | --- |
| P24-09 | Python, npm, or both, plus intended supported/unsupported behavior. | The current Python and npm CPU routes have different packaging and loader mechanics. |
| P24-10 | Trusted-builder form: owner-operated external non-Actions builder, or Actions builder in a dedicated selected-repository/selected-workflow runner group; observed GPU/toolchain facts, trust boundary, artifact transfer, and retention. Labels alone are not access control. | Neither a generic hosted Windows runner nor the local VM proves CUDA capability or artifact trust. |

## Dependencies and handoff

- Slice 10's existing main CI assessment remains authoritative; Slice 40 must
  not recreate main-owned CI merely for this feature.
- Slice 40 owns selected identity, implementation, executor provenance, and
  candidate Windows GPU proof.
- Slice 60 consumes an exact selected identity/digest/smoke record, preserves
  CPU publishers, and later proves registry-installed bytes.
- Slice 70 integrates completed work and coordinates owner-authorized release
  actions; it does not invent a missing Windows artifact or executor.
