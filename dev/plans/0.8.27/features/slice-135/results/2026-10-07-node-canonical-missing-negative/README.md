---
title: Slice 135 missing canonical node authority negative probe
status: PASSING_REAL_DATABASE_NEGATIVE_PROBE
target_release: 0.8.27
---

# Missing canonical node authority probe — 2026-10-07

The [real-database test](test-source.rs) at
`ef1f67c6f15c6cda5d26a85f3f0b2116bb800045` writes and indexes a node,
verifies its normal search result, then drops the current-schema
`canonical_nodes` table while its FTS row remains. The FTS `MATCH` still sees
the orphaned row. The public text search returns `EngineError::Storage`, as
required when canonical authority is unavailable. The [raw run](run.log)
reports one passing focused test on the exact committed source.

This probe confirms the **observable fail-closed result for this fault**. It
does not prove which internal query first produced the error or execute the
pre-step-8/pre-step-12 fallback branches. Those branches remain source-audit
leads, not confirmed defects. The [SHA-256 manifest](SHA256SUMS) seals the
retained source and run output. This is exception-handling evidence, not a
latency measurement or a full Phase 1 gate.
