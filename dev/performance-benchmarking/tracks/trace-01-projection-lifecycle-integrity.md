# TRACE-01 — Projection lifecycle integrity

**Status:** complete canary; rerun only when projection lifecycle behavior changes.

## Decision

Do every derived retrieval unit and its measurements remain attributable to a
canonical source through write, supersession, erasure, and re-open?

## Draft plan

1. Use the fixed synthetic fixture and current projection inventory.
2. Exercise write, supersession, erasure, and reopen once per projection type.
3. Accept only with zero unattributed rows and zero stale searchable rows.

## Stop

Stop on ambiguous source identity or a stale hit. No corpus run is needed.
