# Performance experiment track plans

This directory contains one planning charter for every row in
[the program board](../PROGRAM.md). A charter makes a track understandable and
preparable; it is not permission to execute an experiment. Before execution,
the charter must name a frozen typed configuration, an external artifact root,
a run identifier policy, a cost limit where applicable, and an eligibility
rule. Every completed run still writes the common `experiments.record.v1`
receipt and one append-only `experiments.index-row.v1` row.

Every charter is run through [Track Runner](../TRACK-RUNNER.md). It defines the
coordinator/worker/reviewer handoff, the two-writer WIP limit after the trace
canary, and the external-execution approval boundary. Read the charter through
`./scripts/track-runner.sh brief <TRACK-ID>` before starting its preparation.
Read [the live status board](../TRACK-RUNNER-STATUS.md) first; only the
coordinator updates it after a worker handoff or review gate.

## Identifier migration

| Current ID | Earlier identifier | Track charter | Existing detailed document |
| --- | --- | --- | --- |
| SAFETY-01 | C0 | [campaign controls](safety-01-campaign-controls.md) | [execution runbook](../2026-08-14-experiment-campaign-execution-plan.md) |
| TRACE-01 | new | [projection lifecycle integrity](trace-01-projection-lifecycle-integrity.md) | New dated contract required |
| LOCOMO-01 | L0 | [LOCOMO self-characterization](locomo-01-self-characterization.md) | [LOCOMO campaign](../2026-08-14-locomo-fathomdb-capability-campaign-plan.md) |
| PARENT-01 | new | [parent-child screening](parent-01-parent-child-screening.md) | New plan or LOCOMO amendment required |
| SCALE-01 | T0 | [TC-5 scale fidelity](scale-01-tc5-fidelity.md) | [TC-5 plan](../2026-08-14-eu7-tc5-scale-envelope-rebaseline-plan.md) |
| CORPUS-01 | new | [agent-memory gold coverage](corpus-01-gold-coverage.md) | New dated contract required |
| ANSWER-01 | L1 | [LOCOMO answer scoring](answer-01-shortlist-scoring.md) | [LOCOMO campaign](../2026-08-14-locomo-fathomdb-capability-campaign-plan.md) |
| MEMORY-01 | M0 | [native Mem0 comparison](memory-01-native-mem0-comparison.md) | [Mem0 readiness](../plans/2026-08-11-mem0-oss-baseline-readiness.md) |
| SCALE-02 | F0 | [local-first scale envelope](scale-02-local-first-envelope.md) | [execution runbook](../2026-08-14-experiment-campaign-execution-plan.md) |
| LATENT-01 | new | [late-chunking feasibility](latent-01-late-chunking-feasibility.md) | New dated contract required |
| GRAPH-01 | new | [graph-projection characterization](graph-01-projection-characterization.md) | New dated contract required |
| GLOBAL-01 | G0 | [native GraphRAG comparison](global-01-native-graphrag.md) | [GraphRAG readiness](../plans/2026-08-11-graphrag-baseline-readiness.md) |
| REASON-01 | H0 | [native HippoRAG-2 comparison](reason-01-native-hipporag2.md) | [HippoRAG-2 readiness](../plans/2026-08-11-hipporag2-baseline-readiness.md) |
| SEARCH-01 | I0 | [IR-C baseline retention](search-01-ir-c-baseline.md) | [initial population](../2026-08-11-initial-population-plan.md) |

## Common preparation checklist

1. Reconcile the worktree with canonical main and record the commit, lockfile,
   interpreter, model identity, host/device, and relevant runner hashes.
2. Write failing, human-intended tests for new configuration parsing, provenance,
   receipt, or metric behavior before implementation.
3. Put corpus payloads, databases, predictions, logs, and priced-model output in
   an access-controlled external root. Commit only safe receipts and hashes.
4. Validate a small fixed-subset dry run before any full or paid treatment.
5. Treat blocked prerequisites and incomplete runs as evidence. Do not convert a
   partial run into a product, latency, comparator, or support claim.
