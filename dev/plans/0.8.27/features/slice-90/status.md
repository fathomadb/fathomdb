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

The measurement-only D27 harness has RED/GREEN tests and independent review
in the Slice 90 implementation worktree. It builds and executes against the
unchanged historical `7a2f9bf9` source, but the first two entry attempts were
invalidated by host swap-in during the first repetition. The second attempt
recorded `pswpin` moving from 581794 to 581913; a separate 45-second idle
sample also rose by seven pages. Both attempts retain raw and invalidation
artifacts under `/tmp/fathomdb-s90-d27-entry*`; neither is an entry receipt or
a performance PASS. The host has ample available RAM but nearly full swap,
and this runner sees host-wide swap counters from a PID namespace. The frozen
protocol invalidates any swap-in/out during a repetition. Do not relax that
rule or start semantic runtime changes before a valid historical entry.

Next: complete current-source owner/scanner inventory and independent harness
verification; obtain a quiet performance runner and working NVIDIA driver;
capture a valid historical D27 entry and remaining Slice 85 qualification;
then begin the staged RED/GREEN runtime work.
