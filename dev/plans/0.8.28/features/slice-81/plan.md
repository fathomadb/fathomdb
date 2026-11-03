---
title: FathomDB 0.8.28 Slice 81 — plan
status: PLANNED
baseline_sha: 3fc2df15e4dfadd67b9fb57e5e7cea3d62613ead
target_release: 0.8.28
---

# Slice 81 plan

## Objective and placement

Select or reject an erasure maintenance mechanism using real physical-removal,
foreground availability and durable-progress evidence. Recommend the measurable
operating envelope and the remaining SLA decisions. Scope/authorities are indexed
in [README](README.md#references); [requirements](requirements.md) are normative
for this POC, not a successor to shipped erasure semantics.

Owner places Slice 81 after 80 and before 90 on `dev/0.8.28`. It is the explicit
exception for this reserved slot; no general gap policy changes. Slice 80 stays
complete. Slice 90 retains its filtered-body-FTS scope, now following 81. The
serial release policy and 0.8.29 handover after final release development remain.
Only 81 is placed: the other CH units remain proposed and unnumbered.

## Reconciliation since the original maintenance proposal

1. Slice 80 and P1/P2/P3 are complete within their stated bounds; their tools and
   receipts are reusable, not repeated wholesale (S81-4/5).
2. Native certified current-node reads isolate history, but secure full-family
   reconciliation rose from 93–106 ms to approximately 4.6 s. Healthy-read speed
   does not establish maintenance or integrated public acceptance (S81-4).
3. A separate ordinary-Engine diagnostic confirms physical node-FTS term retention
   after logical erase/WAL truncation. Candidate-cache repairs did not fix shipping
   code; other registered families need independent positive witnesses (S81-4).
4. Owner permits lazy physical erasure with guaranteed deletion and an SLA. The
   previous eager strategy is a reference candidate, not the selected repair.
   No numeric SLA or clock is approved (S81-2/3).
5. Research identifies bounded merge and replacement/retirement alternatives.
   A background thread does not remove SQLite's single writer or snapshot pins.
   Existing global pending-physical fencing also blocks unrelated Engine writes;
   automatic resume/scoped admission require successors (S81-3/6/9).
6. Denial needs an explicit admission boundary and in-flight-read policy. Pending
   physical data must not taint statistics, caps, graph or historical/evidence
   routing. Filtering results after ranking is insufficient (S81-3/7).
7. The old next step was unclear. Slice 81 now owns the bounded mechanism POC
   before a revised CH1 production proposal. Historical backend selection and
   temporal/ranking redesign are not allocated here (S81-1/2).

## Scope and deliverables

- Core A/B: eager secure-delete reference versus committed incremental cleanup;
  legacy ghosts and fresh erasure, zero subsequent traffic and foreground load.
- Conditional C: staged replacement/retirement only under the experiment's fixed
  trigger. No universal backend or scheduler framework.
- Real family/mode inventory and physical witnesses; separate mechanism and
  actual-Engine composition verdicts. A seam/bypass is never public qualification.
- Committed harness/tests/manifests, complete outcome tables and rejected/incomplete
  cells; a recommendation with SLA clock/reader/capacity/finalization conditions.
- A draft production follow-up delta only if evidence supports one; owner approval
  remains required before production remediation.

No new public API, production erasure success meaning, format migration, shipped
fence relaxation, temporal ledger integration, historical retrieval backend or
product performance threshold is delivered here. Existing FTS erasure remains an
adoption blocker until later remediation; a successful POC cannot close it.

## Work order and reviews

1. **Planning:** Astra-medium reviews requirements/AC, experiment design, draft
   implementation design and test oracles. Resolve required findings before code.
2. **Inventory and RED:** freeze real table modes and ownership, reproduce baseline
   ghost/global-fence behavior, stage/commit one genuine failing behavior oracle
   before each candidate GREEN. Existing guards may already pass; label them honestly.
3. **Core implementation:** implement only the standalone experiment and necessary
   engine-owned `test-hooks` seams. Identify/review a seam before using it; preserve
   the ordinary Engine default. No concurrent writers in a checkout.
4. **Correctness gate:** Astra code review and independent subagent verification;
   do not measure an unsafe candidate as a successful mechanism.
5. **Measurement:** freeze source/build/profile/fixture/query/control manifests and
   run the bounded [experiment](experiment-design.md). No competing builds/tests.
6. **Decision/closeout:** independent evidence reconciliation, Astra interpretation
   review, update status/results and release views. A proved rejection is valid;
   missing required evidence is incomplete, not success. Size/approve the production
   repair separately. Merge/clean any task worktree/branch and push explicit refs.

This request delivers planning files only; no POC code, run or product SLA is
claimed. Prior approval of the POC direction persists, but a broad campaign or
changed budgets must not be inferred from this scaffolding request.

## Quality and validation

Consulted S81-8 on 2026-11-03. Apply real-database differential witnesses, physical
positive controls, property/state-machine boundaries, selected commit/fault/reopen
cases and leak/queue cleanup. New evidence changes the assessment: durable closure
and global fencing are central to availability. Defer exhaustive OOM/VFS fuzzing,
MC/DC and a new test framework; these do not answer the bounded mechanism question.

For this documentation phase run Markdown and release-view validators only. During
POC development run focused fmt/clippy/tests and failure replay; run the required
final code gate once when source/test/executable changes land, with scoped repairs.
An experiment-crate pass is not integrated AC-076/081 or release qualification.
CI inspection/actions use `gh` only. No Steward or Orchestrator role is used.
