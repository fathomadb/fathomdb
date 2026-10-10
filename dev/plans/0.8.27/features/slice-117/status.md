---
title: FathomDB 0.8.27 Slice 117 — Jetson CUDA Node addon status
status: PLANNED
target_release: 0.8.27
planning_baseline: 80246a567
---

# Slice 117 status

**PLANNED; implementation authorized 2026-10-09** (distribution ADR accepted;
`slice-117-delivery-shape` ruled).
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
- `slice-117-delivery-shape` (2026-10-09): the owner accepted
  `dev/adr/ADR-0.8.27-jetson-tegra-node-addon-distribution.md` as proposed,
  ruling every remaining delivery-shape item.

## Blockers

- Publication, and therefore AC27-117F, remains behind the release's unruled
  `release-0.8.27-publication` decision.

## Acceptance

| Acceptance | Result |
| --- | --- |
| AC27-117A | Done: the supersession ruling and its in-place notes, the channel ruling (2026-10-05), and the delivery-shape ruling with the distribution ADR accepted (2026-10-09). |
| AC27-117B | Implemented; awaiting the exact-SHA candidate run. `35f098d6e` red, `f704790fa` green (wrapper; `8a1c4a17d` mode bit; `4812c10df`/`9f3757fc8` napi destination fix). A dev build on the AGX Orin passed: tegra glibc floor (max GLIBC_2.34 of 2.35), no CUDA/NVIDIA `NEEDED`, installed import with CUDA search paths unset. |
| AC27-117C | Implemented; awaiting the exact-SHA candidate run. Smoke `e74e37d77`/`8b002efba`. A dev-build run (not candidate evidence) passed every row: embed and rerank `cpu` on CPU, `auto` and `cuda:0` on CUDA; witness pass with `--nvidia-smi`; heap regression 3/3; the Tegra `.node` selected with and without a generic package installed. Driverless import: hosted job `f98019f1c`/`eba09e4a1` (contract-proven here; the real run is on the hosted ARM64 runner). |
| AC27-117D | Implemented. Loader `aedcc2f32`/`1b86f37a4` (11 `LoaderSeams` cases); inject-script refusal and generic-package non-interference tests `35f098d6e`; cross-host install fixture `1ce25ce83`/`48da2036d`; platform-capabilities variant `2abc53474`/`e568c5574`. |
| AC27-117E | Implemented; awaiting the exact-SHA workflow receipt. Retained list `5a8fc2fc0`/`98040492b`; Jetson run builds, smokes and retains wheel and tarballs `c0e96fc41`/`c6b1c5462` (timeout 180 min by owner ruling, `51b34677d`/`d5a88b3f6`); operator path `b148c6180`/`4b4185d97`; post-deploy live verification in both deployers `bd9675e06`/`f47122478`. |
| AC27-117F | Implemented in `publish-tegra-pages.sh`; waits on `release-0.8.27-publication`. |
| AC27-117G | Docs written with a placeholder 0.8.27 digest (`48da2036d`, `11bf12120`, `547b648ad`, `4e463af5a`); the publisher fills the digest. |
| AC27-117H | Not started: full gate, independent review, candidate receipt. |

## Pages retention finding (2026-10-10)

The owner checked the live Pages URLs: the `0.8.24+tegra` wheel returns 404
(encoded and literal `+` forms), and the `0.8.26+tegra` wheel returns 200
(8,987,812 bytes). The 0.8.24 wheel was already unpublished before this
ADR's retention rule, so the retained list holds only `0.8.26+tegra`; a dead
entry would fail every deployment. `main`'s `docs-pages.yml` still fetches
the dead 0.8.24 URL with `curl --fail`, so a Pages deploy from `main` fails
until 0.8.27 merges. A release task in `dev/plans/plan-0.8.27.md` records a
passing `docs-pages` run on `main`, including the new post-deploy live
verification, after that merge.

## Tracking

- Todos ledger: `TC-ffef2129-e5a6-4f8f-8cae-bdfe64792197` (opened at seq 274
  and updated at seq 276 on `release/0.8.27`, replayed from the planning
  branch's seq 271 and 273; after Slice 110's seq-270 revisit todo).
- Release state: Slice 117 `PLANNED`, `depends_on: [110]`, in
  `dev/plans/release-state-0.8.27.json`, with the six ruled records above. Its remaining-ladder position is
  after completed Slice 135 and before Slice 140.
- Decision index: row 61 (proposed).
- On completion, Slice 117 updates the platform-support statement in
  `README.md`, which at this plan's baseline says the Linux AArch64 npm package
  is CPU-only and that Jetson GPU under Node requires a source build.
