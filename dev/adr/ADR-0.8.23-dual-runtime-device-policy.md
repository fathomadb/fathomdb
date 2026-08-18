---
title: ADR — dual CPU/GPU runtime device policy
status: proposed
target_release: 0.8.23
---

# ADR — dual CPU/GPU runtime device policy

## Context

HITL `seq:250` requires one released executable/artifact to support CPU and
compatible CUDA use without an end-user rebuild, with a configurable on/off
policy and diagnostic. The existing ambient device variable silently falls back
to CPU and is not a public contract.

## Proposed decision

Supported CUDA-capable Linux x86_64 artifacts compile both CPU and CUDA support
and use one typed policy: `auto`, `cpu`, or forced `cuda:N`. `auto` is default
on CUDA-capable artifacts; `cpu` never initializes CUDA; forced CUDA fails
typed rather than falling back. `fathomdb doctor gpu` reports the resolved
capability without database access, engine initialization, model activity, or
configuration writes. It reports ordered process-visible UUID inventory rather
than host ordinals. Policy-satisfied automatic CPU fallback exits `0`; a true
auto unknown/OOM/allocation diagnostic failure reports `probe_failed` with a
typed CPU effective device and exits `70`; listed compatibility/architecture
evidence reports `cuda_incompatible` with typed CPU and exits `0`; forced
not-compiled/unavailable/incompatible
CUDA reports no effective device and exits `65`; invalid policy exits `70`. CPU-only artifacts are
truthful about `cuda_not_compiled`.

The Candle classifier is closed and code-based: missing dynamic driver,
`CUDA_ERROR_NO_DEVICE`, and `CUDA_ERROR_STUB_LIBRARY` are unavailable; exactly
`CUDA_ERROR_SYSTEM_DRIVER_MISMATCH`,
`CUDA_ERROR_COMPAT_NOT_SUPPORTED_ON_DEVICE`, `CUDA_ERROR_NO_BINARY_FOR_GPU`,
`CUDA_ERROR_UNSUPPORTED_PTX_VERSION`, `CUBLAS_STATUS_ARCH_MISMATCH`, and
`CUBLAS_STATUS_NOT_SUPPORTED` are incompatible; unknown errors,
`CUDA_ERROR_OUT_OF_MEMORY`, and `CUBLAS_STATUS_ALLOC_FAILED` are probe failures.
ONNX retains a dedicated strict resolver because it cannot expose Candle's UUID
inventory; forced ONNX CUDA never builds a CPU session. Doctor v1 does not
fabricate build-target, toolkit, or driver-version metadata; those are retained
by the Slice 10/20 artifact witnesses.

## Consequences

This changes public CLI and cross-binding configuration semantics and requires
interface updates, release-artifact evidence, and a successor to the existing
ambient fallback behavior. TC-5 remains private and explicit-device only.
