---
title: Slice 135 missing edge FTS index fail-closed repair
status: FOCUSED_REGRESSION_PASSED_FULL_GATE_PENDING
target_release: 0.8.27
---

# Missing edge FTS index repair — 2026-10-07

The edge-body search arm suppressed SQL prepare and query errors. On a damaged
current-schema database whose `search_index_edges` table was removed, a real
database [RED test](test-source.rs) first found the committed edge-body hit,
then repeated the same search after dropping the table. On pre-fix source
`d22fa9eefa0e75e9acdb9cb3d8a7bbdd13bdbca0`, search returned `Ok` with an
empty result list instead of reporting a storage failure. The [raw RED log](red.log)
records that assertion failure and [exit 101](red.exit).

The [committed fix](fix.patch) at
`55f8120f5fff44414f8ee57ef987d40b9f4306ff` propagates statement
preparation and query errors from the edge arm. The prior fallback comment
described schemas before the edge index migration; a supported `Engine::open`
migrates the database to the current schema, so silently treating the absent
table as an empty edge result was wrong. The [GREEN log](green.log) and
[resource report](green-resource.txt) were captured on the exact committed
source: the regression, previous edge-row-error regression, and 13 adjacent
attribute-filter tests passed (**15 tests**). Rust formatting passed. The
full workspace gate remains pending at the next consolidated source checkpoint.

The [SHA-256 manifest](SHA256SUMS) seals the retained artifacts. This is a
confirmed logic and exception-handling defect repair, not a new latency
comparison. Candidate performance artifacts from earlier SHAs need rebuilding
before the Phase 1 checkpoint.
