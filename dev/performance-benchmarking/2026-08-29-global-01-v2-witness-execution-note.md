# GLOBAL-01 v2 witness execution note

**Status:** invalid witness; stopped without held-out execution.

Fresh A/A passed. The witness completed six control map cells, then three
schema-correct responses each exceeded the registered 30-word claim limit.
The deterministic retry loop resent the identical prompt without telling the
model why validation failed. It exhausted the semantic boundary at
$0.13629452 with zero complete witness answers and zero held-out answers.

This is an execution-control failure, not a quality result. Attribution was
valid: every failed response used compact `source_refs`, and the v2 adapter was
not the cause.

The correction keeps the measurement contract unchanged. Semantic retries now
append only the content-free validator error to the original prompt and ask
for corrected JSON. Invalid response content is neither persisted in metadata
nor replayed. A new semantic revision and fresh artifact root prevent reuse of
the failed cells.

## Records

- [Invalid v2 witness receipt](../../experiments/runs/global-01-lazy-witness-20260829T2047Z-483e11ad/record.json)
- [Authorized v2 preflight receipt](../../experiments/runs/global-01-lazy-preflight-20260829T2045Z-483e11ad/record.json)
- [Recovery contract](2026-08-29-global-01-v2-recovery-contract.md)
