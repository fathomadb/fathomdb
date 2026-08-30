# GLOBAL-01 lazy-coverage held-out result

The registered 39-question held-out comparison rejected
`global_lazy_coverage_v1`. The treatment did not meet the all-boundary quality
rule: its pairwise win rates were 0.353 for comprehensiveness, 0.385 for
diversity, and 0.451 for empowerment, and qualified-assertion recall changed
from 0.519 to 0.504.

The treatment did improve the unsupported-claim rate from 0.063 to 0.015,
directness remained eligible, its generation cost was 0.437 times control,
and its end-to-end p95 was 0.830 times control. Canonical attribution was
complete and both lifecycle canaries passed. Those operational and grounding
gains do not override the registered headline-quality and assertion-recall
failures.

Retain `source_mapreduce_c_v1_fts50` for this query shape. Do not promote
`global_lazy_coverage_v1`, and do not tune it against the held-out outcomes.
The complete typed evidence is the
[GLOBAL-01 receipt](../../experiments/runs/global-01-lazy-coverage-20260829T2159Z-60b3642c/record.json).
