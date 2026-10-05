---
title: FathomDB 0.8.27 Slice 117 — Jetson CUDA Node addon status
status: PLANNED
target_release: 0.8.27
planning_baseline: 6735fd9f1
---

# Slice 117 status

**PLANNED; draft; uncommissioned.** The [plan](plan.md) and
[design note](design.md) are drafts for owner review. No code, workflow,
package or documentation change other than planning has been made, and
nothing has been built, qualified or published.

## Blockers

- Slice 110 must close with the aarch64-Linux allocator fallback and early
  `cuInit` integrated on `release/0.8.27`.
- The owner must record a ruling that supersedes D-80.7-3 and D-80.6-2 for
  Node on Tegra, and decide the open items in [design](design.md) § 9.
- Publication, and therefore AC27-117F, remains behind the release's unruled
  `release-0.8.27-publication` decision.

## Acceptance

| Acceptance | Result |
| --- | --- |
| AC27-117A–G | Not started. |

## Tracking

- Todos ledger: `TC-ffef2129-e5a6-4f8f-8cae-bdfe64792197` (seq 271, after Slice 110's seq-270 revisit todo).
- Release state: Slice 117 `PLANNED`, `depends_on: [110]`, in
  `dev/plans/release-state-0.8.27.json`.
- On completion, Slice 117 updates the platform-support statement in
  `README.md`, which at this plan's baseline says the Linux AArch64 npm package
  is CPU-only and that Jetson GPU under Node requires a source build.
