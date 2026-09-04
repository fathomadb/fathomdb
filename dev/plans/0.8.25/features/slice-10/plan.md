---
title: 0.8.25 Slice 10 — executable measurement classification
status: COMPLETE_ON_RELEASE_BRANCH
depends_on: 7
design: design.md
design_status: READY_REVIEW_PASS_CYCLE_3
---

# Slice 10 plan

## Outcome and carried obligations

Implement R25/AC25-10 and Memex need 24: receipts classify data-plane,
semantic-control-plane, and end-to-end metrics; state whether `Engine.search`
ran; identify shared and differing components; reject invalid mixed-layer
claims; and reclassify GLOBAL-01 without rewriting measured evidence.

## Verification routes

Selected: fast, heavy, and a native retrieval-only `Engine.search`
classification fixture. The final installed release-candidate witness belongs
to Slice 75. Windows CPU/native, all-feature/operator, GPU/CUDA, live-model,
packaged, and registry-installed are N/A unless the design introduces a public
executable or binding surface; the readiness review must confirm that judgment.

## Draft-to-ready and delivery

1. Reconcile the design with immutable GLOBAL-01 evidence and obtain an
   independent READY verdict.
2. Commit RED fixtures for schema closure, source-bound metrics, layer
   derivation, historical cutover enforcement, atomic/idempotent sidecars, and
   the native witness.
3. Implement the classifier, validator, historical sidecars, cutover policy,
   lint gate, and native fixture until those unchanged fixtures are GREEN.
4. Obtain independent implementation review, allowing at most three documented
   implementation FIX-n cycles. Design readiness has its own completed
   three-FIX ceiling.
5. Use a separate verifier to run the scoped workspace gates, inspect the
   generated historical/native evidence, and record any release-wide debt
   exposed by diagnostic gates without misattributing it to this slice.
6. Write `status.md` and advance the canonical release state only after every
   acceptance criterion is evidenced.

Stop on ambiguous metric ownership, unverifiable search-execution claims, or
any attempt to alter historical measurements or the append-only experiment
index.
