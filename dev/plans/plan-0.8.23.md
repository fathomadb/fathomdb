---
title: FathomDB 0.8.23 — CUDA-capable Linux artifacts
status: PROPOSED
target_release: 0.8.23
---

# FathomDB 0.8.23 — CUDA-capable Linux artifacts

0.8.23 makes the published Linux x86_64 Python wheel and Node native binary
capable of using NVIDIA CUDA. CPU remains the runtime default; CUDA is an
explicit opt-in through `FATHOMDB_EMBED_DEVICE=cuda:N`.

This release packages existing CUDA embedder support. It does not introduce a
new GPU backend, automatic GPU selection, or a change to the CPU-only,
deterministic retrieval path.

## Goal and scope

1. Ship a GPU-capable Linux x86_64 binary in the normal PyPI and npm packages.
2. Preserve install and CPU operation on hosts without an NVIDIA driver.
3. Require a real CUDA smoke before a 0.8.23 publish can proceed.
4. Keep ordinary GitHub-hosted CI CPU-only; GPU work runs only on the protected
   `windchill3-fathomdb-cuda` repository runner.

**Out of scope:** Windows CUDA artifacts, macOS Metal artifacts, dynamic GPU
selection, NVML-based scheduling, memory budgeting, and ONNX execution-provider
work. Their capability must remain explicitly unavailable until each platform
has a reproducible build and hardware smoke.

## Release requirements and acceptance criteria

| ID | Requirement | Acceptance criterion |
| --- | --- | --- |
| G23-1 | Linux x86_64 npm and PyPI release artifacts compile the existing CUDA embedder. | Contract tests inspect the exact release build arguments and prove the N-API feature path forwards `embed-cuda`. |
| G23-2 | A CUDA-built artifact remains usable without an NVIDIA driver when CPU is selected. | A clean CPU-only smoke opens, writes, searches, closes, and exits with the release-equivalent artifact. |
| G23-3 | An explicit CUDA request uses a real NVIDIA device in the shipped-artifact path. | On `windchill3-fathomdb-cuda`, a release-equivalent artifact runs with `FATHOMDB_EMBED_DEVICE=cuda:0`; the smoke passes and records CUDA process memory/activity. |
| G23-4 | GPU release evidence cannot be silently replaced by ordinary CI. | The release workflow requires the self-hosted CUDA build-and-smoke job; pull-request workflows never target its labels. |
| G23-5 | Users can tell what is supported. | Platform capability metadata and public installation/embedder documentation name Linux x86_64 CUDA only and retain CPU fallback semantics. |

## Slice ladder

```text
0 → 5
```

| Slice | Title | Depends on |
| ---: | --- | --- |
| 0 | CUDA artifact contract, dependency probe, and protected runner gate | — |
| 5 | Build, package, release-gate, and installed-artifact CUDA smoke | 0 |

## Slice 0 — CUDA artifact contract, dependency probe, and protected runner gate

### Requirements

- Freeze the Linux x86_64-only CUDA artifact policy and CPU-default behavior.
- Establish whether the actual shared objects require a CUDA runtime at process
  load, then make the CPU-without-NVIDIA case a release requirement rather than
  an assumption.
- Record the dedicated runner identity, labels, lifecycle owner, and workflow
  trust boundary.

### Acceptance criteria

- A red-first contract test fails against the current release configuration:
  N-API has no `embed-cuda` forwarding feature and Linux release jobs omit it.
- The design review records the `readelf`/`ldd` dependency result for the
  release-equivalent Node binary and Python extension, plus the clean CPU smoke
  environment required by G23-2.
- `windchill3-fathomdb-cuda` is online with labels `self-hosted`, `Linux`,
  `X64`, `gpu`, and `cuda-12`; its service is enabled for the `coreyt` user.
- The CUDA runner is reachable only from protected branch/tag or explicit
  dispatch jobs, never from untrusted pull-request code.

### Design and design review

The design of record is
`dev/design/0.8.23-gpu-artifacts.md`. Its review must close these questions
before Slice 5 starts: runtime-library behavior on driverless Linux, exact
artifact ownership, release-workflow dependency graph, runner trust boundary,
and the observable GPU-engagement witness.

### Implementation discipline — TDD (RED/GREEN)

1. **RED:** add the release-artifact contract test and run it against the
   existing feature and workflow wiring; it must fail for the missing CUDA
   path.
2. **GREEN:** make only the minimum contract/probe and protected-runner changes
   required for the test and design witnesses to pass.
3. **REFACTOR:** remove duplicated build-feature spelling so one checked source
   owns each artifact's feature set.

## Slice 5 — build, package, release-gate, and installed-artifact CUDA smoke

### Requirements

- Forward `embed-cuda` through the N-API crate and select it for the Linux
  x86_64 release NPM artifact and Python wheel.
- Build the Linux CUDA artifacts on the dedicated runner using CUDA 12.6;
  preserve all non-Linux-CUDA artifact builds unchanged.
- Gate publication on a real GPU smoke of release-equivalent, then
  registry-installed, artifacts.

### Acceptance criteria

- The N-API package exposes an `embed-cuda` feature that forwards to the engine;
  a release-equivalent Node build has that feature enabled only for Linux x86_64.
- The Linux x86_64 wheel is built with `pyo3/extension-module`,
  `default-embedder`, and `embed-cuda`; its clean CPU smoke passes without a
  CUDA device request.
- The GPU smoke runs an installed npm package and installed PyPI wheel on
  `cuda:0`, verifies correct open/write/search/close/exit behavior, and records
  a CUDA engagement witness.
- The release workflow blocks before publish when the CUDA build, CPU fallback
  smoke, GPU smoke, or post-publish installed-package smoke fails.
- Platform metadata, npm documentation, and public embedder documentation agree
  that CUDA is supported for Linux x86_64 only in this release.

### Design and design review

Slice 5 implements only the approved Slice 0 artifact contract. Review checks
the feature graph, artifact linkage, CPU fallback, CUDA witness, workflow label
isolation, and package/docs agreement. A finding that requires a new backend,
runtime-bundling policy, or Windows/macOS support returns to a reserved-gap
proposal; it does not widen this slice.

### Implementation discipline — TDD (RED/GREEN)

1. **RED:** land failing tests for N-API feature forwarding, Linux-only release
   arguments, runner-label isolation, CPU fallback, and GPU witness parsing.
2. **GREEN:** implement the smallest Cargo, package-script, workflow, capability,
   and documentation changes that make each test pass; run the real CUDA smoke
   on the self-hosted runner.
3. **REFACTOR:** centralize release feature lists and smoke invocation so Python,
   npm, CI, and release workflows cannot silently drift.

## Cross-cutting Definition of Done

- Every implementation change is red → green → refactor, with the initial
  failing command preserved in the slice witness.
- The CPU query/retrieval path remains CPU-only, deterministic, and unchanged.
- The final release rehearsal passes all ordinary platform builds and the CUDA
  build/smoke; a release is not declared complete until registry-installed
  Python and npm smokes pass.
- No untrusted pull-request workflow may run on the self-hosted runner.

## Decisions recorded

- 2026-08-10 — HITL selected Linux x86_64 CUDA support as mandatory for 0.8.23;
  Windows CUDA follows only after its own reproducible build and hardware smoke.
- 2026-08-10 — CPU remains the default runtime device. CUDA is an explicit
  `FATHOMDB_EMBED_DEVICE=cuda:N` opt-in.
- 2026-08-10 — GitHub-hosted CI remains CPU-only. CUDA release proof runs on
  the repository-scoped `windchill3-fathomdb-cuda` self-hosted runner.

## Immediate next slice

**Slice 0 — CUDA artifact contract, dependency probe, and protected runner
gate.** Commission it from a fresh `main` worktree after the 0.8.23 release
state is opened.
