---
title: FathomDB 0.8.34 release plan
status: PROPOSED
target_release: 0.8.34
date: 2026-11-21
---

# FathomDB 0.8.34 release plan

> **Status: PROPOSED — awaiting completion of `0.8.31`–`0.8.33`, explicit
> budget approval for any priced LLM-judged runs, and owner go-ahead.**
> Execution requires an approved `release-state-0.8.34.json`, live board
> `dev/plans/runs/STATUS-0.8.34.md`, and separate publication authority.

## Goal

Upgrade FathomDB's configurable, repeatable benchmarking suite
(`experiments/sdk_benchmark.py` and `experiments/competitive_runner.py`), run
four rigorous competitive and cross-platform benchmark tracks on the single
pinned `0.8.34` release candidate, and publish reproducible latency, scale,
retrieval-quality, and evidence-sufficiency benchmarks in `docs/benchmarks/`.

## Quality roadmap and standing tripwire check

- **Quality testing roadmap**
  ([fathomdb-quality-testing-roadmap.md](../../notes/fathomdb-quality-testing-roadmap.md)):
  - **Applies in `0.8.34`**: uses real benchmark corpora as workload stress
    tests, enforces exact operation/citation oracles alongside latency
    (preventing fast-but-wrong regressions), qualifies the declared scale and
    platform envelope honestly, and retains reproducible receipts and
    confidence intervals.
  - **Deferred**: new ANN/backend qualification (post-1.0 / 2.x).
- **Standing tripwires**
  ([overflow-chunking-reevaluation.md](../../design/overflow-chunking-reevaluation.md)):
  Track 1 and Track 2 answer-level evaluations will directly observe whether
  tripwire `T1` (answer-level loss traced to evidence past the 512-wordpiece
  cut) fires; if it fires, record the finding and schedule a separate
  re-evaluation without altering `0.8.34` mid-campaign.

## Requirements and acceptance signals

| ID | Requirement | Falsifiable acceptance signal |
| --- | --- | --- |
| R34-01 | Upgraded configurable, repeatable benchmark runner (`sdk.experiment.v2` / `competitive.experiment.v3`) | Same declarative manifest executed twice into distinct output roots produces identical treatment hashes, verified package/dataset SHA-256 attestations, and valid receipts; deterministic CI smoke passes on Linux/macOS/Windows. |
| R34-02 | Full SDK (Rust, Python, TS) + MCP driver coverage and expanded metric families | All 4 drivers execute open, write/actuate, FTS/vector/hybrid/filtered search, property-predicated & historical `graph.expand`, frozen evidence resolution, erasure, and scoped reads; new citation, abstention, historical-path, RSS, and cost metrics populate cleanly. |
| R34-03 | Track 1 complete: Evidence Sufficiency & Embedded Retrieval vs. SQLite+FTS5 (+`sqlite-vec`), LanceDB/Chroma | Paired, held-out Design-Controls suite (exact-ID, paraphrase, conflicting-revision, missing-evidence abstention, graph-hop) + NFCorpus/BEIR + 10k/50k/100k scale complete with 95% CIs; SQLite+FTS5 baseline receives identical revision IDs and source offsets. |
| R34-04 | Track 2 complete: Agent Memory vs. Mem0 (OSS), Graphiti/Zep, and naive RAG | LoCoMo and LongMemEval runs complete on pinned `0.8.34` under one shared reader/judge protocol with checkpointed resume and recorded per-class recall, answer accuracy, temporal accuracy, latency, and cost per verified answer. |
| R34-05 | Track 3 complete: Property Graph & Multi-Hop vs. Kùzu, SurrealDB, and HippoRAG-2 | 1–3 hop property-predicated + historical `as_of` graph benchmarks and MuSiQue multi-hop reachability complete with verified exactness oracles and latency/memory profiles. |
| R34-06 | Track 4 complete: Cross-SDK, MCP & Hardware Matrix | Rust/Python/TS/MCP and CPU (x64/ARM64) / CUDA (RTX 3090) / Jetson Orin matrix completes on pinned `0.8.34` artifacts; TC5 exact-f32 fidelity is reported distinctly from IR relevance. |
| R34-07 | Publishable `docs/benchmarks/` report and reproduction manifests | `docs/benchmarks/` passes `mkdocs build --strict`, `./scripts/agent-lint-md.sh`, and `./scripts/agent-lint-docs.sh`; every published number traces to a receipt, hardware spec, 95% CI, reproduction command, and explicit scope-limits statement. |

## Slice ladder (proposed)

| Slice | Title | Requirements | Depends on | Parallel / conflict notes |
| --- | --- | --- | --- | --- |
| 0 | Benchmark protocol freeze, corpus/gold manifest pins, and priced-run budget approval | All | `0.8.33` published + HITL budget approval | Pins datasets, SHA-256 manifests, competitor versions, and LLM judge/reader budgets before any priced run. |
| 10 | Benchmark runner upgrade (`sdk.experiment.v2`, `competitive.experiment.v3`, 4 drivers, cross-platform RSS, CIs) | R34-01, R34-02 | 0 | Upgrades `experiments/sdk_benchmark.py`, `experiments/competitive_runner.py`, `experiments/sdk_drivers/`, adds MCP driver, and wires lean CI smoke tests. |
| 20 | Competitive adapter build and dry-run validation (SQLite+FTS5/`sqlite-vec`, LanceDB/Chroma, Mem0, Graphiti/Zep, Kùzu, SurrealDB, HippoRAG-2) | R34-03, R34-04, R34-05 | 10 | Builds/updates `experiments/adapters/` and `experiments/comp_drivers/` with `$0` cheap-validate + checkpoint/resume verification before priced runs. |
| 30 | Track 1 & Track 4 execution (Evidence Sufficiency, Embedded Retrieval, Cross-SDK/MCP/Hardware Matrix) | R34-03, R34-06 | 20 | Uses RTX 3090 GPUs when available for embedding/eval; runs on verified installed packages. |
| 40 | Track 2 & Track 3 execution (Agent Memory + Property Graph / Multi-Hop) | R34-04, R34-05 | 20 | Can run parallel to 30 on separate compute/budget envelopes with resumable checkpoints. |
| 50 | Independent result audit, `docs/benchmarks/` authoring, and publication | R34-07 | 30, 40 | Audits receipts, CIs, and claims against `dev/performance-benchmarking/README.md` rules; generates charts/tables; publishes `docs/benchmarks/` and `0.8.34` release. |

## Open HITL decisions required before Slice 0 starts

1. **Competitor final shortlist**: Confirm the exact embedded vector competitor
   for Track 1 (LanceDB vs. Chroma vs. both) and whether Graphiti/Zep and
   HippoRAG-2 are both required in Track 2/3 or prioritized sequentially.
2. **Priced LLM judge/reader budget**: Approve explicit dollar cap and
   reader/judge model pin for Track 1 (host-abstention/answer checks) and Track
   2 (LoCoMo + LongMemEval) before Slice 30/40 launch.
3. **Held-out Design-Controls corpus source**: Confirm whether the Track 1
   Design-Controls / Evidence Sufficiency corpus will be an authored open-license
   benchmark committed with FathomDB (so external users can reproduce every
   number verbatim).
