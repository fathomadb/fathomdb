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

The measurement-only D27 harness is complete through reviewed implementation
commit `177a50ac2` in the Slice 90 worktree. RED/GREEN commits and 34 focused
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
movement. Do not relax that rule or start semantic runtime changes before a
valid six-repetition historical entry.

The local NVIDIA kernel module is 580.173.02 while NVML is 580.178.04;
`nvidia-smi` fails even outside the sandbox. A working host GPU and quiet
windchill3 process/swap state are external prerequisites for the remaining
hidden-surface, Slice 85 recovery and Slice 90 performance evidence.

Next: repair the host GPU driver/library mismatch and stop host swap activity;
run the new protocol's historical D27 entry on unrestricted host `/proc`,
complete the inherited Slice 85 qualification, then begin staged RED/GREEN
runtime work and the candidate-bound checkpoint. Slice 90 remains IN_PROGRESS.
