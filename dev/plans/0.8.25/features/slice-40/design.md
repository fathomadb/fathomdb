---
title: 0.8.25 Slice 40 — projection generation and readiness design
status: REVIEWED_BLOCKED_ON_SLICE_7
design_version: 2
target_release: 0.8.25
depends_on: 35
readiness_blocked_on: Slice 7 architecture activation
architecture: dev/design/fathomdb-data-plane-architecture-v2.md
---

# Slice 40 — projection generation and readiness design

## Authority and comparison

Owns R25/AC25-40, Memex need 13, and A25-05. It makes projection generation
and mutation-to-ready correlation durable across restart.

| Need | Existing behavior | Decision |
| --- | --- | --- |
| Identity | Declarations/cursors, no generation ID | Immutable UUIDv7 generations; one serving role per projection. |
| Truthful readiness | Runtime-derived dense state and terminal rows | Readiness is generation + fixed source boundary + complete applicability manifest. |
| Mutation correlation | Cursor only | One durable applicable or skipped work row per mutation/kind. |
| Rebuild/restart | Rebuild/pending recovery | New generation; durable resumable work; no identity reuse. |

Locked `projections.md` is preserved. Completed 0.8.22 status/runtime designs
are reused; runtime availability remains independent. This is their 0.8.25
generation successor.

## Public and wire contract

```text
ProjectionGenerationId(string)
ProjectionGenerationRole = building | serving | serving_legacy | retired | failed
ProjectionGenerationReadiness = pending | ready | degraded | blocked | deferred
ProjectionWorkState = pending | ready | skipped | degraded | blocked | deferred
ProjectionApplicabilityManifestV1 { schema_version: 1, projection,
  declaration_sha256, fixed_source_boundary, work_kinds,
  applicable_artifact_kinds, applicability_rule_version }
ProjectionGenerationStatusV1 { schema_version: 1, projection, generation_id,
  role, readiness, source_boundary, ready_through, work_counts,
  runtime_status?, created_at, activated_at? }
MutationProjectionWorkV1 { schema_version: 1, projection, generation_id,
  mutation_boundary, work_kind, state, reason_code?, retryable, work_id }
ProjectionBindingMapV1 { schema_version: 1, legacy_binding_sha256,
                         generation_id, equivalence_proof_sha256 }
```

`read_projection_generations(name?)` is pure. Existing status surfaces derive a
compatible coarse state but never translate non-ready to ready. Mutation
receipts carry sorted work entries. Every object inherits Slice 15 wire/SDK/
unknown/u64/error/Windows/registry rules and canonical fixtures.

## Persistence, applicability, and advancement

Add `_fathomdb_projection_generations`, `_fathomdb_projection_work`, and
`_fathomdb_projection_binding_maps`. A partial unique index permits exactly one
`serving` or `serving_legacy` generation per projection. Work is unique by
`(generation, mutation_boundary, work_kind)`.

The immutable applicability manifest fixes source boundary, work kinds, and
artifact-kind rules when the generation is created. For every committed
mutation boundary through that source boundary, each manifest work kind gets a
row: applicable work starts `pending`; deterministic non-applicability is
`skipped` with a reason. No implicit absence carries meaning.

`ready_through = B` only when every earlier boundary has every manifest work
kind in `ready` or `skipped`. The generation is `ready` only when
`ready_through == fixed_source_boundary` and no applicable row is pending,
degraded, blocked, or deferred. Runtime availability is reported separately.

Synchronous work commits ready/skipped rows with canonical mutation.
Asynchronous rows commit pending with it and transition atomically later.
Exhausted failure is degraded; unavailable approved runtime is blocked;
explicit delay is deferred.

## Migration, activation, and Slice 35 mapping

Legacy declarations migrate to one generation. Proven-equivalent, contiguous
legacy state becomes `serving/ready`. Ambiguous state becomes
`serving_legacy/degraded`: it may preserve compatibility reads, but cannot
satisfy generation-readiness, frozen-generation, or activation claims.

A new `building` generation activates only when its readiness is `ready` under
the exact predicate above. Activation atomically retires the old serving role
and makes the build serving. No blocked/deferred/degraded/pending generation or
ready-prefix gap can activate. Failed builds never serve.

For each unexpired Slice 35 legacy binding, migration stores a binding map only
after exact declaration/boundary/cursor/terminal/row equivalence proof. It
never changes the lease. Zero/multiple equivalents produce the Slice 35 typed
failure rather than rebinding.

## Lifecycle, erasure, and invariants

- Generation identity is never reused after failure, retirement, repair,
  erasure, or restart.
- Snapshot/cursor/evidence contracts bind immutable generation IDs.
- Application code cannot clean rows, change manifests, or advance readiness.
- Erasure scrubs source-bearing projection rows and work payloads for **serving,
  serving_legacy, building, retired, and failed** generations before completion.
  It invalidates affected leases immediately. Ordinary retired reclamation may
  wait for leases; erasure may not.
- Failures are `UnknownProjectionGeneration`, `WrongProjectionGeneration`,
  `ProjectionBlocked`, `ProjectionDeferred`, `ProjectionDegraded`, and
  `ProjectionGenerationUnavailable`.

## Tests and verification

Properties cover applicability/skips, multiple work kinds, prefix gaps, exact
activation, legacy ready/ambiguous migration, Slice 35 mapping, and codecs.
Crash injection covers canonical+work, transition, rebuild, activation, and
restart without duplication/reuse. Dense tests cover missing/refused runtime,
CUDA success/failure/retry. Erasure canaries inspect all five generation roles
and work tables. Existing drain/status/compact-search, cross-SDK golden,
Windows CPU/native, operator, CUDA, and registry routes pass.

Routes: fast/heavy/all/all-feature/operator, Windows Rust/Python/Node, CUDA
preflight/rehearsal, registry status. Live-model only if deterministic dense
fixtures cannot prove behavior. Status remains `REVIEWED_BLOCKED_ON_SLICE_7`, blocked on Slice
7/35; no implementation-shaping decision remains.
