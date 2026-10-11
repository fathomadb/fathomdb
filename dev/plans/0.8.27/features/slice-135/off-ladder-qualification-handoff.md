---
title: Slice 135 off-ladder landing qualification handoff
status: BOARD_AND_PLAN_RECORDED_STATE_PENDING
target_release: 0.8.27
---

# Off-ladder landings requiring integrated qualification

Two fixes landed outside the 0.8.27 slice ladder. The release plan and board
now name both under the 2026-10-07 off-ladder section at `b65283317`. The
release state contains the Slice 110/117 and pool-study rulings but does not
yet bind these two landings to Slice 150 verification. This handoff does not
reopen any ruling.

| Landing | Phase 1 check | Slice 150 owner check |
| --- | --- | --- |
| `96796fe04` — corrected embedder close, releasing it after close outside `close_lock`; 0.8.27 prerequisite for the later pool study | Verify ancestry in the exact candidate. Include focused close/reopen, pending embedding/projection work and failure/cancellation observations with bounded completion and resource state. Reuse its existing regression cases against current candidate, and record their exact source and binary identities. | Recheck integrated lifecycle and installed bindings on the final candidate. Do not treat the 0.8.28 pool study as verification of this 0.8.27 fix. |
| `c23e2d23f` — 0.8.26+Tegra Pages pin, install command and docs correction | Resolve the commit in the release branch, verify ancestry and affected files, then check the pinned install path and documentation against an actual qualified Tegra package/install route. Record environment, package identity and any unavailable hardware route. | Confirm the final release artifact and published install instructions retain the pin and command, with an installed Tegra smoke or an explicit qualification limit. |

Both `96796fe04` and `c23e2d23f` are ancestors of the fetched release
commit `b65283317`, which follows `ef4bb42da`. Verify their ancestry again
at the exact measurement candidate. The Tegra fix pins the already published
`0.8.26+tegra` route, so Slice 150's installed smoke must use that route.
The 0.8.27 Tegra publication updates the pin later.
The [focused Tegra-route receipt](results/2026-10-07-off-ladder-tegra-route/README.md)
checks the workflow pin, Python warning and CLI help on the integrated
candidate; an installed AArch64 wheel and live Pages smoke remain open.
The [focused Phase 1 receipt](results/2026-10-07-off-ladder-embedder-close/README.md)
passed three real-database close ownership tests and one timed-out-close retry
test on the repaired candidate. Concurrent pending-work failure/cancellation,
installed bindings and final-candidate integration remain open.
The close fix intentionally releases the engine-owned embedder at `close()`.
Record the 0.8.26/0.8.27 post-close memory behavior and repeated open/close
cycles as a lifecycle difference; do not classify the release as a regression.
Caller-owned embedder references and module-level models have separate
lifetimes.

## Remaining shared-record edit

The release-state writer owns this change on the active release checkout.
Do not create new ladder slices or mark Slice 150 complete to represent the
landings.

1. The release plan and board edits landed at `b65283317`; preserve their
   explicit final-candidate and `0.8.26+tegra` install-route obligations.
2. In `dev/plans/release-state-0.8.27.json`, attach these two off-ladder
   verification obligations to Slice 150 using the state schema's supported
   fields and evidence references. Keep `next_slice`, `landed`, Slice 150's
   `PLANNED` status and generated-view ownership truthful. Regenerate/check
   views with `scripts/check-release-state-views.sh`.

Slice 135 should link its Phase 1 observations to these obligations. The
release-state writer can then apply the shared-record edit without racing
this Slice 135 worktree.
