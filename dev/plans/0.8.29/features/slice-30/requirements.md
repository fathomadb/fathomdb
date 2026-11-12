# Slice 30 requirements

These are release-local measurement requirements, not new canonical ACs.

| ID | Requirement and reason | Acceptance |
| --- | --- | --- |
| R30-1 | Reconcile actual integrated20/60/70 source and current format37 before tool adaptation; old ELIG/FTS/armlimit prototypes are historical evidence. | A30-1 |
| R30-2 | Measure100k natural-text direct search, selected input/order and match/body-length distributions; report tracked AC-083 text targets only. | A30-2 |
| R30-3 | Separate read-mostly from committed write/batch-write→search schedules and drained from projection-overlap cells; authority/time change is observed, never inferred from wall time. | A30-3 |
| R30-4 | Attribute only actually observed work. Distinguish dependency presence, direct eligibility, source fallback, FTS, vector work, query embedding, SDK and MCP queue/open costs. | A30-4 |
| R30-5 | Preserve current counters/hoists and provider/queue contracts. Missing observer data cannot become zero-cost evidence or a new optimization. | A30-4 |
| R30-6 | Keep each measurement tool committed/indexed, configurable and rerunnable; test counts/math/schedule/fresh-store/resume, interrupted raw-prefix preservation and actual real-DB routes. | A30-5 |
| R30-7 | Own all mutable resources under/mnt/data; bound process/storage/memory/time and record native/source identities, errors and outliers. | A30-6 |
| R30-8 | Distinguish authorized P, staged G/D, held M/F and Windows20 cells; partial Linux evidence never closes20/30/Block1 or clears downstream source gates. | A30-7 |
| R30-9 | Preserve accepted AC075 TC5 frozen bridge separately from relevance and latency:7667 real docs/100queries/K192/top10/exact-f32 truth, CI-high>=0.90. | A30-7 |

[REQ-010/011](../../../../requirements.md#performance-req-009req-018) provide
historical performance context; the current AC075/076 reconciliations and
release plan govern their operative measurements. REQ-013/014 freshness remains
an independent write→search correctness obligation, not a timing allowance.
The [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
applies real-database boundaries, explicit independent oracles and resource
release. General fuzzing, fault injection and exhaustive matrices are deferred;
no such investment is needed to establish this bounded measurement baseline.

## Concrete G/D extension requirements

These remain local measurement requirements, mapped to existing A30 rather than
new canonical gates. [Extension](gd-extension.md), [tests](gd-test-map.md) and
[preregistration](gd-preregistration.md) define exact source/cell bounds.

| Requirement | Existing acceptance mapping |
| --- | --- |
| G1: built-in pinned BGE384, normal core/SDK hybrid/direct-text/mean-embed populations on the same current store; actual reserved3090 engagement, no CLS/CPU/model substitution | A30-1/2/6 |
| G2: filtered read-only control versus exact governed write/drain/proof schedule, actual counts/revisions and separate API/loop costs | A30-3/4 |
| G3: actual engine-owned bounded per-query snapshot/predicate/arm observations; called versus skipped/null, inclusive spans, normal/feature-off/on identities and paired raw observer cost | A30-4/5/6 |
| D1: existing contained deterministic768-d fixture, current worker predicates/defaults, held versus actual overlap, durable pending/vector coverage and per-worker/WAL/drain observations | A30-1/3/4 |
| D2:10k/100k actual feasibility before conditional300k, explicit resource stop/incomplete/NOT_RUN and unchanged numerical gates | A30-6/7 |
| GD: per-behavior genuine committed runtime RED before separate GREEN, maintained suite selection, own/mnt/data resources and one stable source phase/full gate, retained errors/skips/subjects | A30-5/6/7 |
