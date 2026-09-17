---
title: FathomDB 0.8.26 Slice 65 — lifecycle recurrence guards and requalification
status: DRAFT
target_release: 0.8.26
---

# Slice 65 plan — semantic ownership gate and candidate requalification

## Outcome

Turn the corrected current-owner model into enforceable lifecycle structure,
repair the error-ownership edges exposed by the adversarial review, and bind a
new exact non-publishing candidate after all post-Slice-50 changes. This is the
only broad verification slice in the reopened ladder.

## Scope

In scope:

- add machine-readable current-profile, semantic-authority, implementation/test
  witness, and evidence-only relationships for maintained design owners;
- reject missing targets, ambiguous ownership, maintained current semantics
  delegated to historical/proposal/deferred documents, and falsely current
  profiles, while permitting explicit evidence-only references;
- reconcile `dev/design/errors.md` and affected maintained topic owners so
  stable error semantics terminate at maintained owners or accepted contracts,
  not release-local or historical slice records;
- add focused RED fixtures for each new lifecycle failure and GREEN the smallest
  checker/catalog/document changes without claiming to prove prose semantics;
- rerun the Slice 55 parity mutation suite and the Slice 60 owner-authority
  matrix; amend their artifacts when the new gate exposes a genuine gap; and
- replace the now-stale Slice 50 candidate/completion claim with exact candidate
  evidence for the final Slice 65 commit.

Out of scope:

- inferring semantic truth from keywords or freezing prose wording in brittle
  string-match tests;
- weakening lifecycle classifications to make the checker green;
- runtime product changes unless separately authorized after a stop condition;
- publication, tagging, registry mutation, or main integration.

## Requirements and acceptance

| Requirement | Acceptance criterion |
| --- | --- |
| R26-65A: explicit current authority | AC26-65A: every maintained catalog entry names a current profile, existing semantic authority, and implementation/test or bounded enforcement witness; metadata cannot elevate a design above ADR/interface authority. |
| R26-65B: valid semantic graph | AC26-65B: current semantic-owner edges are existing, unambiguous, acyclic, and terminate at maintained designs or accepted external authorities; historical/proposal/deferred targets are legal only as explicit evidence, never current authority. |
| R26-65C: error ownership | AC26-65C: current error semantics formerly delegated to historical or release-local slice documents are incorporated into maintained owners or accepted contracts, with the old documents retained as evidence. |
| R26-65D: non-vacuous recurrence tests | AC26-65D: committed or staged RED fixtures prove each invalid edge/profile/witness case fails, evidence-only references pass, and existing structural lifecycle controls remain intact. |
| R26-65E: honest gate claim | AC26-65E: documentation states that automation proves coverage and authority topology, while semantic correctness remains code-grounded review; no generated “all prose current” claim is introduced. |
| R26-65F: upstream feedback | AC26-65F: Slice 55 and Slice 60 outputs are rerun and may be amended when this slice changes a shared classification, owner boundary, or operation mapping; amendments retain their original RED/GREEN or code-grounded acceptance. |
| R26-65G: exact candidate | AC26-65G: focused lifecycle/parity/docs checks, strict security scans, `agent-verify`, and fresh installed Python/TypeScript/CLI surface smokes pass at the final commit. Native/platform evidence may be reused only with a recorded diff proof that runtime/package inputs are unchanged; otherwise the affected witness is rerun. |
| R26-65H: completion state | AC26-65H: a schema-validated candidate manifest and release state bind the final exact commit, supersede Slice 50 as the integration candidate, leave publication unauthorized, and set no completion claim before all evidence passes. |

## TDD and execution sequence

1. Freeze the post-Slice-60 owner inventory and specify the minimal metadata
   extension. Do not design an exemption for a known-stale owner.
2. Add isolated RED fixtures for missing current profiles, invalid semantic
   authority targets, current-to-historical delegation, ambiguous/cyclic owner
   chains, missing witnesses, and valid evidence-only links.
3. Implement the checker and catalog changes; reconcile errors and any owner
   documents needed to make the real catalog conform honestly.
4. Rerun Slice 55 parity mutations and Slice 60 source-grounded comparisons.
   Update earlier artifacts if this slice changed a shared fact.
5. Run focused checks, strict security, `agent-verify`, and exact fresh binding
   surface smokes. Determine reusable versus rerun native evidence from the
   actual diff, not from a planning assumption.
6. Write the final candidate manifest/status and advance release state only
   after every required receipt resolves to the exact candidate commit.

## Stop gates

Stop on an unresolved semantic owner, a checker design that blesses stale prose,
an implementation/public-contract change, an invalidated platform witness that
cannot be rerun, or any request to integrate, tag, or publish.
