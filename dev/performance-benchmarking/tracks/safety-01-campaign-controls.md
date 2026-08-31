# SAFETY-01 — Campaign controls and receipt integrity

**Status:** complete; reuse for every run.

## Decision

Can a campaign produce reproducible, content-safe evidence without storing
corpus-derived payloads in Git?

## Draft plan

1. Before a run, validate its configuration, external output root, safe receipt,
   and append-only index row.
2. If a receipt schema changes, run one content-free fixture first.
3. Accept only when the receipt validates and generated views rebuild.

## Stop

Stop before execution on invalid provenance, unsafe output, or receipt failure.
This plan makes no quality or performance claim.
