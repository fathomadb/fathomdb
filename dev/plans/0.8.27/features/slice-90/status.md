---
title: FathomDB 0.8.27 Slice 90 — implementation status
status: IN_PROGRESS
target_release: 0.8.27
---

# Slice 90 implementation status

Requirements, acceptance criteria, design and TDD batches are reconciled in
the [plan](plan.md). Independent design review passed after the recorded
corrections. No runtime implementation, checkpoint, code review or final
verification is yet claimed.

The operational source entry is current `release/0.8.27` at `e689000d4` or a
documentation-only descendant. Historical `7a2f9bf9` remains the D27
performance reference. Slice 85 recovery code is already in release ancestry,
but its official public/hidden comparison and GPU evidence are outstanding and
release state still binds the earlier Slice 85 candidate. The local public
capture now has enough disk to rerun; the local NVIDIA driver is unavailable.

The reviewed plan is committed at `77b019c39`. The public source surface was
captured at that commit: 13 rows passed, with the capture saved at
`/tmp/fathomdb-s90-entry-public.json` (SHA-256
`0af7b505ab18a52984dc2cf45151d6f413822df0ffce06d9357466d7032bbaa1`).
The official hidden capture stopped at CUDA preflight because `nvidia-smi`
exited 9; no hidden PASS is claimed.

The measurement-only D27 harness was merged into `release/0.8.27` at
`8a7ad580f` from reviewed implementation commit `177a50ac2`. The temporary
implementation and historical-entry worktrees and implementation branch were
removed after merging. RED/GREEN commits and 34 focused
tests cover the frozen workload, raw-to-receipt linkage, invalid attempts and
host process visibility. Sol code review and independent Terra verification
passed at that exact commit; Terra also passed Ruff, Markdown lint, whitespace
checks, a sandbox rejection before output/build and a read-only host process
preflight. The current-source census, inherited Slice 85 delta and item-specific
owner decisions are recorded in `current-source-inventory.md`. The design
amendments passed independent `gpt-6-sol` high review. No semantic engine
runtime change or full Slice 90 verification is claimed.

The runner built and executed against unchanged historical `7a2f9bf9` source,
but the first two entry attempts were invalidated by host swap-in during their
first repetition. The second attempt recorded `pswpin` moving from 581794 to
581913; a separate 45-second idle sample also rose by seven pages. Its retained
attempt, raw and invalidation files under `/tmp/fathomdb-s90-d27-entry-2` have
SHA-256 values `f1a56292a55f535c7141cfadea5038cdb058876cb1a787da9a042e5d0f55fc63`,
`55f1421e09d63bb548b3748b04e24c7a89099b7c899f858d2d1d19390baa8219`
and `36d768fdc145f48add493b1ac962679333dab04241adf357ea49e6bfb28d820a`,
respectively. These are invalid-attempt audit artifacts, not an entry receipt
or performance PASS. They predate the reviewed process-visibility protocol
amendment and cannot be promoted under its new hash. The host has ample
available RAM but nearly full swap; the frozen protocol invalidates swap
movement. These attempts remain nonqualifying. Phase 2 RED/GREEN work may
proceed under the 2026-10-01 current-host HITL direction, while the runtime
checkpoint and structural Phase 3 still require a valid historical entry.

The current reviewed harness then built and executed against the same exact
historical source with protocol SHA-256 `6a9ec1e3eff134c418ea84f659d4f1424cf75773be3d12fe3e1b45524a793de9`.
Its first 91.51-second repetition completed, but strict validation exited 1:
`pswpin` rose from 595891 to 595896, and the host process view saw a pytest
process that has been sleeping for six days (PID 2356747). The retained
attempt/raw/invalidation files under
`/tmp/fathomdb-s90-d27-current-harness-entry` have SHA-256 values
`3d42c5c90f968521fa899ae91f91485759acb3dddc412581020c539609051cc2`,
`6a9c337c5c0dc5b18966fcfae34ba783934c5c55c15f30bcd06da1fb4840c8d8`,
and `25a63acedb087045e08f072f94b57aee96f8e49cdacf5518fcd8376d10120888`.
This proves exact-source execution of the current harness, not a qualifying
historical entry.

The local NVIDIA kernel module is 580.173.02 while NVML is 580.178.04.
Per HITL `seq-296`, this mismatch cannot be resolved for Slice 90: proceed
with needed GPU tests on windchill3 as installed and record their outcomes.
The strict `scripts/test-feature-complete.sh` gate exited 2 because its
`nvidia-smi` query exited 18. The official `hidden_surface.py capture` of
`f5bc7ca5d` also exited 2 at the same CUDA preflight. Neither produced a
GPU or hidden-surface PASS. No driver repair is a prerequisite to continuing
available Slice 90 work.

Next: continue staged RED/GREEN runtime work on windchill3 while settling the
D27 swap policy. Collect a valid historical entry under the ruled protocol
before the candidate-bound checkpoint and structural Phase 3. Report the
remaining GPU and Slice 85 recovery evidence as its actual result. Slice 90
remains IN_PROGRESS.
