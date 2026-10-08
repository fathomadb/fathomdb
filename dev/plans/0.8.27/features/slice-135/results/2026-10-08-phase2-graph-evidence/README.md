---
title: Slice 135 Phase 2 paired graph evidence and erasure contract
status: AUDITED_PAIRED_GRAPH_EVIDENCE_ERASURE
target_release: 0.8.27
---

# Phase 2 paired graph, evidence and erasure contract — 2026-10-08

The [frozen protocol](../../phase2-graph-evidence-protocol.json), SHA-256
`49a848abab6e87d139cc24aee0468900cf3004654b60bf0c071538f2e85bb74f`,
was committed before the two [installed-wheel runs](run-commands.json). It
binds the synthetic [fixture](../../phase2-graph-evidence-fixture.json),
authored public-SDK tests, runner bytes, exact baseline and candidate source
identities, and wheel/native artifact hashes. Both versions wrote the same
graph, source evidence and retained control to separate fresh SQLite files
with default embedding disabled.

The [independent audit](audit.json) found both versions matched the authored
depth-one `{B,D}` and depth-two `{B,C,D}` graph results, the exact fact hit,
evidence reference and resolved canonical source bytes, and the two-node
source erasure report. The retained graph and absence of the erased text hit
held after erasure and after fresh reopen. Direct SQLite inspection found
exactly five expected retained canonical nodes, three expected edges, no
erased text in the content-storing `search_index_v2`, and integrity `ok` in
each retained database. This raw-table witness matters because an empty
search result alone would not prove physical erasure.

Ten [negative controls](negative-controls.json) used copies of the raw
archives and were rejected: wrong graph ID, wrong resolved source bytes,
erasure count, stale reopened hit, missing phase, wrong wheel, missing paired
receipt, changed retained body, inserted erased FTS content and altered
fixture. The original [baseline](raw-baseline/raw.json) and
[candidate](raw-candidate/raw.json) raw receipts and databases remained
unchanged. [SHA256SUMS](SHA256SUMS) binds the retained files.

This is a deterministic SDK and persisted-state contract cell. It does not
measure judged evidence sufficiency, memory usefulness, vector fidelity,
retrieval relevance or generated-answer quality. It does not inject a process
kill during an erasure commit or WAL checkpoint; the Phase 1 fault receipts
and residual positions remain separate under the [Phase 2 plan](../../plan.md#phase-2-qualification-and-execution-order).
