---
title: FathomDB 0.8.28 Slice 70 — status
status: COMPLETE
target_release: 0.8.28
qualified_sha: 398d5c1edaf58d0d2dd68ac13136f3939130059a
---

# Slice 70 status

Complete on `dev/0.8.28` and `release/0.8.28`. Qualified source:
`398d5c1edaf58d0d2dd68ac13136f3939130059a`.
[Plan and acceptance](plan.md), [design](design.md) and the
[format ADR](../../../../adr/ADR-0.8.28-store-format-35.md) reconcile the draft,
landed Slice 60 work, D28-12 and the additional actuation mutation path.
Astra approved requirements/design, implementation and the final erasure repair.

Vec0 now stores lifecycle and inclusive validity metadata. Native filtering
precedes the 192-candidate cap, so superseded, inactive, future and expired
vectors no longer disable unrelated dense results. Source-dependency fallback
remains. Relaxed node windows preserve unconditional edge expiry through
separate native arms and the same global cap. Frozen-time bindings, f32
rerank and public flag admission are unchanged. Slice 80 owns historical
membership and search flags.

Canonical mutations maintain all mapped vector IDs transactionally, including
actuation and future consolidation deadlines. Erasure, attribute refresh and
backfill use record mappings; orphan detection uses vector identity. Rebuild,
reshape and recenter preserve metadata and spans. Ten filterable attributes
work; an eleventh fails atomically. Pre-extension format-35 layouts are refused
without migration. Future overflow generation remains deferred.

Observed RED/GREEN commits cover physical metadata, lifecycle upkeep, all four
ineligible-distractor causes, record-mapped erasure/backfill, orphan identity,
and relaxed-window edge expiry. Final adversarial review found that an unmapped
head could survive a successful purge. RED `5a7628a0c` reproduced that public
failure; GREEN `398d5c1ed` refuses it transactionally, preserves canonical/FTS
state and protects vectors mapped to another owner. Additional endpoint,
property, rollback, capacity, actuation and consolidation tests are coverage,
not fictitious RED evidence.

Independent final verification passed 64 focused tests, including ten Slice 70
integration and ten unit/property tests. AC-013 passed at 10k vectors and 1,000
samples: p50 55 ms / p99 63 ms against 80 / 300 ms. No numerical algorithm or
layout-sweep decision changed; the Slice 60 sweep remains its historical
snapshot rather than evidence for the added metadata.

The full gate campaign completed through scoped repairs and reruns. Lint and
typecheck passed; strict unconfined security reported zero violations, blockers
and downgrades. The 205-suite aggregate initially passed 192 and failed 13.
Twelve executor-restricted cases subsequently passed their unchanged targeted
unconfined commands, including the straced CLI doctor, TypeScript (594 passed,
six expected skips) and NAPI build fixture. The remaining failure was the new
standalone test fixture's source classification: moving it under `cfg(test)`
made the exact WAL ownership audit pass without relaxing production guards.
Engine workspace and Python tests passed. This is a completed gate campaign,
not a claim that the original invocation passed. The final erasure repair
received scoped independent lifecycle/dependency/erasure tests and Clippy;
no full regression restart was needed.

Raw logs and rerun summary are under `/tmp/slice70-*`, including
`/tmp/slice70-rerun-summary.json`. Final records receive Markdown, generated-view
and closed-dependency checks. No new worktree or branch was created. The
existing release branch is fast-forwarded to the same closeout records.
Slice 80 is next; no publication is authorized.
