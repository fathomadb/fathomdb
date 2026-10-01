---
title: FathomDB 0.8.27 Slice 90 — implementation status
status: IN_PROGRESS
target_release: 0.8.27
---

# Slice 90 implementation status

Requirements, acceptance criteria, design and TDD batches are reconciled in
the [plan](plan.md). Independent design review passed after the recorded
corrections. Runtime Batches 2a–2e are merged and reviewed; the runtime checkpoint
and final Slice 90 verification are not yet claimed.

The frozen operational source entry was `release/0.8.27` at `e689000d4` or a
documentation-only descendant before Phase 2 semantic edits. The release
branch now includes the reviewed Batches 2a and 2b. Historical `7a2f9bf9`
remains the D27 performance reference. Slice 85 recovery code is already in
release ancestry,
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

HITL then ruled that the six-day sleeping pytest observation invalidated that
attempt and asked for a fresh strict run. After explicit authorization, the
identified stuck memex pytest process and ten additional memex pytest processes
whose identities and unchanged CPU times were verified were stopped with
SIGTERM. The fresh unchanged-protocol run against `7a2f9bf9` completed its
first 91.57-second repetition with no competing process at start or end, but
`pswpin` rose from 597770 to 597778 and `pswpout` from 2924726 to 2924742.
The strict runner exited 1 with `INVALID_ENVIRONMENT` for swap movement only.
Its attempt, raw, and invalidation files under
`/tmp/fathomdb-s90-d27-currenthost-fresh` have SHA-256 values
`674b5386b39a3a9862b8880ffaf06befae8cf6ab6f915d1fa6c03e81294ad13c`,
`06249c5ba7510cd5fb3bf9fe40705a04b5cfdf04851c239656d38306f4f8dfd4`,
and `8f89eec40d20792894d3464fb10678c3f43d756505a6fbbaf185db966cff6f12`.
It is not a valid D27 baseline; the swap policy remains unsettled.

The local NVIDIA kernel module is 580.173.02 while NVML is 580.178.04.
Per HITL `seq-296`, this mismatch cannot be resolved for Slice 90: proceed
with needed GPU tests on windchill3 as installed and record their outcomes.
The strict `scripts/test-feature-complete.sh` gate exited 2 because its
`nvidia-smi` query exited 18. The official `hidden_surface.py capture` of
`f5bc7ca5d` also exited 2 at the same CUDA preflight. Neither produced a
GPU or hidden-surface PASS. No driver repair is a prerequisite to continuing
available Slice 90 work.

Runtime Batch 2a was merged at `559deb531` from clean implementation commit
`50b485873`. Its RED commit `9294cf73f` reproduced the returned-error
batch fallback deadlock in a bounded child after the provider error marker;
GREEN `703d09385` releases the serialization guard before per-row fallback.
The corrected test requires `UpToDate` and a stored vector for every written
row. The breaker-open fast-failure characterization was added with GREEN,
separately from the primary RED defect witness. An isolated restoration mutant
reintroducing the under-guard fallback fails within the seven-second child
bound and cannot contaminate the candidate Cargo target. Independent
`gpt-6-sol` high code review passed at `50b485873` after correcting a
drain-only test oracle. Independent Terra verification passed the two focused
parent tests, the restoration mutant, PR-9 serialization and watchdog tests,
formatting, and crate clippy at that same commit. This is AC27-90J's
returned-error path; the later embed-dispatch transition must preserve the
no-reacquire invariant.

Rust configuration Batch 2b was merged at `2fff6755a` from clean GREEN
commit `14c708040`, after RED `4b59cc847`. The public `EngineConfig`,
per-engine configuration error, resolved capacities, and configured-open seam
validate the five accepted ranges before open side effects and retain the
requested snapshot separately from effective slow-threshold state. The
`64 × 64` capacity property and four focused integration tests pass. Minimal
exhaustive CLI, PyO3, and NAPI error mappings keep those crates compiling;
the Rust interface documents the new public surface. Independent
`gpt-6-sol` high code review and Terra verification passed at `14c708040`.
The merged release branch reran both Batch 2a and 2b focused integration
suites: 6 parent tests passed, with 2 intentionally ignored bounded-child
entrypoints. The scheduler count, embedder pool size, and deadline remain
validated/stored inputs until later batches make their runtime effects
observable; this batch does not claim AC27-90B complete.

Dynamic projection Batch 2d was merged at `60462c998` from clean GREEN
commit `330721c5c`, after RED `ff0135276` demonstrated that a requested
single worker still started two. The runtime now uses the resolved worker
count for worker-owned connections, startup, inventory, pause/stop and join,
and checked `N × 64` projection-row admission. Focused tests pass at 1, 2,
4 and 64 workers for native/live inventory, at N=1 for actual row saturation,
and for partial startup cleanup and independent engines. The Windows WAL
attribution source guard's dynamic-count assertion and negative mutation pass
317/317 cases. Independent `gpt-6-sol` high code review and Terra verification
passed at `330721c5c`. After merge, the release branch reran both earlier
Slice 90 integration suites (6 parent tests, 2 ignored bounded children) and
all four configured-projection unit tests. The later qualification matrix
still must exercise nondefault consuming/bounds behavior; this batch is not
AC27-90B completion.

Bounded embed-dispatch core Batch 2c was merged at `26b9e9747` from clean
GREEN commit `c1b8a5d58`. Its first RED `de1cb5222` pinned the fixed worker,
bounded queue, deadline, cancellation and accounting outcomes. Additional RED
`3a1654f20` covered late completion, and `442a25356` deterministically
reproduced a stale pre-lock timestamp that could start expired queued work.
GREEN samples time under the reply-state lock. An invalid test expectation
about renewing a spent join deadline was corrected separately at
`794215217`, preserving the one-absolute-budget rule. Independent
`gpt-6-sol` high code review passed after the handoff-race correction, and
Terra verification passed 14/14 focused tests, test-target clippy, library
clippy, formatting and diff hygiene at `c1b8a5d58`; the merged release branch
also passed the 14 tests. The module is deliberately compiled only by the
focused test target at this boundary. It is not yet declared or called by the
production engine; projection, foreground and open-time routing remain for
later batches, so this is no claim of effective engine-owned dispatch.

Projection dispatch Batch 2e was merged at `340b22517` from clean corrected
commit `b9fefcc17`. RED `2ccd40de2` reproduced the old detached-watchdog
behavior: `drain` returned while a provider call still occupied capacity.
GREEN `7f721c8e5` routed projection batch and per-row calls through the fixed
dispatcher, removed detached projection watchdog/circuit state, and adapted
the PR-9 oracles to retained slots, durable pending work and recovery. Sol
review found that a condition-variable wake could shorten provider-failure
retry backoff and that an existing hidden timeout test API had been removed
from default Rust builds. RED `8c159f43b` reproduced both; corrected GREEN
`b9fefcc17` keeps one absolute retry deadline across wakeups, permits prompt
close cancellation, restores the default-build symbol, and removes stale PR-9
comments. Sol re-review and independent Terra verification passed at that
exact commit. On the merged release source, 25 focused parent tests passed,
with two intentionally ignored bounded-child entrypoints, and default engine
`cargo check` passed. Phase 2.7 still owns the truthful incomplete-close
result and shared provider-drain deadline; the current close/reopen witness
proves projection-wait cancellation and durable recovery only.

Next: route foreground and open-time provider calls through dispatch, then
complete two-phase close and binding forwarding on windchill3 while settling
the D27 swap policy.
Collect a valid historical
entry under the ruled protocol before the candidate-bound checkpoint and
structural Phase 3. Report the remaining GPU and Slice 85 recovery evidence
as its actual result. Slice 90 remains IN_PROGRESS.
