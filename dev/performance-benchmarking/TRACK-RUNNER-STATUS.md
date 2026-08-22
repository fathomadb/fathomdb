# Track Runner status

Current coordination state for the
[performance benchmarking program](PROGRAM.md). Track plans define the work;
[`experiments/`](../../experiments/README.md) holds execution receipts and the
append-only evidence index.

- **Last reconciled:** 2026-08-22
- **Branch:** `experiments/performance-0.8.23-plan-20260821`

## Immediate sequence

1. **SCALE-01:** implement TC-5 v2, then run the 7,667-document GPU smoke and
   17,272-document GPU primary.
2. **SCALE-02:** after SCALE-01, run A0's
   advisory 10k-to-50k efficiency envelope.

CORPUS-01 may resolve its two insufficient supersession cases independently.

## Track status

| Track | State | Next action or condition |
| --- | --- | --- |
| [SAFETY-01](tracks/safety-01-campaign-controls.md) | Complete | Reuse its receipt and artifact checks for every run. |
| [TRACE-01](tracks/trace-01-projection-lifecycle-integrity.md) | Complete | Rerun only if projection lifecycle behavior changes. |
| [LOCOMO-01](tracks/locomo-01-self-characterization.md) | Directional decision accepted | Use reported GPU winner `hybrid_ce_alpha_10_pool_20` in ANSWER-01. The missing safe grid receipt limits reproducibility but does not require a confirming grid. |
| [PARENT-01](tracks/parent-01-parent-child-screening.md) | Directional decision accepted | Keep parent expansion opt-in; reopen only for a diagnosed answer-context gap. |
| [ANSWER-01](tracks/answer-01-shortlist-scoring.md) | Complete | The [paired live receipt](../../experiments/runs/answer-01-shortlist-live-20260822T1234Z-8a050808/record.json) retains A0; reopen only with a new retrieval treatment or larger confirmatory contract. |
| [SCALE-01](tracks/scale-01-tc5-fidelity.md) | Active | Add the 0.8.23 GPU-primary controls and run TC-5. CPU is an optional 7,667-document, twelve-worker equivalence bridge. |
| [CORPUS-01](tracks/corpus-01-gold-coverage.md) | Narrow follow-up | Human-review the two insufficient supersession cases; do not broaden the corpus search. |
| [TEMPORAL-01](tracks/temporal-01-time-scoped-retrieval.md) | Planned | Wait for CORPUS-01-qualified temporal gold and the selected retrieval baseline. |
| [EXTRACT-01](tracks/extract-01-semantic-memory.md) | Planned | Wait for TRACE-01 lifecycle coverage, qualified update gold, and the selected canonical baseline. |
| [MEMORY-01](tracks/memory-01-native-mem0-comparison.md) | Blocked | Use A0; wait for native comparator prerequisites. |
| [SCALE-02](tracks/scale-02-local-first-envelope.md) | Blocked | Use A0; wait for the SCALE-01 receipt and workload registration. |
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
