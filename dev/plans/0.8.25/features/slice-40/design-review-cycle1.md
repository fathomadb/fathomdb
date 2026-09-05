---
title: 0.8.25 Slice 40 independent design review — cycle 1
status: CHANGES_REQUIRED
review_cycle: 1
reviewed_on: 2026-09-05
---

# Slice 40 independent design review — cycle 1

The formal code-grounded review found the v3 draft was not implementation-ready.
Its highest-level defect was mixing a global physical serving-generation
contract with a metadata layer over the current in-place stores.

| ID | Severity | Finding | Required correction |
|---|---|---|---|
| DR40-04 | P1 | Generation ownership/granularity was undefined while dense readiness is one shared physical pipeline. | Enumerate physical ownership and choose one generation unit. |
| DR40-05 | P1 | One immutable `source_boundary` could not describe a live generation. | Separate immutable start/build boundary from observed and ready-through boundaries. |
| DR40-06 | P1 | Mutation boundary conflated write cursor, batch result, dependency generation, lifecycle ID, and new work sequence. | Use one existing authority and define exact receipt-to-status flow. |
| DR40-07 | P1 | Side-by-side building/serving activation had no physical routing implementation. | Design a real parallel store or truthfully model in-place transition. |
| DR40-08 | P1 | Durable work rows/public `work_id` implied a new scheduler and Parked work manifest. | Preserve the current scheduler; expose compact aggregate correlation only. |
| DR40-09 | P1 | New readiness states had no safe mapping to accepted coarse status. | Define successor semantics and conservative compatibility mapping. |
| DR40-10 | P1 | Slice-35 token integration lacked an executable mapping/invalidation algorithm. | Keep token v1 unchanged; define generation state in its serving binding and exact drift behavior. |
| DR40-11 | P1 | Migration/minting proof, identity grammar, collision, clone, and corruption behavior were absent. | Specify additive shape, Engine bootstrap, minting, validation, and legacy truth policy. |
| DR40-12 | P1 | Concurrent write/rebuild/erasure/restart closure was unspecified. | Define fencing, publication revalidation, in-place transition, restart, and erasure behavior. |
| DR40-13 | P2 | Applicability/skipped rows could grow as boundary × kind × generation without a bound. | Use compact indexed correlation and state an amplification/retention policy. |
| DR40-14 | P2 | Persisted state machine and SQL constraints were missing. | Define legal transitions, singleton authority, checks, and corruption failures. |
| DR40-15 | P2 | Public API/wire shapes, limits, ordering, SDK names, and u64 encoding were incomplete. | Specify exact versioned methods, records, encoding, caps, and errors. |
| DR40-16 | P2 | FFI validation precedence was absent. | Define cross-SDK precedence and canonical negative fixtures. |
| DR40-17 | P2 | Runtime/CUDA state was conflated with generation identity. | Keep device/runtime outside identity; define blocked/deferred/recovery overlays. |
| DR40-18 | P2 | Declaration digest semantics were missing. | Pin canonical bytes, fields, order, domain, and identity-change rules. |
| DR40-19 | P2 | Feature-local performance/storage criteria were absent. | Preregister write, storage, status, reopen, and CPU/CUDA status bounds. |
| DR40-20 | P2 | The plan lacked numbered requirements, tests, commands, receipts, and platform/package routes. | Add executable traceability and exact verification paths. |
| DR40-21 | P3 | Draft status still said blocked on completed Slice 7. | Reconcile metadata after substantive correction. |
| DR40-22 | P3 | Predecessor design disposition was implicit. | State amendment/successor relationships without rewriting accepted history. |

The selected correction is the narrower metadata-correlation design over the
one current in-place physical serving set. It does not create a parallel index,
new scheduler, or public work-manifest administration surface.
