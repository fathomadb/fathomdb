---
title: FathomDB 0.8.27 Slice 117 — Jetson CUDA Node addon status
status: PLANNED
target_release: 0.8.27
planning_baseline: 80246a567
---

# Slice 117 status

**PLANNED; draft; implementation waits on the delivery-shape rulings.**
Status refreshed 2026-10-09 after Slice 135 closeout. The
[plan](plan.md),
[design note](design.md) and proposed
`dev/adr/ADR-0.8.27-jetson-tegra-node-addon-distribution.md` are drafts for
owner review. Only planning and governance records have changed. No code,
workflow, package or public documentation change has been made, and nothing
has been built, qualified or published.

## Hold

**Owner, 2026-10-05: do not start Slice 117 implementation until Linux x86_64
(amd64) has been tested with the Slice 110 fixes on
`llm/slice110-tegra-allocator-fix`.** That branch did not compile x86_64
(`dev/plans/0.8.27/features/slice-110/tegra-integration-pending.md`). The hold
lifts only when an x86_64 build-and-test result for those fixes is recorded.
It is part of ruled decision `slice-117-jetson-node-cuda-direction`.

**Hold condition met, 2026-10-05.** The x86_64 round on Slice 110 candidate
`8b76f6115` is recorded in
`dev/plans/0.8.27/features/slice-110/tegra-integration-pending.md` (section
"AMD64 regression round on the updated branch"): vendored cudarc tests,
workspace clippy and check, the CUDA N-API release build, and installed-package
forced-CUDA embed and rerank on an RTX 3090 passed; the full `agent-verify`
was not run there. Plan entry criterion 2 is also met: Slice 110 is complete
on `release/0.8.27` at `a25d063cd`. Implementation waits only for entry
criterion 3 (remaining delivery-shape items ruled).

Slice 135 is complete for its pinned product source. Slice 117 will produce
fresh installed Jetson Node candidate evidence for CPU, auto and forced-CUDA
embedding and reranking, the GPU witness, heap growth, loader selection and
generic ARM64 non-interference. It does not inherit Slice 135 measurements or
repeat its full diagnostic matrix. Query or retrieval changes beyond loader
and packaging work require the affected paired Slice 135 cells to be repeated.
The full code gate, exact install docs and independent review precede the
Slice 140 handoff. Post-publication smoke remains a separate completion gate;
Slice 150 owns final-candidate qualification.

## Rulings recorded

- `slice-117-jetson-node-cuda-direction`: Slice 117 is on the ladder, its
  planning and governance work is authorized, and implementation was held
  until the x86_64 test above (met).
- `slice-117-supersede-d-80-7-3-and-d-80-6-2`: D-80.7-3 and D-80.6-2 are
  superseded for Slice 117's scope. In-place notes are in
  `dev/design/0.8.23-aarch64-tegra.md` and `dev/tegra-platform-reference.md`
  § 3.7.
- `slice-110-early-cuinit-with-allocator-fallback`: Slice 110 ships both
  changes.
- `slice-117-channel-tegra-pages`: the channel is the existing Tegra Pages
  route. The npm registry options are not chosen for 0.8.27.
- `tegra-allocator-0.8.27-sync-fallback-pool-study-0.8.28`: no memory pool
  ships in 0.8.27; 0.8.28 evaluates an explicit or lazily created pool.

## Blockers

- The remaining items of unruled `slice-117-delivery-shape`
  ([design](design.md) § 10; the channel is ruled), and acceptance of the
  proposed distribution ADR.
- Publication, and therefore AC27-117F, remains behind the release's unruled
  `release-0.8.27-publication` decision.

## Acceptance

| Acceptance | Result |
| --- | --- |
| AC27-117A | Partial: the supersession ruling, its in-place notes and the channel ruling are recorded. The remaining delivery-shape items and ADR acceptance are pending. |
| AC27-117B–H | Not started. |

## Tracking

- Todos ledger: `TC-ffef2129-e5a6-4f8f-8cae-bdfe64792197` (opened at seq 274
  and updated at seq 276 on `release/0.8.27`, replayed from the planning
  branch's seq 271 and 273; after Slice 110's seq-270 revisit todo).
- Release state: Slice 117 `PLANNED`, `depends_on: [110]`, in
  `dev/plans/release-state-0.8.27.json`, with the five ruled records above
  and unruled `slice-117-delivery-shape`. Its remaining-ladder position is
  after completed Slice 135 and before Slice 140.
- Decision index: row 61 (proposed).
- On completion, Slice 117 updates the platform-support statement in
  `README.md`, which at this plan's baseline says the Linux AArch64 npm package
  is CPU-only and that Jetson GPU under Node requires a source build.
