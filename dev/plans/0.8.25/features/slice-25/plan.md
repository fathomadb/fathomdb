---
title: 0.8.25 Slice 25 — atomic semantic actuation
status: AWAITING_HITL_DESIGN_FIX_CAP
depends_on: 20
design: design.md
design_status: BLOCKED_AFTER_REVIEW_CYCLE_5
---

# Slice 25 plan

## Outcome and boundary

Implement the retained core of R25/AC25-25 and Memex needs 3, 17, and the
compact portion of 18 under the approved
[scope adjustment](../../scope-adjustment-2026-09-02.md). Add one bounded,
typed, model-free, idempotent actuation batch for caller-decided canonical and
derived node writes, Slice 20 source dependency registrations, and existing
  governed lifecycle transitions. Return one compact terminal receipt for a committed or
whole-batch-refused request.

FathomDB validates and atomically applies a decision; it does not extract,
merge, infer conflict, choose truth, call a provider, or recover erased bytes.
Facts, edges, multi-source dependencies, merge/coexist verdicts, recursive
closure, exhaustive consequence receipts, and a durable prepared-request
journal remain outside Slice 25.

## Requirements and acceptance

- **S25-R1 — closed public contract.** Rust, Python, and TypeScript expose the
  same four operation variants, request fields, receipt fields, bounds, and
  closed error/refusal vocabularies.
- **S25-AC1.** Shared fixtures prove schema-version and unknown-field/
  unknown-variant precedence, field paths, operation-order preservation,
  canonical digest bytes, and cross-SDK codec parity. Windows builds the same
  contract.
- **S25-R2 — one atomic transaction.** A new request is validated and applied
  in one SQLite `BEGIN IMMEDIATE` transaction using the landed write,
  dependency, lifecycle, and projection substrates.
- **S25-AC2.** Fault injection at every operation position and before commit
  leaves no partial canonical, provenance, dependency, lifecycle, projection,
  cursor, or receipt state. Successful batches persist all of those together.
- **S25-R3 — ordered prospective state.** Operations execute in array order.
  A reference may name persisted state or a node created earlier in the same
  batch; forward references fail closed.
- **S25-AC3.** Tests cover mixed ordering, same-logical-ID supersession,
  dependency registration against an earlier put, revision-pinned lifecycle,
  forward references, duplicate operations, and whole-batch rollback.
- **S25-R4 — terminal idempotency.** The Engine hashes a normalized request.
  Replaying the same operation ID and digest returns the byte-equivalent stored
  terminal receipt; a different digest is a typed conflict.
- **S25-AC4.** Real-database restart and same-ID races prove one committed or
  refused terminal result. Pre-commit interruption leaves no receipt and a
  retry may execute; post-commit retry replays without domain work.
- **S25-R5 — compact lifecycle-safe receipts.** Receipts identify operation,
  digest, outcome, first deterministic refusal, affected revision IDs,
  resulting global boundary, dependency generation, pending projection
  cursors, and reserved closure references without storing bodies or locators.
- **S25-AC5.** Receipt rows validate lazily on keyed replay, remain bounded by
  the 128-operation request limit, and source erasure redacts content-derived
  replay material while preserving an opaque non-reusable operation tombstone.
- **S25-R6 — dependency-safe staging.** Slice 25 does not invent Slice 30
  closure state. A write or transition that would make a source revision
  non-current/non-live while it has registered dependents is terminally
  refused with `dependency_closure_required`.
- **S25-AC6.** Source supersession/deletion with and without dependencies is
  tested. The refused case changes no domain/cursor/projection state and
  replays identically; Slice 30 is the only owner that may later admit closure.
- **S25-R7 — compatibility.** Existing `write`, dependency, lifecycle,
  erasure, projection, and read behavior remain unchanged. Actuation is an
  additive governed Engine method and uses no recovery verb.
- **S25-AC7.** Existing public-surface, no-recovery, lifecycle, provenance,
  dependency, erasure, and projection suites remain green; interface docs and
  a successor ADR account for the additive method.

## Delivery sequence

1. Reconcile this plan/design with the landed Slice 15/20 APIs and record the
   additive actuation decision in a successor ADR.
2. Obtain independent design review. Resolve at most five FIX cycles; no open
   P1/P2 or implementation-shaping decision may enter RED.
3. Write and preserve RED tests before production changes. Each implementation
   review correction receives its own RED witness before GREEN. Resolve at
   most five implementation FIX cycles.
4. Obtain independent verification at the exact reviewed commit. Resolve at
   most two verifier FIX cycles.
5. Write `status.md`, update the release-state JSON/generated board, and commit
   Slice 25 closure.
6. Compact reusable Slice 25 lessons into the external memory store, verify
   the new file and index entry, then and only then commission Slice 30.

## Verification routes

Selected: focused schema/Engine/property tests; public contract and canonical
fixtures in Rust/Python/TypeScript; lifecycle/dependency/erasure/projection
regressions; facade allowlist; migration policy; packaged Python/npm smokes;
Windows Rust/Python/Node compile and fixture routes; and
`agent-verify.sh --tier=fast`. Heavy or all-feature routes run when focused or
fast evidence indicates cross-cutting risk. Operator is selected for existing
erasure/purge paths touched by the receipt privacy work. CUDA, live-model, and
pre-publication registry routes are N/A.

Stop on partial commit, FathomDB semantic judgment, forward-reference
acceptance, unbounded replay state, erased content retained in receipts,
ambiguous refusal precedence, or a dependency lifecycle gap delegated
implicitly to unfinished Slice 30 behavior.
