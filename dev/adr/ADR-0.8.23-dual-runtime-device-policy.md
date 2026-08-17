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
auto driver/probe diagnostic failure reports `probe_failed` with a typed CPU
effective device and exits `70`; forced not-compiled/unavailable/incompatible
CUDA reports no effective device and exits `65`; invalid policy exits `70`. CPU-only artifacts are
truthful about `cuda_not_compiled`.

## Consequences

This changes public CLI and cross-binding configuration semantics and requires
interface updates, release-artifact evidence, and a successor to the existing
ambient fallback behavior. TC-5 remains private and explicit-device only.
