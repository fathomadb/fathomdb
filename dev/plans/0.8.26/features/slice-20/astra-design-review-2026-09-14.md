---
title: FathomDB 0.8.26 Slice 20 — GPT-6 Astra medium design review
status: REQUEST CHANGES
reviewed_on: 2026-09-14
reviewed_release_tip: 85afa2cf897ac575f584ac1674080815aaea246b
reviewed_implementation_tip: e1449568ea066c5db9fbe5d286c3e067e98cf898
---

# Slice 20 GPT-6 Astra medium design review

## Verdict

`REQUEST CHANGES`. D26-01 option A is translated faithfully and the design
respects the no-parallel-V1/V2 decision. Implementation remains paused until the
findings below are reconciled and independently re-reviewed.

The paused implementation was inspected only as code-grounded evidence of
integration risks. This is not an implementation-review verdict.

## Findings

### P1 — authorization-first hydration and error precedence

The design promises detailed provenance errors and nondisclosing canonical-source
refusal, but the proposed joined hydration cannot distinguish all required states.
The Slice 15 prototype inner-joins source links, source rows, version rows, and
self-links; a missing row erases the classification needed to distinguish missing
source from incomplete artifact provenance. It also parses locator material before
source authorization, so an unauthorized source combined with corrupt locator data
could expose detailed corruption instead of the nondisclosing `/evidence` result.

Required correction: specify two logical phases within the same transaction and
two-statement bound. Preserve requested ordinals with nullable authority/provenance
joins; classify and authorize every required source first; only then validate
detailed provenance, hash, and locator material. Define global precedence for
mixed faults. Add denied-source-plus-malformed-locator and missing-source-versus-
missing-link tests. No row-level corruption detail may escape first.

### P2 — feasible source eligibility inside the two-statement bound

The design adds source status, creation-time, and attribute eligibility while
requiring exactly two hydration statements. Existing eligibility helpers issue a
metadata query and one query per attribute. Calling them per row would violate the
bound and create N+1 work.

Required correction: explicitly compile artifact and source metadata/attribute
predicates into the two joined statements using existing SQL eligibility grammar.
Remove source kind/source-type predicates where required, define missing-vector-
metadata and absent-versus-empty attribute behavior, and count every executed SQL
statement, including helper queries. Test nonempty metadata/attribute filters and
multiple sources.

### P2 — erasure linearization, not caller-observation ordering

The primary mutex proves a transaction ordering point, not that every SDK caller
observes resolver completion before erasure completes. Native response conversion
and scheduling occur after the mutex is released.

Required correction: define the linearization point as successful final frozen
validation and transaction commit while holding the primary mutex. If erasure wins
before that authorized read, resolution fails nondisclosingly. If resolution wins,
already copied bytes are not retrospectively revoked. Tests assert transaction
ordering, not cross-SDK delivery order.

### P2 — preserve the existing graph target write cursor

The design's broad statement that internal cursors never appear publicly conflicts
with the existing `GraphTargetV1.writeCursor`, whose byte path Option A preserves.

Required correction: prohibit cursors only in the new evidence sidecar and resolver
arguments. Explicitly retain the existing graph target cursor without granting it
evidence-resolution authority.

### P2 — freeze token layout and request normalization

The protected selector lacks an exact fixed layout, option tags, stream-extension
rule, and semantic request-normalization grammar. The existing ranked helper masks
only 64 bytes, while the proposed graph selector is larger. Existing graph request
encoding also preserves kind-list order and the literal frozen token, so semantic
normalization cannot be assumed.

Required correction: specify the bounded versioned selector byte layout; nonce;
unambiguous option tags; counter-based length-preserving stream blocks; distinct
MAC, stream, and commitment domains; authenticated framing; and maximum length.
Define whether kind-set reordering and equivalent newly minted frozen contexts
normalize together while preserving explicit seed order. Test payloads beyond 64
bytes, cross-domain rejection, fixed-nonce normalization, and round trips.

### P2 — out-of-window truth and lifecycle reporting

The plan categorically refuses out-of-window evidence while the design inherits
`include_out_of_window`. Existing ranked evidence can treat relaxed visibility as
actual edge validity and report `validAtEffective=true`, which conflates permission
with temporal truth.

Required correction: define target, terminal-edge, and canonical-source window
truth tables. Either reject relaxed-window graph evidence explicitly or honor it
while reporting actual validity at the effective instant. Distinguish graph
traversal edge-recency selection from evidence-byte eligibility. Test future-start,
ended-window, equality boundaries, and relaxed-window contexts.

## Confirmed design direction

The reviewer found the following direction otherwise appropriate:

- opt-in V1 sidecar under frozen authority;
- intrinsic graph-evidence resolver rather than ranked-field synthesis;
- required target and winning terminal-edge identities/references;
- optional direct dependency;
- post-selection hydration in the graph reader transaction;
- point resolution on the primary connection;
- no raw-ID/logical-ID lookup, cache, schema migration, full-path proof, or V2;
- deterministic winner ordering, including parallel-edge logical-ID/cursor
  tie-breaking, which the final ADR must state explicitly.

## Re-entry condition

Before implementation resumes, update the Slice 20 plan/design to resolve all six
findings, obtain another GPT-6 Astra medium design verdict, and require `PASS` with
no P0/P1/P2 findings. Then audit the paused implementation against that corrected
design before continuing its additive TDD chain.
