---
title: Memex collaboration Slice 20 — exact graph-target evidence
status: DRAFT
depends_on: 10
design: design.md
design_status: DRAFT
date: 2026-09-12
---

# Slice 20 plan

## Outcome and boundary

Add a FathomDB-owned mechanism that lets a consumer resolve exact canonical
evidence for a graph target under the same frozen eligibility envelope. The
mechanism must use immutable artifact-revision identity and must not depend on
body search, top-K inclusion, raw SQLite, or a Memex-maintained shadow index.

This slice resolves target-content evidence. It does not claim that target
evidence proves the traversal path, and it does not add full ordered path
evidence. Relationship evidence is explicit and limited to the terminal edge
identified by the graph result.

## Delivery sequence

1. Reconcile [`design.md`](design.md) with the accepted compact-evidence ADR,
   constrained graph contract, lifecycle/erasure rules, and all Rust, Python,
   TypeScript, and wire interfaces.
2. Decide the public evolution shape: additive optional revision identity on
   the open response carrier versus a successor graph result. Reject any shape
   that emits a fabricated revision for legacy/incomplete provenance.
3. Add a successor ADR and interface changes for immutable-revision lookup,
   non-disclosure precedence, terminal-edge evidence, and the distinction
   between target evidence and path evidence.
4. Write failing tests first for exact association, drift, eligibility,
   lifecycle, supersession, erasure, dependency closure, incomplete provenance,
   terminal-edge identity, restart, and cross-database refusal.
5. Implement one reader-transaction lookup using indexed immutable identity;
   then add Rust, Python, TypeScript, and wire parity.
6. Verify the installed wheel and packed npm artifact with a graph-expand to
   evidence-resolution consumer spike that contains duplicate target bodies and
   excludes the target from ordinary search top-K.
7. Update public documentation with the final API, failure model, and examples.

## Acceptance criteria

- **S20-AC1:** Every evidence-capable graph target exposes its exact immutable
  artifact revision; absence is explicit for legacy/incomplete provenance.
- **S20-AC2:** Resolution by artifact revision returns exact canonical bytes,
  locator/span, source identities and hash, lifecycle, direct dependency, and
  applicable graph origin under an equivalent frozen context.
- **S20-AC3:** Missing, unauthorized, superseded, inactive, erased,
  closure-fenced, context-mismatched, or foreign targets collapse to the
  documented non-disclosing refusal.
- **S20-AC4:** Duplicate bodies and adverse top-K ranking cannot change which
  artifact is resolved.
- **S20-AC5:** Target-content evidence and terminal-edge evidence are separately
  addressable; neither is described as full path evidence.
- **S20-AC6:** Existing graph expansion and ordinary evidence search remain
  behaviorally unchanged when the new operation is unused.
- **S20-AC7:** Rust, Python, TypeScript, and wire codecs agree, including
  restart and foreign-database behavior.

## Verification

Run focused real-database tests, codec/property tests, privacy/non-disclosure
tests, concurrency and restart tests, binding conformance, fresh wheel/npm
spikes, `scripts/agent-verify.sh`, and the broader applicable package gates.

## Stop gates

Stop on logical-ID-only resolution, body re-search, top-K dependence, raw
database access, identity leakage through differentiated refusal, use of
`write_cursor` as the durable public identity, weakened frozen eligibility, or
a claim that terminal-edge evidence proves the whole path.
