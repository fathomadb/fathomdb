# SAFETY-01 — Campaign controls and receipt integrity

**Status:** complete infrastructure; retain and re-check before every new track.

## Decision

Can a campaign produce reproducible, content-safe evidence without storing
corpus-derived payloads in Git?

## Preparation and contract

1. Verify the typed `experiments.record.v1` and `experiments.index-row.v1`
   writers reject unknown or missing fields and keep the index append-only.
2. Name the campaign configuration, external artifact root, and generated-view
   refresh procedure; raw artifacts must remain outside the repository.
3. Add a dry-run receipt fixture whenever a runner adds a new sidecar schema.

## Exit evidence

The run writes a valid receipt, resolved configuration, metrics projection, and
one index row; `INDEX.md` and `SCOREBOARD.md` regenerate from those inputs.
This track authorizes no quality or performance claim.
