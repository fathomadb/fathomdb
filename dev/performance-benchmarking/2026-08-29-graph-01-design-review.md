# GRAPH-01 design review

**Verdict:** approved for implementation after the corrections below.

| Finding | Resolution |
| --- | --- |
| The draft track repeated the already-rejected lexical-seeded graph expansion. | Replaced it with exact-anchor, shortest-path bridge completion. Raw BFS, PPR, graph RRF, and graph-only retrieval are explicitly excluded. |
| The old native M1 graph stored question IDs, not paragraph IDs, as edge provenance. | Rebuild a fresh 0.8.23 database and write exact paragraph IDs on every edge. Retrieval reads the native projection back. |
| Extractor confidence was absent and cannot be invented after the fact. | Use a deterministic support-eligibility filter and an independent, question-blind edge-precision audit. Do not write fabricated confidence values. |
| A lexical-only baseline would be a strawman. | Retain the frozen BM25 plus CLS-dense RRF comparator and equal top-ten context budget. |
| Graph promotion could discard good context. | Protect control ranks one through eight, cap promotions at two, and require candidates to remain in the control top twenty. |
| The reused cohort has been observed in earlier work. | Label the result as a registered reused-cohort characterization, not pristine held-out evidence. |
| Paid execution could repeat earlier rate-limit losses. | Require atomic per-cell checkpoints, resume-only-missing behavior, worst-case cost reservation, and `Retry-After` handling. |
| Zero-spend preflight found one missing paragraph extraction in the pinned cache. | Keep the document in both arms and treat its graph contribution as empty. Do not mix extractor models or use its supporting label to repair it. |

No unresolved design finding blocks RED tests.
