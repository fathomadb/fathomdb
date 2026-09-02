---
title: 0.8.25 Slice 40 — core projection generation and readiness design
status: DRAFT_SCOPE_RECONCILED_BLOCKED_ON_SLICE_7
design_version: 3
target_release: 0.8.25
depends_on: 35
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 40 design

## Authority and boundary

Implements the retained core of R25/AC25-40, Memex need 13, and A25-05:
durable projection-generation identity, truthful readiness, restart-safe
advancement, and compact mutation-to-ready correlation. Existing projection
declarations, cursors, runtime status, and rebuild behavior remain substrates.

A caller-visible work manifest or general projection scheduler is not part of
0.8.25 and is preserved in the post-0.8.25 notes.

## Contract

```text
ProjectionGenerationId(string)
ProjectionGenerationRole = building | serving | retired | failed
ProjectionReadiness = pending | ready | degraded | blocked | deferred
ProjectionGenerationStatusV1 {
  schema_version: 1, projection, generation_id, declaration_sha256,
  role, readiness, source_boundary, ready_through,
  pending_count, failure_code?, runtime_status?
}
MutationProjectionStatusV1 {
  schema_version: 1, mutation_boundary, projection,
  generation_id, state: pending | ready | skipped | degraded | blocked | deferred,
  work_id, reason_code?
}
```

`read_projection_generations(name?)` and
`read_mutation_projection_status(boundary)` are bounded, pure status reads.
Existing coarse status surfaces derive from the same durable state and never
translate a non-ready state into ready.

## Persistence and advancement

Persist immutable generation rows and one compact work-status row per
applicable `(generation, mutation_boundary, work_kind)`. Applicability is
determined internally from the versioned projection declaration; deterministic
non-applicability records `skipped` rather than relying on absence. At most one
generation serves a projection.

`ready_through=B` requires every applicable earlier boundary to be `ready` or
`skipped`. A generation is `ready` only when `ready_through` reaches its fixed
source boundary and no applicable row is pending, degraded, blocked, or
deferred. Runtime availability is reported separately.

Synchronous work records ready/skipped with the canonical mutation.
Asynchronous work records pending with it and advances atomically later.
Restart resumes from durable pending rows without duplicating work or reusing a
generation ID. Rebuild always creates a new building generation; activation
atomically retires the former serving generation only after exact readiness.

Legacy projection state migrates to one generation. Proven contiguous state
can serve; ambiguous state remains degraded until rebuilt. Slice 35 frozen
contexts bind generation IDs when available; an unavailable or replaced bound
generation fails typed and is never silently rebound.

## Lifecycle, compatibility, and verification

Erasure scrubs source-bearing rows in serving, building, retired, and failed
generations before completion. Applications cannot mutate generation/work
tables or advance readiness. Failures are `unknown_projection_generation`,
`wrong_projection_generation`, `projection_blocked`, `projection_deferred`,
`projection_degraded`, and `projection_generation_unavailable`.

All types follow Slice 15 wire/SDK rules. RED/GREEN properties cover
applicable/skipped work, prefix gaps, exact activation, ambiguous migration,
wrong-generation access, restart, duplicate prevention, and no identity reuse.
Fault tests cover canonical+work atomicity, async transition, rebuild,
activation, and erasure across every generation role. Dense fixtures cover
missing/refused CUDA runtime and successful retry without false readiness.

Run fast, heavy, all/all-feature/operator, Windows Rust/Python/Node, locally
packed status, and CUDA readiness routes. Live-model and pre-publication
registry routes are N/A. A formal independent READY review remains required
after Slice 7 and Slice 35 complete.
