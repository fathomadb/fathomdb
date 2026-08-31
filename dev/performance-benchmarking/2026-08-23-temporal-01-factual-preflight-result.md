# TEMPORAL-01 factual-preflight result

## Result

The content-free input preflight completed with state
`input_facts_confirmed_treatment_blocked`. Its safe report SHA-256 is
`eaee5eb596333e30f11be0be071e89df7ff0608e7e4bc20238dfae2903b84264`.

| Corpus | Confirmed input fact | Measurement boundary |
| --- | --- | --- |
| TimeQA | Held-out easy/hard test files: 6,075 questions; payload hashes match the registry. | No unanswerable records in this selection. It supports time-sensitive answer reporting, not abstention or `valid_as_of` without a derived validity window. |
| LongMemEval-S | 500 instances, including 133 dated `temporal-reasoning` questions; payload and MIT license hashes match the registry. | Session ordering is available, but a validity window is not yet derived. |
| TimelineQA | Generator pin and all three density counts match the registry: 2,458,339 atomic QA. | Evaluation-only. Its generated payload needs an external selected-manifest and source-derived validity-window mapping. |

## Blockers

No database or measurement was released. The required next artifacts are:

1. An external, deterministic selected-record/evidence manifest that defines
   source identities, timestamps, validity windows, and exclusions without
   copying payload into Git.
2. A temporal adapter that writes those source-derived validity windows and
   exercises the single treatment `ReadView(valid_as_of=...)`.

The future comparison remains bounded to world-time validity. FathomDB does not
provide `history_as_of` search, and its search APIs intentionally refuse
supersession-history relaxation; this result makes no historical-state,
supersession, erasure, or product claim.
