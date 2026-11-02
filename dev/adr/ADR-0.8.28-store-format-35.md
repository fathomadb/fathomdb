---
title: ADR-0.8.28 — Store format 35 vector layout and coverage
status: accepted; owner scope approved and Slice 60 qualification complete
date: 2026-11-01
target_release: 0.8.28
---

# Store format 35 vector layout and coverage

Successor to [ADR-0.7.0-vector-binary-quant](ADR-0.7.0-vector-binary-quant.md)
for physical chunk layout and vector identity. The binary-candidate plus
original-f32 rerank algorithm, candidate count, mean-centering and recall
floor remain unchanged. Owner scope is recorded in the 0.8.28 master plan
and release state (2026-10-31).

Per-connection caches expose vec0's large chunk overflow-chain cost.
Format 35 uses `chunk_size=64` in all vector partition creation paths,
qualified by Slice 60's 64/128/256 comparison through 2M vector rows and real
BGE TC-5 recall qualification. It also pins node body FTS rowid to write cursor.
Version-34 stores are refused at open without migration or compatibility
shims. Operators rebuild projections only within a current-format store.

The format separates vector ID from record cursor, removes the one-vector
storage uniqueness restriction, and records head-window byte coverage in a
per-vector span table. Embedder-reported UTF-8 offsets distinguish unknown
coverage from full coverage. The public trait addition advances independent embedder-api Axis E from
0.7.0 to 0.8.0. Spans participate in physical completeness and read-visibility
validation. Only the head is produced. Overflow chunking,
record-grouped candidate selection and lifecycle upkeep for future multiple
windows remain assigned follow-on work; this ADR does not enable them.

The [Slice 60 design](../plans/0.8.28/features/slice-60/design.md) specifies
constraints, transactional upkeep and the trait contract. The
[plan](../plans/0.8.28/features/slice-60/plan.md) fixes the measurement rule.
The [qualified status](../plans/0.8.28/features/slice-60/status.md) records
passing AC-013, the unchanged TC-5 recall criterion, all 30 scale cells and
cutoff ties. The historical CS64-versus-1024 crossing was not remeasured.
No F32T copy, ANN structure, cache increase or embedder swap is introduced.

## Slice 70 native eligibility

The unpublished format-35 layout adds plain INTEGER `lifecycle`, `valid_from`
and `valid_until` vec0 metadata. Lifecycle preserves inactive/superseded bits;
zero is current and active. Nullable canonical bounds become inclusive
min/max-sentinel bounds, including an exact empty interval for finite MIN.
Edges derive their upper bound from `t_invalid`. Canonical mutations update
all vector IDs of the record atomically. Rebuild, reshape and recenter
preserve metadata and spans. The filterable attribute budget is ten.

Native eligibility precedes the binary 192-candidate cap, replacing the
store-wide direct-eligibility scan and its all-or-nothing dense fallback.
Source-dependency/barrier fallback remains. A relaxed node window uses
separate native node/edge candidate arms with a global cap to preserve the
existing unconditional edge-expiry rule. Public historical view admission and
membership remain Slice 80. Pre-extension format-35 layouts are refused;
there is no migration. The [Slice 70 design](../plans/0.8.28/features/slice-70/design.md)
records the Astra-reviewed implementation and boundary tests.
