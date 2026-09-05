---
title: Recovery design — 0.8.25 projection-generation successor
date: 2026-09-05
target_release: 0.8.25
supersedes: recovery.md only for projection-generation diagnosis
status: COMPLETE
---

# Recovery design — 0.8.25 successor

This additive successor preserves the locked 0.6.0 recovery surface while
documenting the Slice-40 projection-generation failure contract. It does not
add an automatic repair verb or make application code responsible for cleanup.

## Projection generation

Code: `E_CORRUPT_PROJECTION_GENERATION`

An open fails at `ProjectionGeneration` with corruption kind
`ProjectionGenerationDrift` when the retained generation history, current
singleton, declaration digest, receipt correlation, or physical completion
authority is malformed or contradictory. A live status read reports the same
condition as `FDB_PROJECTION_GENERATION` with reason
`projection_generation_corrupt` and field path `/projectionGeneration`.

Do not edit generation tables, projection terminals, vector sidecars, or vec0
rows directly. Preserve the database and diagnostic output. If canonical rows
remain trustworthy, the governed recovery is an operator-authorized
`fathomdb recover --accept-data-loss --rebuild-projections`; it mints a new
generation and rebuilds the in-place physical projections. If generation
authority itself cannot be validated sufficiently to enter that workflow,
safe-export the canonical data and rebuild a new database rather than guessing
an identity or laundering a legacy generation.

A `legacy_unverified` generation with `degraded` readiness is not corruption.
It records an upgraded nonempty pre-Slice-40 database whose earlier physical
state cannot be certified. An explicit governed configuration change or
operator rebuild may establish a new certifiable epoch; an idempotent replay
must not.
