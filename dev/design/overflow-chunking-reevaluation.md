---
title: Overflow chunking — re-evaluation record and tripwires
date: 2026-10-31
status: ACTIVE
desc: Read this before any decision to enable overflow chunking (option C of long-record-embedding.md). Holds the decision timeline, what is already built so chunking needs no format break, the tripwires that reopen the question and how each is monitored, and the costs the owner requires to be weighed, above all latency and the CS64 crossing.
inputs:
  - dev/design/long-record-embedding.md
  - dev/experiments/ac013-page-cache/RECOMMENDATION.md
  - dev/design/1-2M-chunk-scaling-vector-latency-roadmap.md
  - dev/plans/runs/IR-C-retrieval-findings.md
  - dev/plans/plan-0.8.28.md
  - dev/plans/plan-0.8.29.md
---

# Overflow chunking: re-evaluation record

**Status, 2026-10-31: parked, with the store built ready for it.** The owner
ruled that chunking stays parked behind its tripwires, and that the store
format and the lifecycle upkeep are built now so that enabling it later needs
no format break.

**Owner's concern, to be answered at re-evaluation.** The owner is very
concerned about the gain against the latency cost, and especially about how
chunking pushes a store toward or past the point where CS64 stops paying
(§ 4). A re-evaluation that does not answer § 4 with fresh measurements is
incomplete.

## 1. How this file is found

A tripwire firing must lead here. These pointers exist so it does:

- `dev/design/long-record-embedding.md`, in its header and in § 7.2.
- `dev/design/1-2M-chunk-scaling-vector-latency-roadmap.md`, because the
  exact-scan kernel is tripwire 3.
- The plan authoring checklist in `dev/plans/prompts/PLAN-TEMPLATE.md`: every
  new release plan checks the tripwires in § 3.
- A standing todo, `TC-2a9f89fa` in
  `dev/todos-and-considerations-ledger.jsonl`, retargeted at each release
  close.
- A pinned-constant test (planned, 0.8.29 Slice 20) that fails when the
  fusion weights or the default embedder change, and names this file.

## 2. Decision timeline

| Date | Event | Source |
| --- | --- | --- |
| 2026-06-11 | IR-C closes the dense line under current knobs. Chunk geometries (64/48 to 256/192, max, mean and top-2 pooling) lift dense-only recall, but fused hybrid is flat or negative. Whole-record long-context embedding and late chunking are parked, not refuted. | `dev/plans/runs/IR-C-retrieval-findings.md` |
| 2026-07-02 | The 1–2M scaling brief ranks a fast exact-scan kernel (Path A) first, ANN second. It is conditional: gate G-2 (experiment E1, overhead attribution) is open, and the default on no answer is no kernel. ANN stays 2.x. | `dev/design/1-2M-chunk-scaling-vector-latency-roadmap.md`; `dev/design/fathomdb-memex-overall-roadmap/` |
| 2026-10-18 | The long-record study measures the 512-wordpiece cut: 19–37% of gold lies past it on session and document records, about 0% on turn records. Six fix options are costed; overflow chunking (C) is the cheapest that sees past the cut. | `dev/design/long-record-embedding.md` §§ 3, 5, 6 |
| 2026-10-19 | Owner accepts: ship a guard (coverage in `orient`, scored extent in snippets) and park C until a tripwire fires. | same, header |
| 2026-10-30 | The AC-013 investigation finds the vec0 overflow-chain cost and proposes CS64 (`chunk_size=64`). The vector stage under CS64 costs 1.4× more per row in the bit-KNN, so it loses to the old layout near 1.2M vector rows (extrapolated). | `dev/experiments/ac013-page-cache/RECOMMENDATION.md` |
| 2026-10-31 | Owner rules: no backward compatibility, and old stores are refused at open; format 35 is 0.8.28's single break. B2-8 (the guard) is approved, with its storage in format 35 and its reporting in 0.8.29 Block 2. Chunking stays parked, but readiness is built now. | `dev/plans/release-state-0.8.28.json` (`b2-8-guard-storage`, `overflow-chunking-readiness`); steward ledger seq 348 on `dev/0.8.28` |

## 3. Tripwires and how each is monitored

A tripwire reopens the question. It does not decide it: § 5 does.

| # | Tripwire | How it is noticed | Where it stands on 2026-10-31 |
| --- | --- | --- | --- |
| T1 | An answer-level evaluation on session- or document-unit stores shows a loss that traces to evidence past the cut. | The measurement is proposed for the 0.8.29 Slice 9 checkpoint. It needs no chunk vectors: compare answers for P2b's past-cut questions on the original and the rotated store with the existing judge harness. | Never measured. |
| T2 | The fusion weights or the default embedder change. | The pinned-constant test (0.8.29 Slice 20) on `RRF_K`, `RRF_WEIGHT_TEXT`, `RRF_WEIGHT_VECTOR`, `DEFAULT_EMBEDDER_NAME` and `MAX_SEQUENCE_TOKENS`. Its failure message names this file; whoever changes a constant updates the test and records the re-evaluation here. | k = 30, text : vector = 3 : 1, bge-small-en-v1.5, 512 wordpieces. |
| T3 | The exact-scan kernel (1–2M brief, Path A) lands, so the latency gate can pass. | The kernel's own slice must read this file; the back-link in the 1–2M roadmap says so. The release-plan checklist catches it too. | Not scheduled. E1 not run. |

**Not a tripwire, but evidence to collect when one fires:** Memex's real
record-length distribution (long-record-embedding.md § 9, question 1). A
store of turn-sized records gains nothing from chunking.

## 4. Costs the owner requires weighed

### 4.1 Gain

- **Vector arm:** +0.086 R@10 when past-cut gold is moved into the head
  (P2b).
- **Hybrid:** pooled 0.000, made of +0.030 [0.008, 0.060] on 4 of 133
  past-cut questions cancelling −0.022 in the other direction; about +0.01
  across the corpus.
- **Answer level:** unmeasured (T1).

### 4.2 Latency and size (option C on LongMemEval sessions)

| Measure | Today | Option C | Basis |
| --- | --- | --- | --- |
| Vectors per record | 1 | 5.1 (AP-News 2.3, IR-C 1.8) | long-record-embedding.md § 6 |
| Embed time on the 3090 | 3.5–5 min | 18–26 min | same |
| vec0 size | 31 MB | 160 MB | same |
| Vector-side p50 / p95 | 0.18 / 0.22 s | 0.80 / 1.05 s | same; **stale**, see § 4.4 |

### 4.3 The CS64 crossing

- CS64 is fastest for the vector stage up to about 1.2M **vector rows**
  (extrapolated; measured to 300k, 76 ms against 135 ms).
- Chunking counts vector rows, not records. At 5.1 vectors per record, a
  LongMemEval-shaped store reaches 1.2M vector rows at about 235k records.
  Turn-shaped stores (about 1.0) are unaffected.
- Past the crossing, R8 (a rerank that does not multiply chunks, such as
  F32T) is needed to keep CS64's gain. R8 costs a schema bump, erasure
  upkeep and about 70% more disk.
- The 1M vector-stage run went over the 10 GB disk budget on the measuring
  host; a chunked store reaches that size sooner.
- **Budgets it must fit:** AC-013 (10k, p50 ≤ 80 ms, p99 ≤ 300 ms, gated)
  and AC-083 (LongMemEval 100k, 80/300 ms, tracked).

### 4.4 Stale numbers that must be re-measured first

- The § 4.2 latencies were measured on `4ff9239ef` (2026-10-18), which
  predates Unit I's page-cache change and CS64. They do not describe
  format 35.
- The crossing in § 4.3 is an extrapolation from 300k.
- Re-measure both on a build containing 0.8.28 Slices 60 and 70, with
  overflow-multiplied row counts.

## 5. What is built for chunking

| Piece | Where | What it gives chunking |
| --- | --- | --- |
| Per-vector span table (`write_cursor`, window number, byte start, byte end, body length), head windows only | 0.8.28 Slice 60, format 35 | Overflow windows are new rows, not a new table: enabling C is a re-projection, not a format break. The guard reads the same table, so it reports chunk coverage once C is on. |
| `Embedder` truncation-offset method | 0.8.28 Slice 60 | Window boundaries come from the embedder, not re-tokenization. |
| Lifecycle and erasure keyed by record | 0.8.28 Slice 70 | Supersession, retirement, validity and erasure reach every vector of a record. |
| R8 benchmark at overflow-multiplied row counts | 0.8.28 Slice 60 | The chunk size is frozen knowing where a chunked store crosses. |
| One vector candidate per record, its best window | 0.8.29 Slice 50, fusion identity ADR | Chunks cannot crowd the candidate pool or be summed in fusion. |
| Snippet extent as a byte span, not a byte count | 0.8.29 Slice 80/90, Block 2 | A chunk hit can report its window as the anchor. |

What is **not** built: overflow windows, the grouping in the vector stage,
the `SnippetAnchor` choice (long-record-embedding.md § 7.3).

## 6. Re-evaluation procedure

1. Record which tripwire fired, with its evidence, in § 2.
2. Re-measure § 4.4 on the current format.
3. Answer § 4: the gain at answer level, the latency on AC-013 and
   AC-083, and the record count at which a store of the consumer's shape
   crosses 1.2M vector rows. Say whether R8 must precede chunking.
4. Run the preregistered experiment in long-record-embedding.md § 7.2,
   with its decision rule unchanged unless the owner rules otherwise.
5. Take the result to the owner. Record the ruling in § 2 and in the
   steward ledger, and close or retarget the standing todo.
