---
title: Slice 110 — CUDA context-loss lifetime and safe teardown
status: CURRENT_PIN_POLICY_IMPLEMENTED_TRACKED_INTEGRATION_PENDING
target_release: 0.8.29
slice: 110
---

# Slice 110 — CUDA context-loss lifetime and safe teardown

Implement the selected context-loss source/seam repairs. TC-281155a1 remains
unqualified until real original-platform reset→close/drop/normal-exit evidence;
detection or seam tests alone cannot close the defect.
This replaces the proposal-only plan after inspecting accepted source
`a00bf9b9228318e7c236148551bb188f63ff66ee` on November14. Independent Astra-medium design review passed. Owner seq-376 selects the
quiescent completed-construction/frozen-native-live-set policy; seq-377 holds
actual Orin/3090 child reset execution for .30 review. No live fault is authorized. [Requirements](requirements.md),
[design](design.md) and [test map](test-map.md) are the complete slice packet.

## Scope and predecessors

- [Master](../../../plan-0.8.29.md) and [release state](../../../release-state-0.8.29.json)
  allocate110 under R29-REL-R/G, R29-3P-17/D29-RESET and O3-17.10 is closed;
  integrated Linux20/70 provide lifecycle/error behavior;60 local acceptance is
  complete and100 remains a qualified subset. G's retained source stays pinned;
  current-pin110 CPU policy work is implemented without changing G's subject.
- Preserve format37, no migration, provider identity, queue deadlines, allocator
  choice, healthy pool release and ordinary OOM retry. Physical erasure remains
  complete except concrete defects; no new erasure SLA.
- [.28 design](../../../0.8.28/features/slice-30/design.md) and
  [status](../../../0.8.28/features/slice-30/status.md) establish typed loss then
  SIGSEGV in cuBLAS destruction on Orin. The current characterization asserts it.
  [Accepted pool ADR](../../../../adr/ADR-0.8.28-tegra-private-cuda-pool.md)
  still says reset unsupported. Preserve that promise until qualified proof.
- Cudarc owns the unsafe nested destructor seams. Select only the concrete
  [140-A repair](../slice-140/design.md) below; .30 rebase, marker cleanup,
  multiarchitecture and unrelated performance work do not gate110.
- New CUDA13 artifact/compiler work formerly in120 is .31 (seq-378);130 owns affected final platform and SDK pool/soak
  qualification. Reviewed110 obligation → initial140 proof → qualified pins
  →110 integration → affected130 qualification. Completed110 never gates140.

## Execution sequence

The current-pin publication/build/snapshot/release subset is implemented and
accepted at `8247f11056bc2deb3e0c6ec0472abbd4b217e857`; its98 CPU tests are
preserved proof, not a claim of tracked/native integration. Selected cudarc
tracked-pool/library source is CPU-qualified at
`cb36642fad90c1796693c36ee59e3d3d162f4410` with312 selected passes and independent
Astra acceptance. Prepare110's API/model wiring, requirements/design/test deltas
and independent design review now, alongside Candle's actual checked F32 helper
RED/GREEN. Actual adoption remains qualified Candle → reviewed coherent exact
consumption route → product admission/mapping TDD → affected consumer and joint
qualification. Do not repeat repair selection or accepted fork CPU suites.
The [source boundary](design.md#current-source-implementation-boundary) names the actual available interfaces and held work. No partial fork pin,
missing-API compilation RED or outer-lease-only Candle bypass is permitted.

1. Independent Astra-medium design review is complete (PASS after partial-load
   reset was kept seam-only). Owner seq-376/377 is the selected policy and live-fault
   disposition; reconcile its active .29 source/seam scope before implementation.
2. Under the selected policy, place necessary low-level design/tests in each
   owning fork's existing `fathomdb/` area. Parent supplies isolated worktrees and
   independent fork design review. No nested agents or shared checkout writer.
3. Follow the test map in per-behavior genuine RED commit → separate GREEN commit
   order. RED body records the actual semantic assertion failure; preserve the
   oracle during GREEN. Missing-helper compile failures are not behavioral RED.
   Reuse accepted exact cudarc CPU qualification for necessary Candle consumption
   changes; remaining actual-device/consumer proof retains its own scope.
4. Consume independently reviewed qualified repair source. Registry versions must
   exist before registry pins; publication is outside this mandate. If necessary,
   propose a reviewed exact-rev integration route rather than inventing a registry
   checksum or silently publishing. Coordinate manifests/lock/interfaces with120/140.
5. Implement Fathom admission, typed mapping and safe diagnostics; serialize
   engine landings. Use owned tooling, scoped checks while working and one final
   clean full source gate per code phase; rerun affected failures only unless
   wider changes justify another gate.
6. Qualify source/seam changes and applicable healthy/OOM/installed lifecycle
   controls in the available owned window. Actual Orin/3090 reset children,
   reset-immediate-close, last-clone reset and reset consumers remain HELD under
   seq-377; review them in [.30 follow-up](../../../0.8.30/release-followups.md).
   No live reset-survival or original-defect closure follows from seam proof.
7. Amend accepted ADR and affected Rust/SDK/Python/TS contracts in the same PR
   with only selected and proved support. Parent updates release JSON/views once
   with substantive results. Unknown/failed cells remain explicit.

## Resources and exit

The current-pin subset has actual CPU source/seam proof; remaining tracked
integration is held at its qualified fork/Candle artifact dependencies. Future
faults use disposable child-owned primary contexts and
selected GPU UUIDs at explicitly permitted exclusive venues. Never K620,
host-wide GPU reset, driver changes, primary CUDA symlink changes or reflash.
windchill3 remains R580 and primary CUDA12.6. Owned work stays under `/mnt/data/`;
never relocate or clean another session's resources. CI, tag/registry publication
and published-release completion remain outside the current mandate.

The .29 source exit requires S110-A1–A6 only within their selected source/seam
and applicable healthy-control scope, genuine RED/GREEN, qualified consumed pins
and independent final review. Seq-376 accepts frozen native live-set residue
until restart, with no growth and opaque native host bytes without a hard bound;
this explicit policy choice is not a universal-byte-bound proof. Seq-377 removes
real reset execution from mandatory .29 exit, not from future support proof:
reset-survival and TC-281155a1 original-defect closure remain unqualified. Review
policy/live faults in .30. No further mandatory live-fault gate is inferred.
Apply the quality roadmap's bounded lifetime/state properties and controls;
broad GPU fuzzing remains deferred. New CUDA13 delivery/compiler and quantitative
work moves to .31; unrelated platform and installed-artifact qualification stays.
