---
title: ADR — driverless CPU loadability of CUDA-capable Linux artifacts
status: accepted
target_release: 0.8.25
---

# ADR — driverless CPU loadability of CUDA-capable Linux artifacts

## Context

The published 0.8.24 Linux x86_64 CUDA-capable N-API artifact could not load
on the generic Linux npm smoke host: the FathomDB-maintained Candle fork's
`candle-kernels/build.rs` emitted a dynamic `cudart` linker dependency. The
host's dynamic loader therefore rejected the module before FathomDB could
resolve `FATHOMDB_EMBED_DEVICE=auto` or `cpu`. The prior so-called driverless
rehearsal mounted `libcudart.so.12`, so it did not establish the promised
driverless contract.

## Decision

For every supported Linux x86_64 CUDA-capable FathomDB artifact, every packaged
ELF member's OS dynamic-dependency table must contain no CUDA or NVIDIA
user-mode library, and the archives must carry no such shared-library payload.
The complete `candle-kernels` dependency, not only Candle core/nn/transformers,
is pinned to the reviewed FathomDB-maintained fork revision that owns this
linkage. CUDA driver discovery remains lazy through the Candle/cudarc path and
occurs only after runtime policy selects CUDA. A static CUDA runtime
implementation may be linked into the artifact only if the driverless CPU and
explicit-CPU acceptance criteria below pass; it is not permission to probe a
CUDA driver during module import or explicit CPU operation.

## Consequences

- `auto` on a host without CUDA and explicit `cpu` both load and complete the
  installed Python and npm lifecycle smoke without CUDA runtime libraries,
  NVIDIA device nodes, or CUDA-related environment variables. Both the normal
  CPU cases and driverless forced-CUDA cases carry no CUDA runtime mount or
  search path.
- A forced `cuda:N` request remains typed and fail-closed when the driver or
  runtime is unavailable. It never becomes CPU execution.
- The release evidence must inventory and inspect every packaged ELF member,
  bind `readelf -d` output to artifact/member digests, and reject CUDA/NVIDIA
  shared libraries or bundled variants. Container images that happen to
  include or mount CUDA libraries cannot substitute for that proof.
- GPU evidence remains a separate trusted-runner smoke with a selected-device
  allocation witness. CPU compatibility is not inferred from GPU success.

## Alternatives rejected

- Installing or mounting `libcudart.so.12` in the generic registry smoke:
  this hides the broken CPU contract rather than testing it.
- Skipping the Linux npm smoke: this would allow immutable broken artifacts to
  be promoted.
- A separate CUDA-only npm package: it changes normal package identity and
  loader semantics, contrary to the existing normal-name dual-runtime design.
