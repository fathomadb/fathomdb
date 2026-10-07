---
title: Slice 135 installed TypeScript S02 functional feasibility
status: PAIRED_FUNCTIONAL_FEASIBILITY_NO_LATENCY_VERDICT
target_release: 0.8.27
---

# Installed TypeScript S02 functional feasibility — 2026-10-07

The [installed-package runner](runner.mjs) completed one fresh real-database
S02 sequence through each version: 0.8.26 source
`f99e002f0d2e4002f3694c9f8d4986b56089edaa` and repaired candidate
`438cff2995a489f9d5b4e3d6a77fd3b63670bc7d`. Both used Node v25.9.0,
the same [32-record S01 corpus](s01-helper.mjs), and a canonical source with
two derived nodes and a `supports` edge. The candidate's locally built
[main package](candidate-fathomdb-0.8.26.tgz) and
[native package](candidate-fathomdb-linux-x64-gnu-0.8.26.tgz) are retained;
the baseline archives are in the [TypeScript noise pilot](../2026-10-07-ts-s01-noise-pilot/).
Both packages still declare version 0.8.26, so the source SHA and native
binary hash are essential identifiers.

Each sequence opened the installed SDK with the default embedder, wrote the
corpus and graph, configured a searchable vector projection, drained to
`ready`, and checked anchors. It exercised text, vector-bearing and hybrid
retrieval; frozen-context evidence search/resolution; graph expansion and
target/edge evidence resolution; source erasure and idempotent second erasure;
and close/reopen. After erasure and reopen, the SDK found the graph content
absent and retained corpus content present. An independent SQLite connection
counted **0 graph nodes, 0 graph edges and 32 retained corpus nodes**.

The [raw baseline](baseline.json) and [candidate](candidate.json) observations
were semantically identical. The [independent audit](audit.py) checked those
observations, installed module/package/native bytes against the retained npm
archives, source and runner identities, positive stage timers and empty
stdout/stderr. Its [result](audit.json) rejects a retained-edge negative
control. The full sequence wall timer includes validation checks: 5,112.422
ms baseline and 5,272.786 ms candidate. These are **one observation per
version**, with [baseline](baseline.resource.txt) and
[candidate](candidate.resource.txt) resource reports; they establish neither
latency equivalence nor a regression. No contention condition ran. S02's
sampled latency and robustness cells remain open.

The runner's focused fixture test first failed because the runner was absent,
then passed after implementation. Its negative controls reject wrong
canonical evidence, a missing vector branch and a retained edge. The receipt
does not qualify all error, filter, lifecycle or provider behavior across the
TypeScript SDK. The [SHA-256 manifest](SHA256SUMS) seals the retained files.
