# Track Runner status

Current coordination state for the
[performance benchmarking program](PROGRAM.md). Track plans define the work;
[`experiments/`](../../experiments/README.md) holds execution receipts and the
append-only evidence index.

- **Last reconciled:** 2026-08-24
- **Branch:** `experiments/performance-0.8.23-plan-20260821`

## Immediate sequence

1. No run queued. MEMORY-01 is complete; remaining planned tracks require
   their declared trigger or missing input.

## Track status

| Track | State | Next action or condition |
| --- | --- | --- |
| [SAFETY-01](tracks/safety-01-campaign-controls.md) | Complete | Reuse its receipt and artifact checks for every run. |
| [TRACE-01](tracks/trace-01-projection-lifecycle-integrity.md) | Complete | Rerun only if projection lifecycle behavior changes. |
| [LOCOMO-01](tracks/locomo-01-self-characterization.md) | Directional decision accepted | Use reported GPU winner `hybrid_ce_alpha_10_pool_20` in ANSWER-01. The missing safe grid receipt limits reproducibility but does not require a confirming grid. |
| [PARENT-01](tracks/parent-01-parent-child-screening.md) | Directional decision accepted | Keep parent expansion opt-in; reopen only for a diagnosed answer-context gap. |
| [ANSWER-01](tracks/answer-01-shortlist-scoring.md) | Complete | The [paired live receipt](../../experiments/runs/answer-01-shortlist-live-20260822T1234Z-8a050808/record.json) retains A0; reopen only with a new retrieval treatment or larger confirmatory contract. |
| [SCALE-01](tracks/scale-01-tc5-fidelity.md) | Complete | Preserve the [GPU primary receipt](../../experiments/runs/tc5-gpu-primary-20260822T1605Z-2d574205/record.json); rerun only for input or fidelity-contract changes. |
| [CORPUS-01](tracks/corpus-01-gold-coverage.md) | Complete, limited | Preserve `seq-268` and the [result](2026-08-23-corpus-01-supersession-human-review-result.md); neither reviewed case qualifies as supersession gold. |
| [TEMPORAL-01](tracks/temporal-01-time-scoped-retrieval.md) | Synthetic validity complete, corpus blocked | [Eight exact TRACE boundary probes](../../experiments/runs/temporal-01-trace-validity-20260823T1625Z-af0c03f1/record.json) passed; upstream LongMemEval and TimelineQA releases lack an external validity-window manifest. |
| [EXTRACT-01](tracks/extract-01-semantic-memory.md) | Complete, limited | Preserve the [78-case receipt](../../experiments/runs/extract-01-knowledge-update-20260823T2236Z-59e805cb/record.json); retain raw A0 for knowledge updates until value-changing extraction consolidation exists. |
| [MEMORY-01](tracks/memory-01-native-mem0-comparison.md) | Complete | Preserve the [paired pass receipt](../../experiments/runs/fathomdb-vs-mem0-locomo-comparison-20260824T2140Z-01e702be/record.json); treat the multi-hop loss as a diagnosis, not a broad failure-set claim. |
| [SCALE-02](tracks/scale-02-local-first-envelope.md) | Complete | Preserve the `stream_default` production path, shipped reader defaults, decision receipt, and [implementation note](2026-08-23-scale-02-stream-default-implementation.md). |
| [LATENT-01](tracks/latent-01-late-chunking-feasibility.md) | Parked | Start only from a labelled cross-window failure set. |
| [GRAPH-01](tracks/graph-01-projection-characterization.md) | Planned | Start only from a labelled multi-hop failure set. |
| [GLOBAL-01](tracks/global-01-native-graphrag.md) | Parked | Wait for a named global-synthesis failure, an eligible graph or summary/map-reduce treatment, and native-run prerequisites. |
| [REASON-01](tracks/reason-01-native-hipporag2.md) | Parked | Wait for useful GRAPH-01 evidence and native-run prerequisites. |
| [SEARCH-01](tracks/search-01-ir-c-baseline.md) | Complete historical | Preserve as the lexical reference; no current run. |

## Board rules

- Record only current state, the immediate next action, and links to durable
  plans or receipts.
- Do not copy metrics, handoff histories, review dialogue, old blockers,
  credentials, corpus payloads, or model output into this board.
- Update the affected row when its state or next action changes. Git history
  preserves the previous board state.
