---
title: ADR-0.8.30-explicit-module-model-release
date: 2026-11-20
target_release: 0.8.30
desc: Explicit synchronous ownership release for module-level CLS and reranker caches
blast_radius: fathomdb-embedder, engine reranker, Rust SDK, Python and TypeScript bindings
status: accepted (commissioned Slice 50 B30-01)
---

# ADR-0.8.30 — Explicit module model release

## Context

Module-level CLS embedders and the cross-encoder previously retained native
model ownership until process exit. Engine-owned providers already have a
separate close lifecycle. Slice 60 established safe last-owner drop, including
completed-construction CUDA loss, at cudarc `3509cd6bd83b44204be1880e833474ab0f75e060`
and Candle `952edb7dcd1e29563acedd485a3c1ce8ef07a85c`; those exact pins are
the prerequisite for this change.

## Decision

Rust SDK and Python expose `release_models()`, and TypeScript exposes asynchronous
`releaseModels(): Promise<void>`. The call releases both process module caches
before successful return. Empty and repeated calls succeed without constructing
models, downloading weights or initializing CUDA. It does not close an engine or
release its Mean-pooled provider.

A single admission gate covers module model loading, cache waits and inference.
Release refuses immediately with the existing `Overloaded` class if any such
work or another release is admitted. Refusal clears neither cache and leaves
admitted work unchanged. During successful release, admission remains closed
through both native destructors. Load, inference and destruction run outside
cache/gate mutexes; Python performs them outside the GIL. Guards restore gate
and initializer state on unwind.

Healthy calls may subsequently reload. Explicit release clears healthy
unavailable/policy memoization, but never clears the retained per-device CUDA
lifetime or its terminal loss record. Later calls after detected generation loss
still return the existing typed CUDA context-loss error until process restart.
Automatic idle release and same-process CUDA recovery are excluded.

The engine reranker uses a scoped cache lease from the embedder crate; ranking
math and fallback behavior stay in the engine. Rust SDK, Python and N-API CLS
operations share the other scoped cache. No model reference escapes its lease.
The public exhaustive `RerankerDevicePolicyError` adds `ModelBusy`; fallible
reranker paths translate it to `Overloaded`. The infallible `rerank_fused`
retains its existing unchanged-input fallback. This pre-1.0 addition requires
exhaustive downstream matches to account for the variant.

## Alternatives

Keeping leaked or static references cannot support explicit destruction.
Independent cache locks permit partial release and reload during another native
destructor. Holding one mutex across native calls serializes unrelated inference
and risks lock-order deadlocks. Owned cached values with short admission and
cache locks provide bounded refusal and a single teardown boundary.

## Validation

Slice 50 owns drop-count, both-cache admission, loading, teardown and unwind
oracles; actual Rust/Python/Node release, reload and typed Busy behavior; and
fresh affected native healthy/lost-generation children. Slice 60's unchanged
fork and allocator qualification remains its own evidence. No new device policy,
allocator default, numerical policy or disk format is introduced.
