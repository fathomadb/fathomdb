---
title: 0.8.31 quality backlog — coverage-guided fuzzing and power-loss recovery
status: ALLOCATED
target_release: 0.8.31
---

# 0.8.31 quality backlog

Owner seq-414 allocates broad coverage-guided fuzzing and power-loss proof to
0.8.31. This is a receiving backlog, not an implementation plan or a claim that
either campaign has run. Apply the
[quality roadmap](../../notes/fathomdb-quality-testing-roadmap.md) when drafting
the release. Existing [CUDA allocations](cuda-delivery-plan.md) remain separate.

| Work | Intended acceptance and scope |
| --- | --- |
| Broad coverage-guided fuzzing | Select real FathomDB query/filter, authenticated cursor, protocol/codec and recovery boundaries by criticality. Use an instrumented coverage-guided engine, valid and malformed seed corpora, declared per-target resource/time budgets and reproducible toolchains. Record actual inputs executed, reached product paths, coverage limitations, failures and minimized reproducers. Replay actionable minimized cases in ordinary regression tests. Use real disposable stores for stateful integration; include unsafe/FFI sanitizer targets where applicable. Property generation alone does not satisfy this allocation. Avoid building a general SQLite SQL/B-tree fuzzer or a new orchestration framework. |
| Power-loss recovery proof | First specify the supported filesystem/storage/cache configuration, SQLite durability settings, acknowledged-write oracle and fault model. Exercise interrupted persistence with disposable VM/block-device or suitable fault-injection seams that can represent dropped, torn or reordered writes and flush behavior. A process SIGKILL alone does not satisfy this allocation. Target commit, WAL/checkpoint and recovery boundaries; verify acknowledged state according to the declared durability contract, atomic unacknowledged outcomes, canonical/provenance/index cohesion, pending projection work, actual recovery and healthy subsequent writes. Check semantic state as well as SQLite integrity. Preserve exact fault delivery, original outcomes and limits; do not generalize beyond the tested storage/fault model. |

Draft the requirements, independent expected-state oracles, design, necessary
environment and bounded campaign before implementation. Obtain the normal
design/code reviews and independent execution verification. Any privileged or
physical fault work must use a separately selected disposable environment;
this allocation does not authorize host resets, user databases or a paid venue.

0.8.30 retains its existing bounded product properties, process-crash, WAL and
persistent-failure tests from [Slice55](../0.8.30/features/slice-55/status.md).
Its [Slice70 limits](../0.8.30/features/slice-70/status.md) remain explicit.
This allocation neither waives other 0.8.30 qualification nor converts earlier
untested failure models into passes.
