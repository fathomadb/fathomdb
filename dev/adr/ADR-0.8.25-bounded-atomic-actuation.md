---
title: ADR-0.8.25-bounded-atomic-actuation
date: 2026-09-04
target_release: 0.8.25
desc: Add one bounded model-free Engine actuation method while preserving PreparedWrite and existing verbs
blast_radius: Engine public API; Rust/Python/TypeScript bindings; schema step 29; lifecycle and dependency transaction composition; interface docs
status: accepted by approved 0.8.25 scope and Slice 25 execution authorization
supersedes_in_part: ADR-0.6.0-prepared-write-shape one-entry-point composition statement
---

# ADR-0.8.25 — Bounded atomic actuation

## Decision

Add `Engine::actuate(ActuationBatchV1) -> ActuationReceiptV1` as one governed,
typed, bounded method for atomically applying caller-decided canonical/derived
node writes, source dependency registrations, and existing lifecycle
transitions.

Preserve `Engine::write(&[PreparedWrite])` and every existing specialized
method. `PreparedWrite` remains the canonical typed representation for ordinary
record writes and no raw SQL is exposed. This ADR supersedes only the older
claim that every mixed composition must enter through `Engine::write`; it does
not supersede the typed boundary, per-entity newtypes, or `PreparedWrite` shape.

The actuation method exists because idempotent operation identity, ordered
cross-domain prospective validation, whole-batch refusal, and a terminal
receipt are not representable as another record-shaped `PreparedWrite` without
mixing command and entity semantics.

## Constraints

- The caller makes all semantic decisions; FathomDB validates mechanism and
  lifecycle invariants only.
- Requests contain 1–128 closed operations and execute in array order in one
  SQLite transaction.
- Existing write/dependency/lifecycle methods and behavior remain available.
- Slice 25 stores terminal compact receipts only; no prepared-request or
  in-progress public journal is introduced.
- Until Slice 30 owns barriers and closure, an operation that would make a
  source with registered dependents non-current/non-live is refused.
- Rust, Python, TypeScript, wire, Windows, erasure, and recovery-denylist proofs
  land with the method.

## Alternatives

1. **Add actuation variants to `PreparedWrite`.** Rejected: operation identity,
   refusal, lifecycle commands, and terminal receipts are batch-command
   semantics, not stored entity variants; nesting a batch inside the existing
   batch would make atomicity and error ownership ambiguous.
2. **Require callers to sequence existing methods.** Rejected: a caller cannot
   obtain cross-method atomicity and would need a shadow recovery protocol.
3. **Replace existing methods with actuation.** Rejected: unnecessary public
   breakage and contrary to the additive governed-surface policy.
4. **Implement a durable prepared-request journal.** Rejected for 0.8.25: the
   retained scope needs atomic commit and terminal replay, which SQLite plus a
   compact receipt supplies without persisting source-bearing request bodies.

## Consequences

The Engine gains one public method and one terminal receipt table plus a
source-reference index for erasure. Internally, existing operations need
side-effect-free validation and transaction-scoped apply seams so one owner can
compose them without nested transactions or early cursor publication. Slice
25's design is the exact contract.

This is an additive pre-1.0 surface change. It remains governed by the accepted
open-but-curated SDK policy and requires interface documentation, allowlist
updates, binding parity, and no-recovery proof in the implementation commit.

## Authority

The user approved the 0.8.25 scope retaining core model-free atomic actuation,
approved the Slice 7/release plan, and authorized Slice 25 implementation after
Slice 20. This ADR records that already-approved product decision; it does not
introduce a new semantic-policy capability.
