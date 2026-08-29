# GLOBAL-01 lazy-coverage preflight result

**Status:** ready for HITL execution authorization.

The zero-spend preflight passed on the complete 1,397-article AP News corpus
and all 49 qualified questions. The isolated CLI and Python runtimes are
FathomDB 0.8.23. Every question returned 50 control candidates; cold and steady
retrieval p95 were 18.14 ms and 17.81 ms, respectively.

Strict-current supersession, both temporal boundaries, source erasure, and zero
derived persistence passed. Airlock exposes the registered `deepseek-v4-pro`
and `claude-haiku` aliases. The paid CLI checkpoints every cell, resumes only
missing cells, honors numeric and HTTP-date `Retry-After`, serializes calls,
reserves worst-case request cost before submission, and refuses an incomplete
verdict.

The registered configuration SHA-256 is
`6da51962c5411faecabc963d71f01c4d16c5ea2891da56d7eb5b4b2a620329a0`.
Projected spend is $9.50 with a recommended $12.00 hard cap, 1,376 planned paid
cells, and at most 4,128 semantic-format submissions. No paid call has been
made.

Next: obtain HITL authorization for the $12 cap, run A/A judge validation, and
proceed to the three-question witness only if A/A passes.

This receipt supersedes the zero-spend `8567b830` preflight receipt, whose
configuration understated the planned paid-cell count. No model call occurred
under either configuration.

- [Measurement contract](2026-08-29-global-01-lazy-coverage-contract.md)
- [Safe preflight receipt](../../experiments/runs/global-01-lazy-preflight-20260829T1841Z-6da51962/record.json)
