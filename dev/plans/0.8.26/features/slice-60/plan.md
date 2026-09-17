---
title: FathomDB 0.8.26 Slice 60 — current design-owner reconciliation
status: DRAFT
target_release: 0.8.26
---

# Slice 60 plan — retrieval, recovery, and engine owners

## Outcome

Make the maintained retrieval, recovery, and engine designs trustworthy
current-state owners. The expected implementation is documentation-only: the
accepted ADRs, public interfaces, schema, implementation, and tests determine
truth. An actual code contradiction is reported and separately scoped rather
than repaired inside this slice.

## Scope

In scope:

- build one shared authority-and-witness inventory for all three owners;
- rewrite `dev/design/retrieval.md` around the live lexical, vector, graph,
  fusion, cross-encoder reranking, device, validity/frozen-view, evidence, and
  explanation pipeline;
- rewrite `dev/design/recovery.md` with an explicit ownership boundary and an
  exact current CLI diagnostic inventory, while preserving the distinction
  between immutable inspection and recovery mutation;
- rewrite `dev/design/engine.md` as the current schema-34 profile, including
  `logical_id`-alone active identity, caller and projection write lanes,
  `commit_gate` serialization, and the shared vector-table model;
- remove or relocate obsolete 0.6.0 claims, including nonexistent current API
  claims, without deleting unique historical rationale; and
- cross-read all three final owners and update an earlier rewrite when a later
  rewrite changes shared terminology, boundaries, or references.

Out of scope:

- engine, schema, migration, binding, or CLI implementation changes;
- new public APIs, ADR decisions, or compatibility shims;
- changes to accepted identity, concurrency, or vector architecture; and
- generalized lifecycle checker work, which belongs to Slice 65.

## Requirements and acceptance

| Requirement | Acceptance criterion |
| --- | --- |
| R26-60A: shared grounded inventory | AC26-60A: every current claim in the three owners is traceable to an accepted ADR/interface plus an implementation or test witness, or is explicitly labeled historical/deferred. |
| R26-60B: retrieval truth | AC26-60B: `retrieval.md` describes all live retrieval arms, fusion/reranking, device behavior, view/eligibility interactions, bounds, explanations, and evidence without presenting deferred behavior as shipped. |
| R26-60C: recovery truth | AC26-60C: `recovery.md` states its ownership boundary, matches the live CLI diagnostic inventory, preserves immutable inspection versus mutation, and does not imply an SDK `doctor` or recovery verb. |
| R26-60D: engine truth | AC26-60D: `engine.md` agrees with the accepted `logical_id`-alone identity, current writer topology and `commit_gate`, schema-34 fresh-open boundary, and shared vector layout. |
| R26-60E: no hidden product rewrite | AC26-60E: the product-code diff is empty. Any implementation contradiction is recorded with upstream/downstream impact and stops completion pending explicit scope. |
| R26-60F: feedback reconciliation | AC26-60F: after the engine rewrite, retrieval and recovery are reread against it and amended where shared facts changed; any affected Slice 55 mapping is updated only through its RED/GREEN parity contract. |
| R26-60G: verification | AC26-60G: design lifecycle/reference/authority checks, Markdown/link/docs checks, focused source comparisons, and `agent-verify` pass. |

## Execution sequence

1. Build the code-grounded authority matrix once for all three topics. Record
   current behavior versus historical or net-new behavior before drafting.
2. Rewrite retrieval and recovery against that matrix and the Slice 55
   canonical SDK inventory.
3. Rewrite engine last, using accepted ADRs, architecture v2.2, schema, engine
   code, and tests as upstream truth—not the old prose.
4. Reread and amend retrieval, recovery, bindings references, and the authority
   matrix for any boundary or terminology changed by the engine rewrite.
5. Verify the product-code diff is empty and run the proportional repository
   gates. Record the disposition and evidence in `status.md`.

## Stop gates

Stop on an actual code-versus-accepted-authority conflict, an unresolved
current owner, a required public-contract/ADR change, or loss of unique
historical rationale. Do not silently turn the finding into product surgery.
