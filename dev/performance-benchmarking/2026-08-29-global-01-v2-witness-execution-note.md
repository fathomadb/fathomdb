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
the failed cells. The post-correction
[zero-spend preflight](../../experiments/runs/global-01-lazy-preflight-20260829T2053Z-483e11ad/record.json)
passed.

On the fresh retry, the map correction worked: one over-limit map response
self-corrected, and all ten control map cells completed. The control reduction
then reached exactly 1,500 completion tokens on all three attempts and returned
truncated JSON. Execution stopped at $0.18734320 with no complete answer and no
held-out work. The generic JSON parse position did not tell the model that it
had to shorten its response.

The second correction classifies a parse failure at the exact completion-token
ceiling as an output-limit failure. Its content-free retry instruction requires
complete JSON with shorter prose and all schema entries retained. A new
semantic revision isolates these attempts; the bound checkpoint may resume its
completed A/A and map cells after a fresh zero-spend code binding.

That [binding passed](../../experiments/runs/global-01-lazy-preflight-20260829T2059Z-483e11ad/record.json)
at zero spend.

The bound resume classified all three new failures as output-limit failures,
but explicit shortening feedback still produced exactly 1,500 tokens. Spend
reached $0.21801604 with no complete witness answer and no held-out execution.
The [model-limit review](2026-08-29-global-01-output-limit-research.md) closes v2
and moves the unchanged matched comparison to a 4,096-token v3 reduction
ceiling.

## Records

- [Invalid v2 witness receipt](../../experiments/runs/global-01-lazy-witness-20260829T2047Z-483e11ad/record.json)
- [Invalid post-correction witness receipt](../../experiments/runs/global-01-lazy-witness-20260829T2054Z-483e11ad/record.json)
- [Authorized v2 preflight receipt](../../experiments/runs/global-01-lazy-preflight-20260829T2045Z-483e11ad/record.json)
- [Recovery contract](2026-08-29-global-01-v2-recovery-contract.md)
