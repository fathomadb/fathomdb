---
title: FathomDB 0.8.26 Slice 65 — lifecycle recurrence guards and requalification
status: REVIEW_CANDIDATE
target_release: 0.8.26
---

# Slice 65 plan — semantic ownership gate and candidate requalification

## Outcome

Turn the corrected current-owner model into enforceable lifecycle structure,
repair the error-ownership edges exposed by the adversarial review, and bind a
new exact non-publishing candidate after all post-Slice-50 changes. This is the
only broad verification slice in the reopened ladder.

## Need and reconciliation since the draft

N26-65: maintainers and release reviewers need a non-vacuous recurrence gate
that distinguishes current semantic authority from historical evidence, plus
an exact replacement candidate after the reopened ladder changed runtime and
package inputs.

The draft landed at `65f69ca1`. Eighteen later commits changed 53 files before
this entry review. The exact categories and dispositions are:

1. Slice 55 added the executable 69-token/44-operation Python/TypeScript parity
   model, real-surface discovery, mutation tests, and the already-authorized
   TypeScript standalone `rerank` peer. Reuse its canonical operation map and
   mutation suite; do not create another SDK inventory.
2. Slice 60 rewrote `retrieval.md`, `recovery.md`, and `engine.md` from their
   historical profiles into current schema-34 owners. Their reviewed inventory
   supplies authority and implementation/test witnesses for the new gate.
3. Slice 60 also accepted one narrow CLI-scope successor ADR and changed the
   operator-feature Rust/CLI path for malformed-WAL recovery. These runtime and
   package-input changes invalidate automatic reuse of Slice 50 native/package
   qualification; affected artifacts and platform receipts must be rebuilt at
   the Slice 65 candidate SHA.
4. Slice 55 and Slice 60 both passed independent design/code review and the
   unchanged 117/117 canonical gate. Slice 65 reruns their focused owner/parity
   proofs but does not reopen settled operation spelling, recovery policy, or
   product behavior.
5. Neither `dev/design/document-lifecycle.json` nor `dev/design/errors.md`
   changed after the draft. The catalog still proves structural lifecycle only,
   and the error taxonomy still delegates eight current semantic rows to
   historical/release-local plans. The drafted recurrence and error-owner work
   therefore remains necessary.
6. The Slice 46 inventory already contains reviewed human-readable authority
   and witness dispositions for all 25 maintained owners. Promote that bounded
   evidence into one compact companion catalog instead of duplicating prose or
   adding semantic keyword inference.
7. No Slice 3–8 draft item, unruled decision, deferred design, dependency
   upgrade, schema change, migration, or public operation is additionally
   allocated here. All release decisions are ruled.

Disposition: **adjust and approve subject to independent design review**. Keep
the drafted lifecycle/error/candidate outcome, add the exact companion-catalog
schema and candidate-evidence consequences above, and reject runtime repair,
prose parsing, broad historical cleanup, and a second SDK/candidate framework
as overbuild. The reviewed inventory is recorded in `inventory.md`.

## Scope

In scope:

- add `dev/design/current-owner-authority.json`, a machine-readable exact-set
  companion to the lifecycle catalog containing current-profile,
  semantic-authority, implementation/test witness, and evidence-only
  relationships for the 25 maintained design owners;
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
- a second lifecycle inventory, SDK operation map, package-smoke framework, or
  broad rewrite of historical design records;
- publication, tagging, registry mutation, or main integration.

## Requirements and acceptance

| Requirement | Acceptance criterion |
| --- | --- |
| R26-65A: explicit current authority | AC26-65A: the companion catalog covers exactly the lifecycle catalog's maintained set; every entry names profile `current`, at least one existing semantic authority, and at least one implementation/test or bounded enforcement witness; metadata cannot elevate a design above ADR/interface authority. |
| R26-65B: valid semantic graph | AC26-65B: semantic-authority edges are duplicate-free, existing, acyclic across maintained owners, and terminate at a maintained owner or an accepted ADR/locked interface/repository invariant; historical/proposal/deferred targets are legal only as explicit evidence, never current authority or implementation witnesses. |
| R26-65C: error ownership | AC26-65C: current error semantics formerly delegated to historical or release-local slice documents are incorporated into maintained owners or accepted contracts, with the old documents retained as evidence. |
| R26-65D: non-vacuous recurrence tests | AC26-65D: committed RED fixtures prove missing/extra/duplicate profiles, invalid or historical authority, maintained-owner cycles, missing/invalid witnesses, authority/evidence overlap, and non-current profile claims fail; valid external termination and evidence-only historical references pass; existing structural lifecycle controls remain intact. |
| R26-65E: honest gate claim | AC26-65E: documentation states that automation proves coverage and authority topology, while semantic correctness remains code-grounded review; no generated “all prose current” claim is introduced. |
| R26-65F: upstream feedback | AC26-65F: Slice 55 and Slice 60 outputs are rerun and may be amended when this slice changes a shared classification, owner boundary, or operation mapping; amendments retain their original RED/GREEN or code-grounded acceptance. |
| R26-65G: exact candidate | AC26-65G: focused lifecycle/parity/docs checks, strict security scans, `agent-verify`, and fresh installed Python/TypeScript/CLI surface smokes pass at one immutable candidate commit. Because Slices 55–60 changed Rust/TypeScript/package inputs, the five-target native matrix and distinct Windows WAL installed-wheel receipt are rerun at that same SHA rather than reused. |
| R26-65H: completion state | AC26-65H: a strictly validated Slice 65 candidate manifest and release state bind the immutable implementation candidate plus its exact receipts, preserve Slice 50 as historical evidence, leave publication unauthorized, and distinguish the later documentation-only closeout commit from the candidate. |
| R26-65I: review and cleanup | AC26-65I: independent design review passes before RED implementation, independent code review and independent verification report no unresolved P1/P2 finding, Slice 65 writes its status/chronology/review records, and the retained release worktree is clean without a new branch or worktree. |

## TDD and execution sequence

1. Commit this reconciliation, the post-Slice-60 inventory, and the exact
   companion-catalog/checker design; obtain independent read-only design review
   and resolve every P1/P2 before implementation.
2. **RED:** extend `test_check_design_lifecycle.sh` with isolated fixtures for
   each AC26-65D invalid and valid case. Run the real checker and preserve the
   failing result before changing checker/catalog implementation.
3. **GREEN:** add the smallest companion catalog and checker extension that
   makes those fixtures and the real 25-owner set pass. Reconcile only the eight
   historical/release-local semantic-owner cells in `errors.md` and the
   smallest maintained-owner text needed to own them.
4. Rerun the Slice 55 parity mutation suite and Slice 60 lifecycle,
   source-grounded, Rust/CLI recovery, and product-diff comparisons. Amend
   earlier artifacts only if this slice changed a shared fact.
5. Obtain independent code review over RED/GREEN chronology, checker safety,
   catalog truth, and error-owner edits. Any factual finding receives a focused
   RED before the smallest correction.
6. Freeze an immutable implementation candidate, run focused checks, strict
   security, `agent-verify`, fresh installed wheel/npm/CLI profiles, generated
   and tracked-tree scans, and exact-SHA native plus Windows WAL jobs. Use the
   existing Slice 50 package/profile/receipt machinery rather than replacing it.
7. Have an independent verifier reproduce the focused and manifest checks.
   Write `tdd-chronology.md`, design/code/verification review records, the
   strict candidate manifest, and `status.md`; then advance the single-writer
   release state with the candidate SHA and a separate closeout SHA.

## Stop gates

Stop on an unresolved semantic owner, a checker design that blesses stale prose,
an implementation/public-contract change, an invalidated platform witness that
cannot be rerun, or any request to integrate, tag, or publish.
