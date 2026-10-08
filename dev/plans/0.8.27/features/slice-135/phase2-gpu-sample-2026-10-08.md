---
title: Slice 135 candidate CPU–CUDA query sample — 2026-10-08
status: COMPLETE_FOR_NAMED_SAMPLE
target_release: 0.8.27
---

# Candidate CPU–CUDA query sample

## Scope and identities

The candidate's installed Python CPU and CUDA routes returned identical
top-K document lists on **611 frozen queries out of 6,115 (9.99%)** across
the four Phase 2 retrieval corpora. Every scored gold metric on those query
IDs also matched. This is a candidate route comparison, not a paired
0.8.26-versus-candidate CUDA campaign or a Slice 135 closeout verdict.

The unchanged candidate product source is
`224e44c593c13d86ece648adabe445723db04070`. The CPU Phase 2 candidate
wheel/native SHA-256 values are
`ee8b402f76377956034f900ef69f9e3c0296d79ea30d6728dc1283040266f85a` /
`1f213a700cf0c96b9997700d6e44ed7019b26fb91df1259bc6514fefb4a8d013`.
An isolated `embed-cuda` wheel built from the clean detached checkout has
wheel/native values
`116ff91cd5afd51660a17faff69cb30ccfcf82730c3fe14567ee1e78c6e7e46a` /
`dacd2610d88e0ffd74fe6bedfe584cd974037de19fd8c73880cb7c1ccd085b37`.
The installed GPU run forced `FATHOMDB_EMBED_DEVICE=cuda:0` and recorded the
effective RTX 3090 device, UUID
`GPU-5f9cfc90-2be1-06a7-ce39-5a6d294b209b`. An in-process `nvidia-smi`
capture showed nonzero allocation on that UUID for each corpus run. CPU
and GPU used the same pinned BGE model and full canonical corpus; every
CPU/GPU canonical row was independently compared after ingestion.

Before GPU scoring, the manifest froze a SHA-256-order sample within each
source/class stratum. The [audit summary](phase2-gpu-sample-audit-2026-10-08.json)
binds the manifest SHA-256
`e224d6627a82fa111a2fa5e514c39ab1e40f63d14276e8cfd86926e3dab9252b`,
input CPU raw and database hashes, CUDA wheel, GPU database hashes, query
counts, and independently recomputed paired outcomes. Sample sizes are 447
relevance queries (71 EnronQA, 218 QAConv, 158 QMSum), 10 MuSiQue questions
(4 two-hop, 3 three-hop, 3 four-hop), 144 LOCOMO questions (84 factoid, 32
temporal, 28 multi-session), and 10 vector queries across all four sources.

## Same-query results

| Corpus and gold measure | Full corpus | Sample | CPU candidate | CUDA candidate |
| --- | ---: | ---: | ---: | ---: |
| IR exact-fact required-document Recall@10 | 2,050 docs | 289 queries | 256/289 | 256/289 |
| IR exploratory required-document Recall@10 | 2,050 docs | 158 queries | 60/158 | 60/158 |
| MuSiQue complete support set @10 / @20 | 1,669 passages | 10 queries | 3/10 / 3/10 | 3/10 / 3/10 |
| LOCOMO complete required sessions @10 / @20 | 272 sessions | 144 queries | 119/144 / 138/144 | 119/144 / 138/144 |
| LOCOMO strict multi-session complete @20 | 272 sessions | 27 queries | 22/27 | 22/27 |
| Vector-stage exact-f32 neighbor Recall@10 | 1,000 bodies | 10 queries | 0.980 | 0.980 |

All sampled IR top-10, MuSiQue and LOCOMO top-20, and vector top-15
ordered hit lists matched exactly. The sampled IR exact-fact and
exploratory MRR@10 values also matched at `0.6889218432` and
`0.1355686156`. The vector corpus used its frozen 15-hit retrieval limit
so the scorer could exclude the query's target body before assessing ten
non-target neighbors. A bounded diagnostic on the same GPU index queried
all 100 vector cases: CPU and CUDA each scored **0.931 Recall@10**, with
100/100 identical per-query scores and top-15 lists.

The first GPU vector probe incorrectly requested only ten hits and read
0.930 on the sampled queries. This was an invalid comparison. The target
body occupied a top-ten slot in 68/100 diagnostic queries; the short cap
lowered 37 per-query fidelity scores by one neighbor. The initial probe
and the corrected 15-hit rerun are retained in
the raw bundle. No product bytes changed.

## Disposition and retention

The named x86_64, installed-Python, default BGE candidate retrieval route
has CPU–CUDA parity on this sample. GPU document and query embeddings need
not be byte-identical; the observed scores and hit IDs are the comparison.
The result does not qualify the 0.8.26 CUDA route, Jetson, other
models/providers, installed Rust/TypeScript, generated answers, or Phase 1
GPU latency. Those cells remain under the [Slice 135 plan](plan.md).

The licensed raw receipts, frozen IDs, CUDA wheel, CPU source receipts,
GPU databases, device captures, scoring scripts and invalid first probe
are in the verified local archive
`/home/coreyt/projects/fathomdb/data/corpus-data/eval/slice135-gpu-query-sample-raw-2026-10-08.tar.zst`.
Its SHA-256 is
`f61c7c958cc97af1ec252cd788076dc544de9b781da0f79c70e92f44c5c0ef33`;
`zstd -t` and archive listing passed. The tracked audit summary contains
no corpus text. Preserve this archive alongside the Phase 2 raw archive
before worktree cleanup.
