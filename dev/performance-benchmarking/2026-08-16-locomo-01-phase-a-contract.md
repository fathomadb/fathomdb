# LOCOMO-01 Phase-A preparation contract

**Status:** prepared; not authorization to execute.

This dated contract makes the active `LOCOMO-01` charter's Phase-A inputs
machine-checkable. It consumes historical evidence only and introduces no
receipt, corpus artifact, model invocation, or benchmark result.

## Authority boundary

The typed catalog is [phase-a-grid.v1.json](../../experiments/configs/locomo-01/phase-a-grid.v1.json).
It declares `execution.mode = plan_only` and `live_execution = forbidden`.
`experiments.locomo_phase_a` deliberately exposes `validate` and `preview`, but
no command that can acquire a corpus, launch a harness, start a façade, select a
GPU, invoke a model, or write an external artifact. Its `run()` API always
raises.

Phase-A preparation does not authorize the fixed-subset dry run or full grid.
Those require a later, explicit user authorization and safe receipt/index
handling under the existing experiment contract.

## Frozen inputs

| Input | Pinned value | Source |
| --- | --- | --- |
| Canonical A0 | turn-level FTS, top-10 | `locomo-capability-a0-baseline-20260814T2311Z-d4a71071` |
| A0 source arm | `fathomdb-locomo-official-seam-20260814T2303Z-fb622897` | historical official-seam receipt |
| Corpus | LOCOMO raw `79fa…698ff4c`, normalized `e999…fc4c7c`; 272 sessions; external-only | historical official-seam receipt |
| Turn manifest | `locomo-turn-provenance.v1`, `43453c…a78f66` | Phase-A preflight receipt |
| Session manifest | `locomo-session-provenance.v1`, `46e928…c6e4` | Phase-A preflight receipt |
| M1 | paired R@10 lower 95% bound at least `-δ`; `δ = 0.02405130733344985` | A0 margin contract |
| Bootstrap | seed `20260814`, 10,000 resamples | A0 margin contract |

The complete 64-character hashes, not the abbreviated display values above, are
the configuration's normative values. Corpus payloads and provenance manifests
remain external and are never copied into this repository.

## Planned grid and measurement semantics

The catalog expands 2 ingest units × 6 retrieval treatments × 4 runtime cells =
48 explicit planning cells. Ingest units are `turn` and `session`; treatments
are FTS-only, hybrid, three bounded cross-encoder profiles, and FTS with bounded
neighbor expansion. Every treatment receives distinct `cpu/cold`,
`cpu/steady`, `gpu/cold`, and `gpu/steady` cells.

Every later executable cell must preserve these reporting dimensions:

- M1: R@10 and the pinned paired acceptance rule.
- M2: MRR, R@1, and nDCG@10.
- M4 proxy: temporal evidence recall; actual judge-scored M4 remains unmeasured.
- M6: façade and engine query timings.
- M7: ingest acknowledgement and ready-to-search timings.

Fast-local requires an M1 pass and p95 at most `1.5 × A0 p95`. GPU timing is
report-only for LOCOMO; it is not a LOCOMO acceptance claim.

## Readiness and review

Preparation is ready for review when the catalog validates, previews exactly 48
unique cells, preserves all pins above, and refuses live execution. The review
must check that no shared helper, historical config, receipt/index row, or
external artifact was changed; that the catalog remains content-free; and that
future execution cannot mistake preparation for authorization.
