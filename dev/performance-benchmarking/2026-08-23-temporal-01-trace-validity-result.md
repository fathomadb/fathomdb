# TEMPORAL-01 synthetic TRACE validity result

**Run:** [temporal-01-trace-validity-20260823T1625Z-af0c03f1](../../experiments/runs/temporal-01-trace-validity-20260823T1625Z-af0c03f1/record.json)  
**Verdict:** complete

The fresh FTS-only database returned the exact expected logical-ID set for all
eight fixed `ReadView(valid_as_of=...)` probes. The three synthetic half-open
windows included each lower boundary and upper boundary; missing and unexpected
hit counts were both zero. Query latency was p50 0.299 ms, p95 0.378 ms, and
max 0.398 ms.

This is a deterministic world-time validity contract check. It does not make a
claim about external-corpus retrieval, answer quality, supersession, erasure,
or `history_as_of` behavior. LongMemEval and TimelineQA remain blocked for the
external comparison because their upstream releases do not provide the required
source-derived validity-window manifest.
