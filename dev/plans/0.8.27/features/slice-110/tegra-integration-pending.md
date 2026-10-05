---
title: Slice 110 Tegra allocator integration intake
status: OPEN
target_release: 0.8.27
date: 2026-10-05
---

# Slice 110 Tegra allocator integration intake

On 2026-10-05, the clean
`/home/coreyt/projects/fathomdb-worktrees/slice-110-tegra-allocator-fix`
worktree was fast-forwarded to
`llm/slice110-tegra-allocator-fix` at `77d742153`. This is a branch intake,
not a release merge or Slice 110 qualification. At intake, the release branch
had ten commits absent from the Tegra branch, and the Tegra branch had eighteen
commits absent from release. Release state still has Slice 110 `IN_PROGRESS`
and `next_slice: 110`; Slices 114 and 115 completed under separate HITL
sequencing exceptions. Preserve that state when integrating branch documents.

## Branch result and limits

The branch's `dev/plans/0.8.27/features/slice-110/tegra-allocator-handoff.md`
and `dev/plans/runs/0.8.27-slice-110-tegra/receipt.md` report an
aarch64-Linux-only vendored cudarc fallback when the Orin default CUDA memory
pool cannot be obtained. Reported final-code checks include the fragmented
address-space regression 12/12, vendored unit tests 15/15 with a RED witness,
Node 25.9.0 in-tree and installed 10/10 each, forced CPU 3/3 in each form, and
a rebuilt Tegra Python wheel 10/10 (five normal, five fragmented). These are
branch receipts; integration must bind fresh results to the merged code.

The branch did not compile x86_64. Its off-target proxy test exercised the
stock allocator path, but a real x86_64 build remains a CI obligation. The
branch's `agent-verify` stopped at the Slice 90 checkpoint lint failure also
seen at its baseline; separate typecheck and security legs passed. All-tier
`agent-test` reported 177/183 suites, with six failures attributed in the
receipt to missing local release ref, Python 3.13 `cgi`, shallow-clone
transport, Python loader path, worktree editable-install restrictions, and a
Python timing failure under suite load. This is not a full gate pass. The
fallback's synchronous path is reported about 1.9–2.4 times slower per steady
embed, and very heap-heavy Node processes can still fail at `cuInit` before
allocator selection.

## Work before integration and closure

1. **Finish the Tegra agent's review follow-up.** The fix-2 reviewer reported
   no compile, behavior or governance defect and considered it safe to push,
   but identified six low-severity items. The Tegra agent owns the pending
   remeasurement and corrections: remove the explicit-pool confound from the
   teardown probe; scope the pool-survival claim to the measured Jetson;
   document that the aarch64 Linux rule also reaches unmeasured CUDA hosts;
   remove or replace a tautological test; skip the regression before probing
   unmeasured Jetsons; and reconcile slowdown ranges. Do not duplicate that
   work while it is underway. If the teardown result changes, reassess the
   once-per-device cached decision and repeat the affected runtime tests.
2. **Review and qualify the final candidate.** After the Tegra agent's final
   push, independently review the exact vendored delta, patch
   reproducibility, governance exception, tests and evidence. Run real x86_64
   CI compilation and applicable hosted platform builds, then repeat
   installed Node forced-CUDA and allocation-witness checks on the Orin. Run
   the required full source gate on a capable release executor; classify any
   environmental failure against an unchanged baseline rather than counting
   it as green.
3. **Reconcile branch state before merge.** Bring the allocator commits onto
   the current `release/0.8.27` without replacing the completed Slice 114/115
   records or the live `next_slice: 110`. The branch handoff reports its todos
   ledger at seq 270 versus origin/main at seq 242. Use `ledgerwatch` and
   `ledgerwrite` to inspect the full current tails and reconcile the single
   appended row before changing any ledger or generated release-state view.
   Recheck the branch and release tips at integration time; the SHAs above are
   this intake snapshot.

Only after these items have candidate-bound evidence should Slice 110 status
and the release ladder advance. No publication follows from this intake.

## Separate early-initialization investigation

The repository owner's 2026-10-05 direction supersedes the branch handoff's
statement that early `cuInit` will follow **in Slice 110**. The Tegra agent may
investigate it and other approaches, but they are unlikely to land in 0.8.27.
They are not prerequisites for integrating the reviewed allocator fallback or
closing Slice 110. Any 0.8.27 inclusion needs its own explicit scope decision,
review and candidate-bound verification. The very-heavy-heap `cuInit` refusal
remains a stated limitation of the allocator fix meanwhile; forced CUDA must
not silently fall back to CPU.

The proposed experiment is a silent, non-failing `cuInit` at Node addon load,
compiled only for aarch64 Linux with a CUDA feature and skipped when both
device policies are CPU. It would keep the typed error and refusal contract,
but make address-space fragmentation errors clearer. Before claiming a user
remedy, the agent must verify that `node --import fathomdb` works with the
published package layout. Proposed Jetson guidance would then cover early
import, the roughly 61 GiB additional virtual-size observation, late-import
limits and synchronous-path cost. The proposed RED/GREEN check imports, grows
the heap to one million objects and forces CUDA. Its investigation matrix
includes Node 24/25/26, late import with and without `--import`, installed
packages, CPU-only and no-GPU cases, import-time cost, and every
`agent-verify` leg. Record results for a later release decision; do not turn
this experiment into a Slice 110 acceptance gate by implication.
