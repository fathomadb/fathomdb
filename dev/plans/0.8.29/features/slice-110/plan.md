---
title: Slice 110 — CUDA context-loss lifetime and safe teardown
status: DESIGN_REVIEWED_OWNER_FAULT_POLICY_PENDING
target_release: 0.8.29
slice: 110
---

# Slice 110 — CUDA context-loss lifetime and safe teardown

Resolve TC-281155a1 through positive operation refusal, close, drop and normal
exit in an explicitly selected loss envelope. Detection alone cannot close it.
This replaces the proposal-only plan after inspecting accepted source
`a00bf9b9228318e7c236148551bb188f63ff66ee` on November14. Independent Astra-medium design review passed; fault execution is not approved. [Requirements](requirements.md),
[design](design.md) and [test map](test-map.md) are the complete slice packet.

## Scope and predecessors

- [Master](../../../plan-0.8.29.md) and [release state](../../../release-state-0.8.29.json)
  allocate110 under R29-REL-R/G, R29-3P-17/D29-RESET and O3-17.10 is closed;
  integrated Linux20/70 provide lifecycle/error behavior;60/100 are qualified
  subsets, not whole-slice closure. Source stays pinned during G recovery;
  queued20 then M precede110 in the shared heavy window.
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
- 120 selects artifacts/policy;130 owns affected final platform and SDK pool/soak
  qualification. Reviewed110 obligation → initial140 proof → qualified pins
  →110 integration → affected130 qualification. Completed110 never gates140.

## Execution sequence

1. Independent Astra-medium design review is complete (PASS after partial-load
   reset was kept seam-only). Present the concrete question in design § Owner
   decision; unchanged design is not reviewed again.
2. After actual policy selection, place necessary low-level design/tests in each
   owning fork's existing `fathomdb/` area. Parent supplies isolated worktrees and
   independent fork design review. No nested agents or shared checkout writer.
3. Follow the test map in per-behavior genuine RED commit → separate GREEN commit
   order. RED body records the actual semantic assertion failure; preserve the
   oracle during GREEN. Missing-helper compile failures are not behavioral RED.
   Qualify cudarc before necessary Candle consumption changes.
4. Consume independently reviewed qualified repair source. Registry versions must
   exist before registry pins; publication is outside this mandate. If necessary,
   propose a reviewed exact-rev integration route rather than inventing a registry
   checksum or silently publishing. Coordinate manifests/lock/interfaces with120/140.
5. Implement Fathom admission, typed mapping and safe diagnostics; serialize
   engine landings. Use owned tooling, scoped checks while working and one final
   clean full source gate per code phase; rerun affected failures only unless
   wider changes justify another gate.
6. After actual venue/session authority and an available heavy window, run bounded
   positive child cases after completed construction. Partial-load loss remains
   seam-only, outside real reset support pending a separately reviewed handoff.
   Orin private pool is the original defect; a3090 result
   cannot close it. Run source-bound installed Rust/Python/Node consumers, then
   fresh independent Astra implementation/results review and parent integration.
7. Amend accepted ADR and affected Rust/SDK/Python/TS contracts in the same PR
   with only selected and proved support. Parent updates release JSON/views once
   with substantive results. Unknown/failed cells remain explicit.

## Resources and exit

No faults, builds, native installs, fork mutations or product edits were performed
for this design. Later faults use disposable child-owned primary contexts and
selected GPU UUIDs at explicitly permitted exclusive venues. Never K620,
host-wide GPU reset, driver changes, primary CUDA symlink changes or reflash.
windchill3 remains R580 and primary CUDA12.6. Owned work stays under `/mnt/data/`;
never relocate or clean another session's resources. CI, tag/registry publication
and published-release completion remain outside the current mandate.

Exit requires S110-A1–A6, actual cleanup-policy ruling, positive teardown/normal
exit on the original private-pool defect and declared support cells, and resolved
reviews. The opaque native-residue byte question is not an implied waiver.
Apply the quality roadmap's bounded lifetime anomalies, state properties and
resource controls; defer broad GPU fuzzing/exhaustive architecture matrices.
The master already records the dated CUDA usage review;120 owns policy changes.
