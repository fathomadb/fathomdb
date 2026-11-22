---
title: FathomDB 0.8.35 release scope — publishable benchmarking, runner upgrades and competitive evaluations
status: PROPOSED
target_release: 0.8.35
date: 2026-11-21
---

# FathomDB 0.8.35 release scope

## Theme

Upgrade FathomDB's configurable, repeatable benchmarking toolchain and execute
**rigorous, reproducible, publishable latency, scale, retrieval-quality,
evidence-sufficiency, and competitive benchmark evaluations** on the finalized
`0.8.31`–`0.8.34` production surface — directly closing the External Reviewer's
P0 proof bar and publishing transparent results in `docs/benchmarks/`.

## In scope

### 1. Maintain and upgrade the configurable, repeatable benchmarking tool

Build on the existing config-driven coordinators on `main`
(`experiments/sdk_benchmark.py` with `sdk.experiment.v1` / `sdk.driver.v1`, and
`experiments/competitive_runner.py` with `competitive.experiment.v1/v2` /
`competitive.adapter.v1` / `experiments/scorecard.py`):

- **Unified, declarative configuration (`sdk.experiment.v2` /
  `competitive.experiment.v3`)**:
  - Pin dataset paths and SHA-256 digests, verified installed package versions
    and build attestations (published wheel/npm/crate, never accidental dev
    builds), hardware/device profile (`cpu`, `cuda`, `orin`), warmup vs. timed
    repetitions, random seed, process isolation/timeouts, and paired-cell
    comparability rules.
  - Enforce strict separation between **data-plane** and **end-to-end (host +
    LLM)** layers per
    [FATHOMDB-BOUNDARY.md](../../performance-benchmarking/FATHOMDB-BOUNDARY.md).
- **Full operation & driver coverage (closing `BT-AC04` and `BT-AC06`)**:
  - Upgrade Python, TypeScript, and Rust SDK drivers — and add a **4th driver
    for read+write `fathomdb-mcp`** — to benchmark: cold/warm `open`,
    strict vs. bulk `write`/`actuate` (ingest-to-ready and mutation-to-ready),
    `search_text_only`, `search` (vector, hybrid, reranked, filtered —
    explanations off and on), `graph.expand` (1–3 hops: unpredicated,
    ad-hoc & declared edge-property-predicated, fused-hit seeded, historical
    `as_of`), `freeze_read_context` + `resolve_evidence` /
    `resolve_evidence_many` / `resolve_graph_evidence`, `erase_source`/`purge`,
    and scoped multi-caller queries.
- **Expanded metric families & cross-platform telemetry**:
  - Add `complete_evidence_recall`, `exact_citation_accuracy`,
    `abstention_precision` / `abstention_recall` / `abstention_f1`,
    `historical_path_exact_match`, `scope_isolation_violations`,
    `ingest_to_ready_ms`, `tokens_per_verified_answer`, and
    `tool_calls_per_verified_answer`.
  - Add macOS and Windows peak RSS attribution alongside Linux.
  - Compute paired bootstrap 95% confidence intervals and generate publishable
    Markdown tables/charts directly for `docs/benchmarks/`.
- **Lean runner self-tests**:
  - Maintain fast deterministic CI smoke checks for config validation, rerun
    repeatability (identical treatment hashes), and driver contracts (measuring
    correctness only — no tamper-test bloat).

### 2. Competitive benchmark evaluations (four apples-to-apples tracks)

All competitive runs use identical corpora, chunking, and (where applicable)
identical LLM reader/answerer and judge models on verified installed packages:

1. **Track 1 — Evidence Sufficiency & Embedded Retrieval (External Reviewer P0
   flagship)**:
   - **Competitors**: **SQLite + FTS5** (fairly equipped with identical
     revision IDs and source offsets), **SQLite + FTS5 + `sqlite-vec`** (DIY
     stack), and **LanceDB** or **Chroma** (embedded).
   - **Corpora & query classes**: Held-out **Design-Controls / Evidence
     Sufficiency suite** (exact-ID, paraphrase, conflicting-revision,
     missing-evidence abstention via host policy, and graph-hop) plus
     **NFCorpus / BEIR** and scale tiers (10k / 50k / 100k).
   - **Metrics**: complete evidence retrieval, exact citation/span accuracy,
     host-level abstention F1, p50/p95/p99 latency, ingest-to-ready, DB bytes,
     peak RSS.
2. **Track 2 — Agent Memory Systems (Personal & Long-Horizon Agent Fit)**:
   - **Competitors**: **Mem0 (OSS)**, **Graphiti / Zep**, and naive RAG
     baseline under one shared reader and judge.
   - **Corpora**: **LoCoMo** and **LongMemEval** (factoid, temporal,
     multi-session, knowledge-update, abstention) — refreshing all historical
     0.8.3 numbers on the single current `0.8.35` artifact.
   - **Metrics**: answer accuracy, temporal correctness, per-class recall,
     latency, tokens and dollar cost per verified answer.
3. **Track 3 — Embedded Property Graph & Multi-Hop Retrieval**:
   - **Competitors**: **Kùzu** (embedded property graph), **SurrealDB**
     (reusing existing `experiments/comp_drivers/surrealdb`), and **HippoRAG-2**
     (multi-hop retrieval).
   - **Workloads**: 1–3 hop property-filtered graph traversals (`0.8.31`
     inline + declared edge/node predicates + historical `as_of` replay) and
     **MuSiQue** (2–4 hop supporting-evidence reachability).
   - **Metrics**: traversal p50/p95/p99, historical path exactness, all-bridges
     multi-hop recall, memory footprint.
4. **Track 4 — Cross-SDK, MCP & Hardware Matrix (Internal Parity & Scaling)**:
   - **Matrix**: Rust (`fathomdb-sdk`) vs. Python vs. TypeScript vs. MCP across
     **CPU (x86_64/ARM64)**, **CUDA (RTX 3090)**, and **Jetson Orin** at 1k /
     10k / 50k / 100k+ records.
   - **Metrics**: cross-binding latency overhead, GPU speedup envelope,
     tokens/calls per verified answer (SDK vs. MCP), RSS and disk amplification,
     and TC5 exact-f32 vector fidelity (strictly separated from IR relevance).

### 3. Public benchmark documentation and reproducibility package

- Publish `docs/benchmarks/` containing methodology, hardware/software
  disclosure, summary tables and charts, 95% confidence intervals, one-command
  reproduction manifests, and an explicit **"What this proves and what it does
  not establish"** section for every track.

## Out of scope

- Engine architecture rewrites (ANN/HNSW stays post-1.0 / 2.x; 1–2M custom
  kernel stays gated on E1 attribution).
- Unrepeatable single-run marketing claims or undisclosed judge/hardware
  comparisons.
- Mixing historical `0.8.x` numbers with `0.8.35` results in published tables.
