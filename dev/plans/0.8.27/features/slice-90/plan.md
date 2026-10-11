---
title: FathomDB 0.8.27 Slice 90 — reconciled execution plan
status: PLANNED
target_release: 0.8.27
---

# Slice 90 reconciled execution plan

This plan approves the bounded runtime and root-closure scope in the accepted
[design](design.md) and [runtime ADR](../../../../adr/ADR-0.8.27-engine-owned-runtime-topology.md),
subject to the entry evidence below. The [code](slice-90-code-change-plan.md)
and [test](slice-90-test-change-plan.md) change plans specify the RED/GREEN
batches. Slice 90 is incomplete until all R27-90A–K and its candidate-bound
runtime checkpoint pass. No publication is part of this slice.

## Changes since the draft was written

The code and test drafts were prepared at `b4e90afe7` and finalized at
`38e375226`. I compared those commits with `release/0.8.27` at `e689000d4`,
the release-state ladder, Slice 85 recovery records, current engine and
binding source, and the allocated Slice 100 pre-entry plan.

1. **Slice 85 source advanced.** Recovery commits through `294af94b5` are
   ancestors of the current release branch, although Slice 85 status described
   them as unlanded. They reduce the module-boundary gate, replace its frozen
   edge census with a bounded explicit-path contract, adjust Engine/read/graph
   test seams, and add callback and filter-error coverage. The Slice 90 owner
   and scanner inventory must start from current source, not historical
   `7a2f9bf9`. Recovery records are corrected for ancestry, but no missing
   qualification is called a PASS.
2. **Release authority did not advance with source.** Release state still binds
   Slice 85 to `7a2f9bf9`; the recovery receipt lacks official public/hidden
   comparison and moved GPU-route evidence. The current host reports 127 GB
   free, enough for the public comparator's 100 GB preflight. The NVIDIA kernel
   module and NVML library mismatch, so GPU-dependent evidence is unavailable
   on the installed host. HITL `seq-296` directs execution of needed GPU tests
   on windchill3 as installed and recording their actual outcomes; it does not
   turn a failed preflight into PASS. Source ancestry cannot replace the missing
   receipts or silently rebind release state.
3. **The D27 historical entry cannot report new executor metrics.** The frozen
   protocol asked the old source for an embed queue and fixed embed-worker
   inventory it does not have. The protocol now compares only shared operation
   throughput and latency; candidate-only queue, deadline, bound, resource and
   cleanup metrics remain mandatory. Historical absence is recorded as
   unavailable, never zero. The historical `7a2f9bf9` entry SHA and the
   median/MAD comparison formula remain unchanged.
4. **Direct-call outcomes needed a decision.** The draft named saturation,
   started failure and close but omitted queued expiration and started timeout
   mappings. The design now maps queue full/queued expiration to `Overloaded`,
   started error/timeout to `Embedder`, and pending close cancellation to
   `Closing`, with request-state completion precedence. RED tests pin each.
5. **Related allocations remain distinct.** Slice 85 owns its recovery
   qualification and boundary policy. Slice 90 owns engine open/configuration,
   two executors, all production inference routes, shutdown, WAL/operator/
   projector/root closure and the installed binding config seam. Slice 100's
   new pre-entry plan remains blocked on Slice 90 and owns PyO3 decomposition;
   Slice 110 owns NAPI decomposition. Slices 114/115/135 may characterize an
   already qualified runtime; Slice 150 owns final AC-037. No draft item is
   promoted into Slice 90 merely because it appears nearby.
6. **Tooling changed.** The release preflight now finds its own directory
   before switching checkouts. It passes `--expect-closed 85` on current
   release state. The checkpoint checker already exists; it must bind the
   exact runtime candidate before structural moves.
7. **Current-host qualification was attempted.** The current reviewed D27
   harness built and ran against exact historical source; its first repetition
   was invalidated by host swap-in and a sleeping host pytest process. Two
   earlier attempts used the old protocol hash and were also swap-invalid.
   The strict feature-complete gate and
   official hidden-surface capture both exited 2 at the installed NVIDIA
   driver/NVML mismatch. HITL `seq-296` removes driver repair as a prerequisite
   to doing available work; these observations remain failed or unavailable
   evidence, not qualification receipts. The D27 swap policy needs a separate
   explicit disposition before its historical entry can qualify.
8. **HITL directed current-host continuation.** The reviewed harness executed
   on the exact historical source; its environment-invalid attempt establishes
   no baseline. Phase 2 RED/GREEN runtime implementation may proceed
   on windchill3 as installed while the swap rule is settled. No invalid
   attempt becomes a PASS receipt, and the runtime checkpoint and structural
   Phase 3 remain gated by qualifying D27 evidence.
9. **Installed binding evidence exposed a proof allocation gap.** The Python
   candidate forwards all five settings and its isolated installed wheel passes
   30 cases, but the draft asked that installed Python directly observe custom
   provider concurrency/timeout and lifecycle slow events. Existing public
   Python and Node opens accept no caller provider, and their subscriber
   adapters deliver no lifecycle events. The reviewed proof allocation now
   requires deterministic consuming-effect witnesses at the Rust owner and
   installed native forwarding plus all effects the bindings actually expose.
   It does not create a test-only public binding adapter for duplicate evidence.
10. **HITL settled D27 swap and updated GPU policy.** `seq-297` authorizes one
    reviewed bounded-swap rule applied identically to historical and candidate
    entire child repetition. The design caps combined movement at 128 pages, checks every
    sample for counter validity, and retains raw-linked deltas; earlier
    invalid attempts are not reused. `seq-298` supersedes the prior GPU-driver
    ruling: the newly installed version is authorized, but the loaded kernel
    module and NVML differed at the pre-reboot unsandboxed check. A successful
    GPU gate is required before claiming that route passed.
11. **The host reboot cleared GPU preflight.** On 2026-10-01 after reboot,
    `nvidia-smi`, the loaded kernel module, and the installed module all report
    driver 580.178.04. The strict GPU feature gate can now execute, but only
    its completed result and the official hidden-surface capture can qualify
    the route. The historical D27 entry separately passed the reviewed
    bounded-swap protocol in six repetitions, but the reboot cleared its
    `/tmp` raw bundle. Rerun the entry into persistent storage before the
    candidate comparison.
12. **Host `/proc` link access required a narrow qualification correction.**
    An unsandboxed host run still received `EACCES` on `/proc/1/ns/pid` after
    reboot, although the host PID namespace, systemd PID 1, `hidepid=0` and
    runner PID corroboration were available. The independently reviewed
    protocol accepts only `EACCES`/`EPERM` on that link as a blank sentinel;
    other link failures and unreadable PID 1 identity remain invalid. The
    test-first runner correction passed independent code review and Terra
    verification. A new six-repetition historical entry on exact `7a2f9bf9`
    passed under the new shared protocol hash; its complete raw bundle and
    byte-for-byte reverified receipt are in persistent Slice 90 evidence.
13. **Binding and GPU implementation evidence advanced.** Two-phase close and
    Python/Node five-setting forwarding have merged after independent reviews.
    Installed Python 3.12 and Node 25 consumers passed; an off-PATH system
    Python 3.10.20 consumed the installed `cp310` abi3 wheel and opened/closed
    a real engine. The first strict post-reboot GPU gate exercised its CUDA
    routes with 350 passes and zero test failures, but failed its skip
    allowlist after a PR-9 test rename. The stale name was corrected; only a
    completed final strict run and official surface captures can qualify GPU.
    The remaining runtime docs still contain stale executor and error claims;
    update them against the accepted ADR and actual implementation before the
    runtime checkpoint.
14. **The candidate workload and owner effects need explicit witnesses.** The
    historical D27 bundle is valid, but the current workload's default-only
    open omits the candidate configuration, projection-admission and dispatch
    records required by its strict verifier. The reviewed design adds only a
    test-hooks engine observation seam and keeps the frozen corpus, operation
    mix, timing and comparison rule. Open-time provenance-cap and slow-event
    effects also need direct owner tests; a parked-provider close test retains
    the pre-2g expected result. The six configuration cells and checkpoint
    receipt-content guard remain required before structural Phase 3.

## Evaluation and scope decision

Keep the accepted five-setting behavior, engine-owned bounded executors,
universal production inference dispatch, operation-specific outcomes, safe
shutdown and complete named root ownership. These are approved by the runtime
ADR and cannot be reduced to forwarding or a partial extraction. Keep the
stage-2 checkpoint before structural moves, with exact candidate review and
verification. Amend the draft's stale source baseline, impossible historical
metrics and incomplete direct-call mapping as above. Apply the recovered gate
as it exists; do not recreate its retired edge census. Use focused affected
tests and boundary/surface checks per move; reserve expensive official capture,
installed artifacts and full qualification for the checkpoint and final
candidate. The suggested batch sizes are review aids, not a fixed commit count.

## Requirements and acceptance

The design's R27-90A–K and AC27-90A–K remain the complete acceptance set.
The corrections make these parts falsifiable:

- **Entry and ownership (A, C, E, F, H):** enumerate every current release
  root item, field, method, cfg arm and named handoff; assign its reviewed owner
  or exact retained-root reason. Record the inherited recovery delta and run
  the reduced boundary gate on each relevant move. A historical inventory or
  an unclassified residual fails.
- **Runtime (B, D, J):** all five settings have effective Rust and installed
  Python/Node behavior; production embed calls use the engine dispatcher;
  queue full/expiry, started failure/timeout, panic and close yield their
  specified outcomes. RED tests precede each change, including the batch
  fallback deadlock and restoration mutant. No database mock stands in for
  projection, read, WAL or shutdown tests.
- **Qualification (G, I, K):** the historical performance entry records only
  comparable metrics; the corrected candidate supplies all new dispatch and
  resource evidence. The exact stage-2 checkpoint has PASS performance, Sol
  code review and independent verification receipts before the first
  structural commit. Final candidate repeats affected default performance,
  installed-surface and resource checks. Recovery qualification gaps remain
  visible until measured; an unavailable GPU or official capture cannot be
  recorded as PASS.

## Execution and reviews

1. Freeze current-source owner/scanner inventories and historical D27 entry
   evidence in separate worktrees. Reconcile the inherited Slice 85 recovery
   evidence. Review and execute the measurement-only runner on exact historical
   source before any semantic edit; preserve invalid attempts as diagnostics.
2. Follow code-plan batches 2a–2l. For each behavior change, stage a failing
   test, record RED, implement the narrow GREEN fix, then run affected tests
   and lint. Keep the test oracle fixed during its corresponding fix. Establish
   the exact runtime checkpoint and obtain independent `gpt-6-sol` high code
   review plus independent verification before moving bodies.
3. Follow the design's owner map through structural batches. Characterize
   behavior before moving it, preserve public re-exports and cfg identities,
   check affected scanner/boundary routes, and compare to the post-correction
   surface. Review any newly discovered unallocated item before moving it.
4. At the final candidate, run the required affected feature/platform matrix,
   installed binding checks, default D27 and release performance checks,
   repository verification, independent `gpt-6-sol` high code review and
   Terra verification. Record exact candidate, commands, outcomes and limits
   in `status.md`; only then advance release state, merge into
   `release/0.8.27`, and remove a task-created worktree and branch.

The first independent `gpt-6.1-sol` high design review found the three gaps
corrected above. Its verdict was FAIL on the draft. A second independent
`gpt-6-sol` high review must assess this amended design and plan before
implementation; findings receive focused corrections and re-review.
