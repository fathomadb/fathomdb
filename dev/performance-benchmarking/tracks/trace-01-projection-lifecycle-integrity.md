# TRACE-01 — Projection lifecycle integrity

**Status:** complete canary; rerun only when projection lifecycle behavior changes.

## Decision

Do every derived retrieval unit and its measurements remain attributable to a
canonical source through write, supersession, erasure, and re-open?

## Draft plan

1. Use the fixed synthetic fixture and enumerate every materialized projection:
   vector children/embeddings, summaries, extracted facts/events, and graph
   entities, claims, and edges. Record absent types as not applicable.
2. For each type, verify projection-registry membership and canonical source
   identity through write, supersession, source erasure, and reopen.
3. Accept only with complete source attribution, zero orphan projection rows,
   and zero stale searchable rows.

## Stop

Stop on ambiguous source identity or a stale hit. No corpus run is needed.
