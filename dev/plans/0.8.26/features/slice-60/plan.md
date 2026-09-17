---
title: FathomDB 0.8.26 Slice 60 — current design-owner reconciliation
status: APPROVED
target_release: 0.8.26
---

# Slice 60 plan — retrieval, recovery, and engine owners

## Need and outcome

N26-60: maintainers need one current explanation of retrieval, recovery, and
engine behavior that does not make them reconstruct 0.8.26 from historical
0.6.0 prose and later amendments.

Make the three maintained designs trustworthy current-state owners. This is a
documentation reconciliation: accepted ADRs, public interfaces, requirements,
schema, implementation, and tests determine truth. Product runtime behavior is
not changed. A public contract that contradicts the already-enforced
implementation may be corrected only when code and tests agree and no accepted
decision is changed; a genuine implementation or policy defect stops the slice.

## Reconciliation since the draft

The draft landed at `65f69ca1`. Nine later implementation commits completed
Slice 55 before this entry review. The exact delta and disposition are:

1. Slice 55 added the executable 69-token/44-operation SDK parity model,
   real-surface Python and TypeScript discovery, and mutation tests. Reuse that
   operation classification rather than building a competing surface list.
2. Slice 55 restored the already-approved standalone TypeScript `rerank` peer
   and amended `dev/design/bindings.md` plus the Python/TypeScript interfaces.
   The retrieval owner still calls reranking deferred and its two-value branch
   list omits live `TextEdge` and `GraphArm`; those claims are rejected.
3. No post-draft commit changed `dev/design/retrieval.md`,
   `dev/design/recovery.md`, or `dev/design/engine.md`. Their remaining drift is
   inherited from the 0.6.0 baseline, not a moving Slice 55 target.
4. Slice 46 already classifies all three files as maintained cross-release
   owners. Its bounded patches did not make their 0.6.0 bodies current. Update
   those owners in place; do not add or reclassify an owner.
5. Assigned-source review found one accepted-authority conflict and one adjacent
   contract correction. The CLI
   interface describes `recover` as an NDJSON progress stream although every
   live action and process test emits one JSON object. REQ-036 and recovery
   prose describe every doctor path as bit-preserving/read-only although
   `doctor recompute-mean` transactionally changes derived vector state. Resolve
   the conflict through a narrow successor ADR that grandfathers only this
   atomic, non-lossy, derived-only action under `doctor`; then correct REQ-036.
   Do not weaken `recover`'s exclusive ownership of data-loss-authorized work or
   treat this as permission for more mutating doctor actions.
6. The Slice 3–8 proposal register allocates no unresolved D26 item here. All
   release decisions are ruled. Slice 65 retains generalized recurrence guards,
   error-owner reconciliation, and candidate requalification.
7. Release navigation drifted while Slice 55 closed: the master slice table and
   feature README still call Slice 55 or 50 next. Closeout must reconcile them
   to state-file-derived Slice 65 after Slice 60 completes.

Disposition: approve the three-owner rewrite with one narrow ADR successor and
bounded requirements/CLI contract corrections. Reject product changes, a new
checker, a new public operation, generalized mutating doctor scope, migration
support, historical bulk cleanup, and Slice 65 work.

## Scope

In scope:

- commit `inventory.md`, bounded to the normative claim clusters below, with an
  authority, implementation witness, test witness, and disposition for each;
- rewrite `dev/design/retrieval.md` around live lexical, vector, graph, fusion,
  cross-encoder, device, validity/frozen-view, evidence, and explanation paths;
- rewrite `dev/design/recovery.md` with the exact current doctor/recovery
  inventory and accurate effect classes;
- rewrite `dev/design/engine.md` as the current schema-34 profile, including
  `logical_id`-alone active identity, caller and projection write lanes,
  `commit_gate`, pooled readers, current receipts, and shared vector storage;
- add an accepted successor to `ADR-0.6.0-cli-scope`, update its decision-index
  relationship, and correct REQ-036 plus CLI recovery-output wording;
- remove obsolete current claims without deleting unique rationale; and
- cross-read all three owners after the engine rewrite.

Out of scope:

- engine, schema, migration, binding, or CLI implementation changes;
- new public APIs, compatibility shims, migration support, or ADR decisions
  beyond the single derived-maintenance successor required by the discovered
  authority conflict;
- changes to accepted identity, concurrency, vector, retrieval, or recovery
  behavior;
- changing the Slice 55 canonical operation map; and
- generalized lifecycle/semantic-owner checking and candidate requalification,
  which belong to Slice 65.

## Requirements and acceptance

| Requirement | Acceptance criterion |
| --- | --- |
| R26-60A: bounded grounded inventory | AC26-60A: `inventory.md` covers each named retrieval, recovery, and engine claim cluster with accepted requirement/ADR/interface authority, implementation witness, test witness, and disposition; every normative section in the three owners maps to one cluster or delegates to a narrower maintained owner. |
| R26-60B: retrieval truth | AC26-60B: `retrieval.md` describes live lexical node/edge/projected text, vector shortlist/exact rerank, RRF and optional reweights/CE, standalone rerank, graph behavior, devices/fallback, validity/filter/frozen eligibility, bounds, explanations, and evidence without presenting deferred behavior as shipped. |
| R26-60C: recovery truth | AC26-60C: `recovery.md` states its operator boundary, enumerates the complete current doctor commands and five recovery actions, classifies immutable, engine-backed, artifact/cache, derived-maintenance, and loss-authorized effects accurately, matches one-object recovery output, and does not imply SDK `doctor`/`recover` or CLI-only erasure. |
| R26-60C1: authority conflict closure | AC26-60C1: an explicitly owner-authorized successor ADR supersedes only the doctor-wide read-only/bit-preserving clause, permits only the already-shipped atomic non-lossy derived-only `recompute-mean` exception, preserves `recover` for CLI data-loss-authorized operator recovery, and REQ-036/index wording agrees. |
| R26-60D: engine truth | AC26-60D: `engine.md` agrees with schema-34 fresh admission, `logical_id`-alone identity, write-cursor identity, current write receipt, primary caller-writer versus projection-worker topology, shared `commit_gate`, pooled readers, atomic batches, cursor/readiness distinction, and shared `vector_default` layout. |
| R26-60E: no hidden product rewrite | AC26-60E: the product-code diff is empty. Any actual implementation-versus-accepted-authority conflict is recorded with impact and stops completion pending explicit scope. |
| R26-60F: feedback reconciliation | AC26-60F: retrieval, recovery, requirements/CLI corrections, inventory, and Slice 55 mapping are reread after engine is final; Slice 55 artifacts change only if their RED/GREEN parity contract requires it. |
| R26-60G: TDD and review | AC26-60G: baseline semantic probes fail on the stale owners; the same probes pass after rewrite; independent design review passes before implementation; independent content/code review has no unresolved P1/P2 defect. |
| R26-60H: verification | AC26-60H: design lifecycle/reference/authority checks, Markdown/link/docs checks, focused source comparisons, product-code diff checks, and `agent-verify` pass; an independent verifier reruns proportional evidence. |

## TDD implementation sequence

1. Record acceptance authority for the narrow successor ADR, then commit the
   reconciled plan, design, inventory, ADR, and decision-index relationship
   after independent design review. If acceptance is unavailable, stop.
2. RED: record source-comparison probes that fail because the owners remain
   0.6.0 profiles, omit live rerank/recovery/concurrency facts, and retain the
   disproven claims. This documentation-only mechanical exception uses the real
   source/interface comparison as its oracle; it does not manufacture a product
   test or generated golden.
3. GREEN: rewrite the three owners plus the accepted ADR/index and bounded
   requirements/CLI wording until the same probes and focused comparisons pass.
4. Cross-read the owners and compare the exact CLI enum/actions and Slice 55
   operation map. Amend only facts made inconsistent by that pass.
5. Obtain independent content/code review. Preserve a focused RED witness for
   any factual review finding before the smallest correction.
6. Run proportional documentation/source checks and `agent-verify`; have an
   independent verifier reproduce the evidence. Record chronology and verdict
   in `status.md`, then advance the single-writer release state.

The reproducible focused commands are:

- RED/GREEN presence probes: `rg -q` for `target_release: 0.8.26`, standalone
  cross-encoder, `orphan-provenance`, `commit_gate`, and `vector_default` in the
  assigned owners; inverse `rg` for the exact stale rerank-deferred,
  progress-stream, per-kind-identity, restore, and one-field-receipt claims;
- `python3 scripts/check-design-lifecycle.py`;
- `bash scripts/tests/test_check_design_lifecycle.sh`;
- `bash scripts/check-release-state-views.sh`;
- `./scripts/agent-lint-md.sh`;
- `git diff --name-only b770de01 -- src/rust src/python src/ts` must be empty;
  and
- `./scripts/agent-verify.sh`.

## Stop gates

Stop on an actual product defect, accepted-ADR conflict, unresolved owner, new
public behavior, required schema/runtime change, or loss of unique historical
rationale. Do not turn documentation drift into product surgery.
