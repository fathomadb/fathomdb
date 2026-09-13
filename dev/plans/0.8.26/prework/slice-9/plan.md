---
title: FathomDB 0.8.26 Slice 9 — approved-prework implementation
status: ACTIVE
execution_authorized: seq-289
---

# Slice 9 plan — approved-prework implementation

## Slice-complete workflow

This plan adopts the full [lean slice execution contract](../../slice-execution-contract.md):
delta reconciliation, need/requirement/acceptance completion, design review,
RED/GREEN implementation where behavioral, code review, independent
verification, status, and any temporary worktree cleanup.

## Purpose

Implement only feature-independent preparation items approved and allocated to
Slice 9 by Slice 8 so feature work begins from a clean, verified,
contract-consistent repository.

## Ruled scope

Per `seq-286`, implement only:

1. **P26-01:** repair the inconsistent 0.8.25 publication receipt/state and add
   a regression check that keeps published lifecycle authority truthful.
2. **P26-02:** only after P26-01 is green, create the 0.8.26 release-state JSON
   and board, generate their owned views, and prove release preflight.
3. **P26-03:** make preflight select an explicit authoritative main ref and
   cover stale-local, remote-only, and offline behavior.
4. **P26-05:** correct only `dev/platform-capabilities.json`,
   `src/ts/README.md`, `dev/plans/README.md`, and `AGENTS.md` as enumerated by
   Slice 2. CLI verb/reference work remains in Slice 30.
5. **P26-08:** correct the inaccurate `download-artifact` v4.3.0 comments; do
   not upgrade its runtime version.

P26-04 is an execution condition: use checkout-owned dev tools, fresh artifact
environments, and a disk-budget check before broad matrices. P26-06 may enter
only if its markdown dependency is a hard build blocker, in which case stop
for HITL before expanding scope. P26-09 is assigned to Slice 50. All other
postponed or rejected proposals remain untouched.

Feature implementation for Slices 10–50 is prohibited here.

## Entry reconciliation

Enumerate changes since this draft, re-read `seq-286`, review the Slice 0–7
evidence and exact Slice 2 file inventory, and inspect related work already in
the repository. Update, approve, reject, or narrow this plan if evidence has
changed; do not broaden it without a new HITL ruling. Complete any Slice 9
needs, requirements, and acceptance criteria before implementation.

## Execution discipline

1. Review and complete [`design.md`](design.md), obtain independent design
   review, and resolve findings before code changes.
2. For P26-01 through P26-03, commit or stage visible failing tests before the
   smallest GREEN implementation. Treat the two documentation corrections as
   mechanical only after confirming their exact source of truth.
3. Preserve one writer per checkout and isolate any concurrent writer in a
   separate worktree.
4. Update public contracts and successor ADRs in the same change as any public
   surface they govern.
5. Test build/CI corrections with the narrowest non-publishing reproducer, then
   the affected typed verb or workflow validator; do not weaken a gate.
6. Run focused checks after each finding. Then run `./scripts/agent-verify.sh`;
   run the broader non-publishing gate because this slice changes release
   authority or preflight behavior.
7. Obtain independent code review and independent verification, allow no more
   than two focused correction cycles, and return unresolved findings to HITL.
8. Write `status.md` with RED/GREEN commits, review verdicts, exact evidence,
   deferred items, and the next slice. Merge and remove a temporary branch or
   worktree if one was used.

The P26-03 implementation performs no fetch: prefer `origin/main`, fall back
to local `main` with a loud warning only when the remote-tracking ref is
absent, hard-fail when neither resolves, and emit `main_ref` with `main_sha`.
P26-08 changes only the seven `d3f86a...` pin comments to v4.3.0.

## Exit criteria

Every ruled Slice 9 item is implemented and evidenced; release state names
Slice 10 as next; every later, postponed, or rejected item remains untouched
and durably allocated; focused checks and the appropriate repository gates are
green; and no publication or feature behavior occurred.
