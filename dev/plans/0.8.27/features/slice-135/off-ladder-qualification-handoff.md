---
title: Slice 135 off-ladder landing qualification handoff
status: DRAFT_FOR_RELEASE_STATE_WRITER
target_release: 0.8.27
---

# Off-ladder landings requiring integrated qualification

Two reported fixes landed outside the 0.8.27 slice ladder. This checkout's
release plan, board and state name neither commit as of this handoff. The
repository owner reports that the Slice 110/117 and pool-study-for-0.8.28
rulings are already recorded on the other machine; this checkout cannot
verify those newer records and this handoff does not reopen the rulings.

| Landing | Phase 1 check | Slice 150 owner check |
| --- | --- | --- |
| `96796fe04` — corrected embedder close, releasing it after close outside `close_lock`; 0.8.27 prerequisite for the later pool study | Verify ancestry in the exact candidate. Include focused close/reopen, pending embedding/projection work and failure/cancellation observations with bounded completion and resource state. Reuse its existing regression cases against current candidate, and record their exact source and binary identities. | Recheck integrated lifecycle and installed bindings on the final candidate. Do not treat the 0.8.28 pool study as verification of this 0.8.27 fix. |
| `c23e2d23f` — 0.8.26+Tegra Pages pin, install command and docs correction | Resolve the commit in the release branch, verify ancestry and affected files, then check the pinned install path and documentation against an actual qualified Tegra package/install route. Record environment, package identity and any unavailable hardware route. | Confirm the final release artifact and published install instructions retain the pin and command, with an installed Tegra smoke or an explicit qualification limit. |

`96796fe04` resolves locally and is an ancestor of this Slice 135 candidate.
The [focused Phase 1 receipt](results/2026-10-07-off-ladder-embedder-close/README.md)
passed three real-database close ownership tests and one timed-out-close retry
test on the repaired candidate. Concurrent pending-work failure/cancellation,
installed bindings and final-candidate integration remain open.
`c23e2d23f` is not present in this checkout's local Git object database at
draft time, so its ancestry and exact diff are **unverified here**. Refresh
the release ref on the machine that received it before claiming inclusion.

## Proposed shared-record edits

The release-state writer owns these changes on the active release checkout.
Do not create new ladder slices or mark Slice 150 complete to represent the
landings.

1. In `dev/plans/plan-0.8.27.md`, add an off-ladder landings note near Slice
   150: both commit IDs, scope, their exact-candidate checks, and Slice 150
   integrated qualification ownership. Keep the pool study in 0.8.28.
2. In `dev/plans/runs/STATUS-0.8.27.md`, add a concise nongenerated board
   note naming both landings and their pending final-candidate checks. Do not
   hand-edit release-state-generated regions.
3. In `dev/plans/release-state-0.8.27.json`, attach these two off-ladder
   verification obligations to Slice 150 using the state schema's supported
   fields and evidence references. Keep `next_slice`, `landed`, Slice 150's
   `PLANNED` status and generated-view ownership truthful. Regenerate/check
   views with `scripts/check-release-state-views.sh`.

Slice 135 should link its Phase 1 observations to these obligations. The
release-state writer can then apply the shared-record edit without racing
this Slice 135 worktree.
