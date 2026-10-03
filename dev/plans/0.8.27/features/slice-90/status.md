---
title: FathomDB 0.8.27 Slice 90 — implementation status
status: COMPLETE
target_release: 0.8.27
---

# Slice 90 implementation status

Requirements, acceptance criteria, design and TDD batches are reconciled in
the [plan](plan.md). Independent design review passed after the recorded
corrections. Runtime Batches 2a–2f, two-phase close, and Python/Node forwarding
are merged and independently reviewed. The stage-2 runtime checkpoint and
final Slice 90 verification are PASS; the final code candidate is bound at
`1398c821dd26b7945bb2f6fbfa02b68cd4daa8af`.

The frozen operational source entry was `release/0.8.27` at `e689000d4` or a
documentation-only descendant before Phase 2 semantic edits. Historical `7a2f9bf9`
remains the D27 performance reference. Slice 85 recovery code is already in
release ancestry. Its historical-candidate public/hidden comparison has now
been measured, while release state still binds the earlier Slice 85 candidate.
The host reboot
loaded the authorized driver; the first strict GPU feature gate reached CUDA
tests but did not pass because of a stale allowlist name. A later corrected
gate and current-source captures are recorded below.

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
It is not a valid D27 baseline. HITL later ruled a reviewed bounded-swap
protocol at `seq-297`; this old attempt remains invalid under its own hash.

The reviewed bounded-swap D27 harness correction was merged at `27e0089e7`
from clean GREEN `70bcfa426` after test-only RED `8562afb1e`. The current
protocol SHA-256 is `489a5a765615bbda82f7e3b585a868e481121def0fb38fa4e9bd416632f5efb0`.
It permits at most 128 combined host swap pages across each entire workload
child, rejects missing/negative/reset counters at every sample, and binds both
deltas to raw per-repetition metrics. The global six-repetition zero-swap
check was removed. Independent `gpt-6-sol` high code review and Terra
verification passed at the exact GREEN SHA; the merged release source passed
38 focused D27 tests. The prior invalid attempts retain their old hashes.

A fresh six-repetition historical entry on windchill3 against unchanged source
`7a2f9bf90783f545603516502bac0016d4b93a14` passed the reviewed protocol.
The receipt is `/tmp/fathomdb-s90-d27-bounded-entry-20261001/receipt.json`
(SHA-256 `43c3c3480e1d6e83ee2a4228446436ca3fff41d2601ff7540a955525a103fdbd`).
It binds protocol SHA-256 `489a5a765615bbda82f7e3b585a868e481121def0fb38fa4e9bd416632f5efb0`,
runner bundle `d0e8765ad7afad80e27a506156f16be4707b7ed5ebe045641ec5284867f46e90`,
historical binary `b658501f6020c22d92c3e53ec91d888bb7383720929f4adc6e7635d5f0518361`,
corpus `a4eeb2a7c714d0d450e7ecd41bf7bb45a83f97c30d3adc513629eeca836a685d`,
and raw output `32966c5c5154c980f6cda1b727a978066b77a34d726e4de227fbf7cc7d66442f`.
Every repetition passed environment validation; the foreground-heavy swap
in/out deltas were (1, 0), (9, 0), (0, 0) pages and projection-heavy deltas
were (0, 0), (6, 0), (2, 0). The standalone verifier reproduced the receipt
byte for byte. The subsequent host reboot cleared `/tmp`, including that
receipt and its raw artifacts. The recorded hashes preserve the audit trail,
but the lost bundle is not used for the candidate comparison. A fresh
historical run and retained bundle are recorded below.

The local NVIDIA kernel module is 580.173.02 while NVML is 580.178.04.
Per HITL `seq-296`, this mismatch cannot be resolved for Slice 90: proceed
with needed GPU tests on windchill3 as installed and record their outcomes.
The strict `scripts/test-feature-complete.sh` gate exited 2 because its
`nvidia-smi` query exited 18. The official `hidden_surface.py capture` of
`f5bc7ca5d` also exited 2 at the same CUDA preflight. Neither produced a
GPU or hidden-surface PASS. That ruling was superseded at `seq-298`: the newly
installed driver version is authorized, but the unsandboxed 2026-10-01 check
still found loaded kernel module 580.173.02, installed module 580.178.04 and
NVML 580.178. `nvidia-smi` exited 18. Desktop processes currently hold the GPU
devices, so a module reload was not safe during that session. After the host
reboot, `nvidia-smi`, `/proc/driver/nvidia/version`, and `modinfo` all report
580.178.04. The GPU inventory includes two RTX 3090 cards and one K620. This
cleared the driver/NVML preflight blocker; at that point the strict
feature-complete and official hidden-surface gates still needed successful
results before GPU PASS.
HITL `seq-300` clarifies the final-candidate rule: use and test whichever
NVIDIA driver version is installed on windchill3 at the time of that run,
record its observed version, and continue the remaining work without a
fixed-version prerequisite. The version above is a historical observation.

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

Foreground/open-time dispatch Batch 2f was merged at `5d93cd556` from clean
corrected commit `e933dd4a7`. Test-only RED `e9ab011b2` reproduced an
unbounded direct-call deadline and blocked vector-equivalence probe; GREEN
`9a4e14f94` routed direct, ordinary/frozen search, and open-time provider calls
through the same engine dispatcher. Independent Sol review found that two
search boundaries converted provider panics into sparse fallback. Corrective
RED `4a2469368` pinned the ordinary caller unwind and frozen reader-owner
`Storage` boundary; GREEN `e933dd4a7` preserves those panic outcomes while
nonpanic dispatch failures retain sparse fallback. Narrow Sol re-review and
Terra verification passed at the clean final commit, including 9/9 foreground
tests in default and test-hooks builds. The focused full candidate also passed
43/43 targeted tests, selected-feature check, Clippy, formatting, and diff
checks. Two-phase close remained Batch 2g at this boundary; this merge did
not claim the runtime checkpoint.

Two-phase close was merged at `f8af83635` after independent Sol code review
and Terra verification. On the merged source, eight focused `slice90_` library
tests and 14 embed-dispatch tests passed. Python forwarding was merged at
`93db9fd67` after independent review and installed wheel verification. The
wheel passed the exact native five-setting oracle and 31 Python 3.12 consumer
cases. The system's off-PATH Python 3.10.20 also parsed the installed `cp310`
abi3 wheel's config and opened and closed a real engine. Its receipt is under
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/python-wheel-ae7a47abb`.
Node forwarding was merged at `5b3aa844b` after independent review; an
installed Node 25 production package and witness package passed under
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/node-6e88d84aa6-node25`.
Neither installed binding exposes a caller provider and lifecycle subscriber,
so deterministic provider/slow-event consuming effects remain Rust-owner
qualification, with native forwarding checked in the installed artifacts.

The first post-reboot strict GPU feature gate ran 21 sets: 350 passed, zero
failed, and eight were ignored out of 358 planned. It exited 1 only because
the PR-9 opt-in test was renamed while the skip allowlist retained its former
name. The corrected allowlist landed at `72037f7f2`; a focused plan audit and
static coverage check passed. The retained first-run summary is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/gpu-feature-gate-first/summary.json`
(SHA-256 `381bd2d6acb7415dc48ff2e5b1ba1b74b5e57a131c99ccaf834f8787d139cfce`).
The final strict GPU gate and official public/hidden surface captures remain
pending; the first run is not a GPU PASS receipt.

On windchill3, the host denied the read-only `/proc/1/ns/pid` link with
`EACCES` even outside the sandbox. The reviewed protocol amendment at
`325b024b3` permits only `EACCES`/`EPERM` on that link to yield a blank
sentinel while retaining the host namespace, systemd PID 1, unrestricted
`/proc`, runner PID corroboration, process competitor, and swap checks. A
test-first fix, RED `d8751b16d` and GREEN `327078a67`, passed independent
Sol code review, Terra verification, and 40 merged focused tests; it was
merged at `204fa402c`. Real-host preflight showed `pid:[4026531836]`,
systemd PID 1, `hidepid=0`, matching runner PIDs and no competitors.

The fresh six-repetition historical entry on unchanged
`7a2f9bf90783f545603516502bac0016d4b93a14` is PASS under protocol
SHA-256 `b835f79a426a038e3528a9cf820177876451a02c3defad7aa0c545ebf642441b`.
Its persistent receipt is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-entry-post-reboot/receipt.json`
(SHA-256 `74c5ee0b43188dd2f2af138eb84dd67aa281034017693e095c82327c6dadc129`).
It binds runner bundle `812c03254b5e18b6fbf2a35f6d1a968a6fc26cda7dc4db40d5c08c3c2436b6ca`,
binary `b658501f6020c22d92c3e53ec91d888bb7383720929f4adc6e7635d5f0518361`,
corpus `a4eeb2a7c714d0d450e7ecd41bf7bb45a83f97c30d3adc513629eeca836a685d`,
and raw output `e8b9eb210f01400f6bd887c5ee35fd4934924bdaedcbcdf22ab341ba1b060f39`.
The standalone verifier reproduced the receipt byte for byte. The previous
`/tmp` result remains historical audit only. The D27 implementation worktree
and branch were removed after merge.

Runtime contract documentation and its narrow clarification were merged at
`bb72eee8e` after independent design review and Markdown verification. The
NAPI test-hook cfg correction was merged at `80770ab68` after independent code
review and Terra verification. The module-boundary policy now classifies 78
modules, including the new runtime owners; its correction was merged at
`2fdd64c3d` after independent review and 34 gate self-tests.

The merged source passed the lint and typecheck stages of `agent-verify`. The
unchanged strict security gate passed outside the sandbox, including the
ptrace-dependent AC-036 and live AC-037 paths. A full host `agent-test.sh
--tier=all` run exercised all 130 registered suites: 128 passed, and Rust and
Python failed. Rust's only two failures were stale close expectations that
required successful close before a held provider returned. Python's 21
failures came from missing declared `networkx`/`scipy` test packages and a
subprocess import path; after local test-environment repair, all 36 focused
Python cases covering those failures passed. These findings do not constitute
a full-suite PASS.

The owner-level provenance, slow-threshold, and close qualification tests were
merged at `db6cbaac4` after corrected independent Sol review and Terra
verification. A measured 1–99 ms SQL statement distinguishes explicit zero
from the 100 ms default on both slow-signal channels; a held projection
provider test proves database admission is released before the provider
returns, while close remains pending. The 21 focused tests passed again on the
merged release source. These are later qualification assertions for runtime
behavior whose original implementation has separate RED/GREEN history; the
zero/default test's temporary mutation was observed by the implementer but
does not have a retained executed-mutant transcript.

The reviewed D27 test-hooks observation seam, exact live SQLite role checks,
five nondefault matrix cells, and strengthened checkpoint verifier are merged
through `2d3979286`. The final one-line checkpoint fixture integration fix was
independently reviewed by Sol and verified by Terra, then merged at
`0ac1efb24`; 24 focused checkpoint tests and the current PENDING-state gate
pass. The matrix and checkpoint task worktrees and branches are removed. The
D27 candidate worktree remains clean for performance remediation.

The strict six-repetition D27 candidate comparison against the retained
historical entry (receipt SHA-256
`74c5ee0b43188dd2f2af138eb84dd67aa281034017693e095c82327c6dadc129`)
ran at `2d3979286` with the frozen protocol and corpus. All six repetitions
were environment-valid, but the comparison failed latency ceilings; the raw
bundle is under
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-candidate-stage2-retry-2d3979286`.
Provider service p50 stayed about 2.06 ms while the default single shared
worker introduced 4–6 ms direct-embed queue wait at p50 and about 8.2 ms at
p95. Projection-heavy freshness p50 was 21.52 ms against a 19.75 ms ceiling;
foreground-heavy direct-embed p95 was about 10.31 ms against 2.38 ms. No D27
PASS receipt exists. An earlier six-run attempt was invalidated by a competing
test process and is retained separately.

The unconfined full `agent-verify` on `2d3979286` passed lint, typecheck,
strict security and 129 of 130 test suites; only the checkpoint fixture's old
SQLite inventory failed after the D27 hook merge. That fixture is corrected at
`0ac1efb24` and its focused tests pass; no new full-suite PASS is claimed.
At `0ac1efb24`, unchanged AC-011a/b, AC-017, AC-018, AC-029 and AC-081c
selectors pass. The first AC-081a/b seven-run campaign stopped on its first
real selector because synchronized vector-only searches returned empty after
bounded embed admission saturated. A committed eight-reader RED regression at
`e51268ab1` reproduces this in an isolated worktree. The accepted ADR's
one-worker/four-waiting-slot nonblocking contract permits at most five of the
eight simultaneous requests, so scheduling alone cannot green that selector.
No runtime semantic change or ADR successor is merged; the capacity ruling is
pending.

Current-source installed Python wheel checks pass 31 Python 3.12 cases and a
real Python 3.10.20 open/close. Node 25 production and witness installed
packages both pass. Their retained evidence is under
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/python-wheel-0ac1efb`
and
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/node-0ac1efb`.
The corrected strict GPU gate passed 351 tests, 8
planned ignores, 0 failures in 21 runs (summary SHA-256
`f50ff66032f2ac7fdcf56dc4be1393bcbca527fb228a7199565d169ac9b8ebf3`).
Official current-source public and hidden captures passed 13 and 33 rows;
their SHA-256 digests are
`e414b2c3a99c855b10b52149155476a3fbbd8619dcda84f4913b0c3860e26e48`
and `62cc86e14193cd2f41d7bef02ff67fe507e5eda629cb1073cd45624556b4f55f`.
These are stage-2 current-source evidence, not final post-extraction receipts.

HITL `seq-299` authorized testing higher default embed-worker counts and
selecting the lowest count that passes the unchanged frozen D27 comparison,
AC-081, and named release performance gates. The `4 * N` embed waiting bound,
frozen protocol, corpus, workload, and comparison remain unchanged. The
test-first two-worker candidate at `781b2f5e0` completed six valid D27
repetitions and failed projection-heavy direct-embed p50; its direct-embed p95
median was about 6.18 ms in both directions. The three-worker run at
`e0c3a1b16` had one valid repetition with direct-embed p95 4.14 ms, then was
invalidated by 1,617 swap pages in its next repetition. It is diagnostic,
not a six-run verdict. A fresh three-worker proof on `2a62b14c7` completed
six valid repetitions and failed projection-heavy direct-embed p95; the p95
medians were 4.139 ms under projection-heavy load and 4.141 ms under
foreground-heavy load. Its raw bundle is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-default3-proof-2a62b14c7`.
The four-worker candidate at `c03a398f8` completed
six valid repetitions and failed projection-heavy direct-embed p95, with
about 4.12 ms p95 in both directions.

The five-worker candidate at `ad31c3a61` passed the frozen six-repetition
D27 comparison against the retained historical entry after one invalid swap
attempt. Its valid receipt is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-default5-retry-ad31c3a61/receipt.json`
(SHA-256 `29e5206e02751ac83d2a28fa5cf9b85bf98e164cc3270c0cdf88cef5a5b4d185`);
direct-embed p95 medians were 2.116 ms under projection-heavy load and 2.120
ms under foreground-heavy load. The same exact source passed the unchanged
AC-081a/b seven-run campaign and AC-081c. The official campaign summary is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/ac081-default5-ad31c3a61/summary.txt`
(SHA-256 `00a4313dc52ae5cb9c1939e6c220901581d530adad691467c63ce09c8d1084ab`).
At that stage this was a provisional default selection until every named
selector passed. The ADR successor
`dev/adr/ADR-0.8.27-embed-dispatch-default-capacity.md` was not yet accepted,
and the runtime checkpoint remained PENDING.

On exact clean source `ad31c3a61`, the unchanged named selectors passed
AC-011a at 1,263.671 commits/s against 1,000, AC-011b at 306.721 against
100, AC-017, AC-018 (22 ms drain), AC-029, the official three-run AC-072
campaign (p50 71 ms, p99 78/78/79 ms), and AC-076 (p50 1 ms, p99 3 ms).
The CUDA EU7 run on 7,667 real documents measured AC-073 mixed-tail stress
p99 418 ms within its same-run 491 ms bound. The unchanged combined selector
exited 101 because AC-075 vector-stage recall was 0.772 with CI high 0.798
against its old 0.90 floor, exactly matching the earlier Slice 85 receipt
that marks AC-075 `superseded-by-tc5`. Do not report the combined selector as
PASS. Raw evidence and an exact verdict report are under
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/named-selectors-default5-ad31c3a61`;
the report SHA-256 is
`1c83338b90dcbf6c870f108f665bf704cb462970a68ec84da18a9a253f612230`.
The checkpoint receipt needs a reviewed, test-first way to bind the AC-073
stress PASS and retain the unrelated superseded AC-075 failure without a
false combined-selector PASS claim.

The repository owner then directed a relaxation of the host-wide D27 swap
cap because other host activity cannot be contained. The original v1 protocol
and six-run historical entry remain byte-identical. The test-first v2
successor at `2e94aaf4f` makes swap movement report-only while still requiring
present, nonnegative, monotonic counters and raw-linked deltas. Its semantic
diff is confined to the swap rule. The exact historical v1 entry and raw
artifacts revalidate under the original stricter rule before a v2 candidate
comparison; no failed or incomplete historical rebuild was promoted. An
independent `gpt-6-sol` high review passed this proof bridge.

Two six-repetition v2 campaigns on clean source `2e94aaf4f` were fully
environment-valid and conflict under the unchanged median/MAD performance
rule. The first failed projection-heavy canonical-commit p95 at 7.1435 ms
against a 5.5604 ms ceiling; its projection-heavy repetitions moved 8, 52,
and 143 host swap pages. Its raw bundle is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-v2-candidate-2e94aaf4f`.
The single retry passed with projection-heavy canonical-commit p95 median
5.0637 ms and direct-embed p95 median 2.1161 ms; its projection-heavy
repetitions moved 382, 10, and 39 host swap pages. The retry receipt is
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-v2-candidate-2e94aaf4f-retry/receipt.json`
(SHA-256 `52a20f29cb3bbb19a0ec159dace4a3cfbd74307c2dd3234cf59dbbfc820ad9b1`).
Neither result erases the other. The third complete, environment-valid v2
campaign on the same source used CPU affinity `2-11,14-23`; 19 sampled workload
threads carried that mask. The mask constrained the process and its children,
but did not exclude other host activity from those cores; the historical
entry used all 24 logical CPUs. The third campaign's projection-heavy
canonical-commit p95 median passed at 4.8958 ms against 5.5604 ms. Its
canonical throughput median was 33.1802 commits/s against the unchanged
33.5926 commits/s floor, a 0.4125 commits/s (1.23%) shortfall. The strict
verifier rejected it on throughput and produced no PASS receipt. On
2026-10-02, the repository owner explicitly accepted that absolute shortfall
as PASS for the release decision. The raw third campaign and exact rejected
outcome remain at
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/d27-v2-candidate-2e94aaf4f-third-affinity`.
The strict passing retry remains the machine-valid D27 checkpoint receipt;
the earlier valid latency FAIL remains in the record. The candidate-bound
receipts are bound at `4a98a1e80`; the structured runtime checkpoint is PASS.

Published crates.io `fathomdb-engine` 0.8.26 then completed an equivalent
six-repetition workload with the same corpus, deterministic provider,
warm-up, measurement, operation mix and 20-logical-CPU affinity mask as the
third candidate campaign. Its projection-heavy median was 36.6958 canonical
commits/s and 4.5476 ms canonical-commit p95, versus the third candidate's
33.1802 commits/s and 4.8958 ms: absolute differences of 3.5156 commits/s
and 0.3482 ms. It lacks the new D27 dispatch and exact SQLite-role telemetry,
so this is comparable diagnostic evidence, not a strict D27 receipt. Published
PyPI and npm 0.8.26 artifacts were downloaded and passed isolated open/close
smokes. The registry package hashes, six raw runs and comparability limits
are indexed at
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/registry-0.8.26-equivalent/registry-artifacts.json`
(SHA-256 `23464d1e2d36c1a412d0732a51c3c1115bf6d460094b790c889d385fdd904d0f`).

The exact `2e94aaf4f` source passed the full unconfined `agent-verify` gate:
lint, typecheck, strict security with zero violations/blockers/downgrades,
and 130/130 registered test suites. Its default `2/5`, explicit `1/1`,
`2/2`, `4/4`, `64/64`, and no-provider real-engine matrix passed, as did the
4,096-pair checked-capacity property. The explicit `2/1` test was added
test-first on an isolated branch; it passed on the merged tree with exact
managed SQLite roles, provider use, close/reopen and cleanup. Independent
`gpt-6-sol` high review passed that correction. The merged engine source
remains byte-identical to the measured `2e94aaf4f` engine source.

On clean `2e94aaf4f`, AC-011a/b passed at 1,239.384 and 297.223 commits/s;
AC-017, AC-018 (64 ms drain), AC-029, official three-run AC-072 (p50
70/69/70 ms, p99 76/77/76 ms), AC-076 (p50 1 ms, p99 2 ms), official seven-run
AC-081a/b and AC-081c all passed. The installed Python wheel passed 31/31
isolated Python 3.12 cases and a real open/close on off-PATH Python 3.10.20.
Installed Node 25 production and witness consumers passed. Exact logs and
hashes are under
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/stage2-gates-2e94aaf4f`,
`python-wheel-2e94aaf4f`, and `node-installed-2e94aaf4f`.

The sealed exact-source CUDA EU7 run measured AC-073 mixed-tail stress p99
493 ms within its same-run 513 ms bound on 7,667 real documents, with no
padding. The unchanged combined selector exited 101 solely on AC-075
vector-stage recall 0.773 (CI high 0.799 below 0.90), retained as
`superseded-by-tc5`; it is not reported as a combined PASS. The exact run
log, EU7 JSON, execution manifest and stress receipt are committed here, and
the tightened checkpoint validator accepts their source, binary, command,
output path and byte bindings. Initial missing CUDA toolkit path setup
stopped before test execution; its failed build logs are retained externally.

Independent Sol high review found and test-first corrections closed two
receipt proof gaps: D27 v2 now binds the entire runner/workload/verifier
bundle to exact Git source, and AC-073 binds the logged output to the sealed
bundle file and retained JSON. The reviewed fix passed 48 focused Python
tests; the historical v1 entry and candidate raw data remain unchanged.
The final stage-2 Sol high review passed at merged tree `9b04d0b25` with no
remaining material code or proof finding. Independent Terra read-only
verification at that tree passed 48 focused D27/checkpoint tests, validated
the sealed AC-073 receipt, revalidated historical v1 and candidate v2 D27
data, and checked 25 stage-2 log hashes. The later merged explicit `2/1`
matrix run passed; its hash made the manifest 26/26, independently rechecked
without mismatch. Terra did not rerun the host-heavy gates. The full
unconfined merged-tree `agent-verify` independently passed lint, typecheck,
strict security and all 130 registered test suites with none skipped or
excluded. Candidate-bound stage-2 review, verification and performance
receipts are recorded. The performance receipt binds the exact strict passing
D27 retry and records the third-run HITL ruling with the unchanged verifier
miss. The structured runtime checkpoint binds exact candidate `2e94aaf4f`,
receipt commit `4a98a1e80`, and three PASS receipt hashes in release state;
`scripts/check-runtime-checkpoints.py` passes. Structural Phase 3 is landed on
`release/0.8.27` through `1088df022`. The reviewed owner moves preserve the
public paths and approved feature gates. The D27 dispatcher protocol now lives
in its standalone Engine-free core, and its three test-only observation methods
live in the ordinary `embed_dispatch` owner. The blob-bound
`phase3-root-reconciliation.md` records the 43 retained storage fields, four
approved root test helpers, the `#[cfg(test)]` worker fixture, and zero
unmapped root production declarations. The independent Sol high review passed
on the exact implementation tree, and the merged release branch passed the
full unconfined verification gate at `1088df022`: 180/180 suites, zero skips
or exclusions, and zero security violations, blockers, or downgrades.

## Final code-candidate qualification on 2026-10-03

The clean final code candidate is
`1398c821dd26b7945bb2f6fbfa02b68cd4daa8af` on `release/0.8.27`.
The last two commits preserve the stage-2 public D27 observation type paths
and close a negative boundary-check gap found by independent review. The new
mutant fails before the guard and is rejected after it; the 104-configuration
module-boundary gate passes. Independent `gpt-6-sol` high re-review passed the
correction and final `gpt-6-sol` high code review passed the entire exact tree
with no actionable finding. The final reviewer separately passed 226 owner
guard tests. No further engine source edit followed that review.

| Candidate-bound gate | Result |
| --- | --- |
| Full unconfined `scripts/agent-verify.sh` | **PASS**: 180/180 suites, zero skipped or excluded; strict security has zero violations, blockers and downgrades. |
| Runtime resources and cleanup | **PASS**: all four `slice90_runtime_matrix` tests, including default `2/5`, explicit `2/1`, configured matrix and no-provider reopen. |
| D27 default workload | **Strict PASS**: six repetitions under the reviewed v2 protocol against frozen historical entry; receipt SHA-256 `8e6ecba12087c41e76a242ec7578531ffbefa5c29aa7bb5067b7d02fcbaaaaf8`. The stage-2 third-campaign HITL exception was not reused. |
| Named release selectors | **PASS**: AC-011a/b, AC-017, AC-018, AC-029, AC-076 and AC-081c. The sealed AC-072 campaign passed three cells (p50 70/70/70 ms, p99 78/77/77 ms). AC-081a/b passed seven fresh processes with sequential median 184.075 ms, concurrent median 65.339 ms and no warnings. |
| AC-073 real corpus | **Stress PASS**: 7,667 real documents, no padding, mixed-tail p99 437 ms within the same-run 488 ms bound. The combined selector exited 101 solely on the separately superseded AC-075 vector-stage recall floor: 0.773, CI high 0.799 below 0.90. The failure is retained, not rewritten as a combined PASS. |
| Installed bindings | **PASS**: exact wheel, PyO3 oracle and 31/31 isolated Python 3.12 cases; Node 25 installed production and witness consumers both passed. |
| Linux and Windows features | **PASS**: nine Linux all-target feature routes, combined CUDA all-target typecheck, and three Windows 11 MSVC all-target routes from a guest hash-verified exact Git archive. |
| Strict GPU gate | **PASS**: 21 feature runs, 353 tests passed, eight planned ignores, zero failures. At execution windchill3 reported driver 580.178.04 on both RTX 3090s; HITL `seq-300` permits whatever driver is installed at run time. |
| Official surfaces | **PASS**: 13 public rows exactly equal to the stage-2 candidate with no metadata or row diff; 33 hidden rows have only additive test targets/inventory (14 rows), zero removals or changes, and no structural rustdoc or probe difference. |
| Slice 85 recovery reconciliation | Exact pre-move recaptures match both recorded hashes. Official recovery diffs show only the documented hidden hook replacement and additive tests; exact recovery source `294af94b5` separately passed 21 GPU feature runs, 349 tests, eight planned ignores, zero failures. Historical Slice 85 authority remains bound to `7a2f9bf9`; no rebind is inferred. |

The candidate-bound logs and SHA-256 inventory are preserved at
`/home/coreyt/projects/fathomdb-worktrees/qualification-evidence/slice-90/final-1398c821d/final-evidence-manifest.json`.
The exact Slice 85 baseline/recovery diff is preserved in the adjacent
`slice85-recovery-comparison.json`. The full repository gate, surfaces, D27,
installed bindings, GPU, Windows, resource matrix, and named performance
selectors all passed without a post-review source edit.

R27-90A–K have implementation and candidate-bound evidence above and in the
reviewed owner inventory. Independent `gpt-5.6-terra` high read-only
verification returned **PASS** for the exact final code candidate with no
actionable finding; see [final-review-verification.md](final-review-verification.md).
The documentation successor `65f5e004d` has the same source tree. The
remaining AC27-90I gate is therefore closed. Slice 90 is **COMPLETE_ON_RELEASE_BRANCH**;
Slice 100 is the next planned slice and requires separate commissioning.
