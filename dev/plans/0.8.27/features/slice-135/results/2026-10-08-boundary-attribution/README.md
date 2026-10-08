---
title: Slice 135 exact-source engine and installed SDK query-boundary attribution
status: DIAGNOSTIC_ATTRIBUTION_NOT_CAUSAL_PROOF
target_release: 0.8.27
---

# Query-boundary attribution — 2026-10-08

The source-bound [E01–E12 engine](../2026-10-08-e12-224e-paired/README.md),
[Python S01](../2026-10-08-python-s01-224e-paired/README.md) and
[TypeScript S01](../2026-10-08-ts-s01-224e-paired/README.md) comparisons all
use candidate `224e44c593c13d86ece648adabe445723db04070` against baseline
`f99e002f0d2e4002f3694c9f8d4986b56089edaa`. Their ordered paired
changes cannot be treated as repeated measures of one call.

| Boundary | Work and corpus | Candidate query scale | Median five-pair p50 change |
| --- | --- | ---: | ---: |
| E01–E12 vector stage | Engine call, 32 documents, deterministic synthetic provider, test-only pre-fusion vector return | About 0.39 ms | +15.67% |
| E01–E12 hybrid | Engine call, same synthetic provider and `needle` query | About 0.39 ms | +14.54% |
| Python S01 vector | Installed materialized call, real CPU BGE model, 32/256-row JSON corpus and semantic query | About 10/11.5 ms | −3.425%/−0.045% |
| TypeScript S01 vector | Installed async materialized call, same model and 32/256-row corpus | About 10.9/12.0 ms | +2.3715%/−0.7761% |

The engine vector-stage hook excludes fusion and uses a different query and
provider. The synthetic provider's recorded median call time across the five
candidate query blocks was about **0.00042 ms**, versus about **0.393 ms**
median whole engine vector-stage time. Installed Python and TypeScript use
the real BGE CPU model and include binding work, yielding a much larger
boundary. A 0.05 ms engine-stage increase cannot be subtracted from a
10–12 ms installed call to explain its change: model, corpus, path and timer
are not matched. The 256-row Python vector delta is near zero relative to
pair variation; the TypeScript 256-row direction is negative but has only two
warning-free pairs. The engine's positive vector and hybrid leads survive
four warning-free pairs, so the stage remains an investigation target.

The E01–E12 populated-open p50 changed +12.14% across the five pairs, from
roughly 12.68 to 14.11 ms in the median block. Installed S01 excludes open
and model load. Installed S02 includes both open and close within a roughly
5.3-second whole sequence; stage and lifecycle receipts are needed before
assigning that whole-sequence difference to populated open. The earlier
whole-process futex trace at source `3f29d649d` is only a queue-wait
hypothesis; it is neither exact-source nor query-only evidence. Unprofiled
latency remains separate from any later CPU or queue trace.
