---
title: FathomDB 0.8.26 Slice 20 — Astra design finding reconciliation
status: READY FOR GPT-6 ASTRA MEDIUM RE-REVIEW
reconciled_on: 2026-09-14
source_review: astra-design-review-2026-09-14.md
---

# Slice 20 Astra design finding reconciliation

Implementation remains paused. This record maps every finding from the first
GPT-6 Astra medium review to the corrected plan/design and its required oracle.

## Reconciled findings

1. **Authorization-first hydration and global precedence.** The design now
   requires ordinal-preserving nullable joins and two logical phases across both
   query result sets. Authority classification completes before locator/hash
   parsing or positional detail. Missing linked source is nondisclosing;
   missing source-link is incomplete provenance only after global authority
   passes. Mixed denied-source/corrupt-locator and missing-source/link fixtures
   pin the ordering.
2. **Eligibility within two statements.** Artifact and canonical-source
   metadata, lifecycle, and attribute predicates are compiled into the two
   hydration statements. No query-emitting helper is permitted. Missing vector
   metadata and absent-versus-present-empty attribute semantics are explicit;
   filtered multi-source tests count every executed statement.
3. **Erasure linearization.** The design now names successful final frozen
   validation and transaction commit under the primary mutex as the resolver
   linearization point. It does not claim SDK observation order or retroactive
   revocation of copied bytes. Rendezvous tests assert database transaction
   order only.
4. **Existing cursor compatibility.** The ordinary
   `GraphTargetV1.writeCursor` remains public and byte-compatible but grants no
   evidence authority. Only the new evidence sidecar and resolver request are
   cursor-free.
5. **Fixed token and normalization grammar.** The design fixes a 300-byte
   selector, exact 704-character token, tags, nonce, counter-based ten-block
   stream, authenticated framing, literal separated domains, and canonical
   request encoding. Kind lists normalize as sets; query text and explicit seed
   order do not; equivalent semantic frozen contexts exclude their literal
   token from the request digest. Beyond-64-byte, cross-domain, and fixed-nonce
   tests are mandatory.
6. **Temporal truth.** Evidence rejects authenticated
   `includeOutOfWindow=true` contexts at an exact path. Target, terminal edge,
   and canonical source use start-inclusive/end-exclusive actual validity.
   Traversal recency remains a separate graph-selection fact; lifecycle output
   cannot convert relaxed visibility to temporal truth.

The design also pins the existing deterministic winning-edge order, including
parallel-edge logical-ID and cursor tie-breaking, for the final ADR and test.

## Re-entry gate

A fresh GPT-6 Astra medium reviewer must inspect the revised plan and design,
the original review, this reconciliation, and the current implementation only as
code-grounded feasibility evidence. Implementation may resume only on `PASS`
with no P0/P1/P2 finding. A new finding that changes the authorized public shape
or crosses a stop gate returns to HITL.
