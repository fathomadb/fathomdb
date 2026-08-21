# TRACE-01 — Projection lifecycle integrity

**Status:** complete canary; integrated at `ca5b656d` after independent review
and a passing synthetic lifecycle suite. It authorizes lifecycle-dependent
preparation, not a live benchmark.

## Decision

Do every derived retrieval unit and its measurements remain attributable to a
canonical source through write, supersession, erasure, and re-open?

## Preparation and contract

1. Freeze a projection inventory: text row, vector child, summary, extracted
   fact, entity, and edge, including canonical source and lifecycle fields.
2. Specify a safe `trace-projection.v1` sidecar that records only identifiers,
   counts, hashes, and lifecycle outcomes; reference it from the common receipt.
3. Write tests for source coverage, ambiguous-source rejection, supersession,
   stale-hit absence, source erasure, and re-open recovery.
4. Resolve the ELPS supersession representation: use one documented warning or
   result field shape, then test the extractor-to-sidecar mapping.

## Exit evidence

A fixed synthetic lifecycle fixture has zero unattributed derived rows, zero
stale searchable rows after supersession or erasure, and a complete safe sidecar.
No corpus or live extractor run is authorized by this plan.
