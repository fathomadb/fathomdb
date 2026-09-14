---
title: FathomDB 0.8.26 Slice 20 — GPT-6 Astra medium design re-review
status: PASS
reviewed_on: 2026-09-14
reviewed_release_tip: 57fb8ca61873b63c260b06a303e69950b5b75e04
reviewed_implementation_tip: e1449568ea066c5db9fbe5d286c3e067e98cf898
---

# Slice 20 GPT-6 Astra medium design re-review

## Verdict

`PASS` with no P0, P1, or P2 design findings. The reviewer read the plan,
design, original review, reconciliation, and pause record completely. Release
code and the paused implementation were inspected only as feasibility evidence;
this verdict does not certify implementation, tests, or release readiness.

## Original-finding disposition

1. **Authorization and error precedence — resolved.** Ordinal-preserving nullable
   joins and global authority classification precede provenance decoding across
   both artifact classes. Missing links and missing linked sources are distinct,
   and mixed-fault tests are required.
2. **Two-statement eligibility — resolved.** Artifact/source eligibility is
   compiled into the two hydration statements; query-emitting helpers are
   forbidden. Missing metadata and absent-versus-empty attributes are specified.
3. **Erasure linearization — resolved.** Final validation and commit under the
   primary mutex is the linearization point. Later SDK delivery of copied bytes
   is permitted and no retroactive revocation is claimed.
4. **Existing target cursor — resolved.** `GraphTargetV1.writeCursor` remains;
   the new sidecar and resolver arguments are cursor-free.
5. **Selector/framing/normalization — resolved.** The arithmetic is consistent:
   44 scalar bytes plus eight 32-byte commitments is a 300-byte selector;
   nonce plus selector plus MAC is 348 bytes, encoded as 696 hex characters
   after the eight-character prefix for a 704-character token. Ten SHA-256
   stream blocks cover the selector. Domains and normalization are explicit.
6. **Temporal truth — resolved.** Relaxed-window evidence is rejected and actual
   start-inclusive/end-exclusive validity is independently defined for target,
   winning edge, and canonical source.

The reviewer also confirmed that the deterministic parallel-edge winner rule
matches the existing traversal, the public carrier/error shapes are implementable
across Rust/Python/TypeScript without schema work, and the planned tests are
proportionate.

## Implementation re-entry

The design-review gate is satisfied. Before coding continues, audit paused tip
`e1449568ea066c5db9fbe5d286c3e067e98cf898` against the corrected design and
supersede its known nonconforming mechanisms through the additive TDD chain. In
particular, replace inner-join classification, per-row eligibility queries,
ranked-style short selector masking, relaxed-window behavior, and delivery-order
erasure claims rather than treating the paused prototype as approved.
