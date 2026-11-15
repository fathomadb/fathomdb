---
title: "Slice 130 — CUDA platform, SDK release and soak qualification"
status: WAITING_FOR_CAMPAIGN_INPUTS
target_release: 0.8.29
slice: 130
---

# Slice 130 — CUDA platform, SDK release and soak qualification

Owner 2026-11-14 seq-378 moves only proposed new x86 CUDA13 delivery/compiler
work formerly in120 to [.31](../../../0.8.31/cuda-delivery-plan.md). Owner seq-381 additionally moves
Tegra CUDA13 unit A to [.31](../../../0.8.31/cuda-delivery-plan.md), preserving
TC-b267d940-1add-4d08-a596-a7669ae4a500 and future venue/proof requirements.
D/E and existing CUDA12.6 obligations remain .29 with their own target/artifact/
venue prerequisites. References below to120 artifacts mean applicable independently
selected artifacts, not an assumed new x86 CUDA13 package or a blanket120 gate.
Seq-376/377 selects110 source/seam repairs but holds Orin/3090 reset children for
.30;130 adds no live-reset gate or reset-survival claim. Seq-379 accepts bounded
quantitative unknowns/current ratings; quantitative evidence is .31.

## Phase A scope

Owner [seq-387](../../../release-state-0.8.29.json) removes non-Tegra ARM+GPU
from known-working environments and every .29 release gate. No confirming run
or cloud quote is required to apply that ruling.

The bounded D/E design has independent Astra-medium PASS. Reconcile actual
current source/artifact inputs and proceed with independent source/harness
preparation after applicable preflight and affected design review. This is not
campaign, provisioning or spend authority; missing venue/caps block only the
affected execution. [Requirements](requirements.md), [acceptance](acceptance.md),
[design](design.md), [test map](test-map.md) and
[campaign protocol](campaign-protocol.md) define separate A–E exits: A is future .31 under seq-381; only D/E remain .29 under seq-387.
[Status](status.md) records actual evidence, pending decisions and validation.
All S130 identifiers are release-local, not additions to canonical acceptance.

| Unit / carryover | Required qualification |
| --- | --- |
| A / TC-b267d940 — future .31 under seq-381 | Actual selected CUDA13 Tegra artifact/consumer on the previously directed, vetted CloudJetson venue; Q5 remains unmeasured until then. |
| B / TC-ff476f59 — excluded from .29 (seq-387) | Unqualified non-Tegra ARM+GPU configuration; no working-environment claim, procurement or .29 qualification requirement. No future release assigned. |
| C / TC-4755a9d9 — excluded from .29 (seq-387) | Unqualified hypothesis retained outside .29; no initialization-study or host-availability gate. No future release assigned. |
| D / TC-8278f17d | Actual Rust/Python/Node engine first/last close/drop RESERVED memory observations, including process-lifetime exceptions. |
| E / TC-84f7f62d | Revised plateau/control rule and churn-free sampler on the explicit instrumented counterpart pair, separately required exact shipping lifecycle/witness proof and reviewed build equivalence; preserve Q39 and every historical failed drift row. |

130's A–E are unrelated to fork maintenance A/B/C. Owner seq-367 limits140 to
required110/120 repairs and necessary release versioning; fork B/C is .30.
Seq-381 moves A to .31. Owner seq-387 separately excludes platform B/C from
.29; these unqualified units are not passed or silently placed into fork B/C.
Format37/no database migration, R580/primary12.6 on windchill3, never K620,
no host-wide reset/driver/symlink change and no local Orin reflash remain fixed.

## Readiness and dependency order

D/E are included .29 qualification work; A is allocated to .31 and B/C are
excluded under seq-387. Exact missing inputs block affected
execution or claims, not unrelated release work. Design/harness preparation can
precede110/120/140 closure. Source entry requires actual10 closure, final-baseline
reread, parent-created isolated worktree/strict preflight and independent fresh
Astra-medium design PASS. Reuse the existing accepted design unless the affected
source/contract/protocol changes; do not recapture its proposal-validation packet.

| Edge | Kind / meaning |
| --- | --- |
|10 actual closure → source entry | Hard; owned tools/native identity/resource reservation/UUID selection, not borrowed10 environments. |
| Vetted available CloudJetson/session → A (future .31) | Hard for actual A run; JetPack7.2/root/data handling and exact installed stack must be checked, not inferred from vendor marketing. |
| Reviewed D/E seam/protocol → D/E | Hard; existing CUDA12.6 Orin access/reservation required, independent of A/B availability. Numeric bounds below are proposals. |
| Required140 repair qualification → selected pins → affected110/120 integration/artifacts → final130 | Conditional hard edge for affected final claims;130 never gates initial140 or initial120 proof. |
| D before E; cheap controls before long/paid cells | Preferred order, avoiding wasted runs; no invented release-wide gate. |

New x86 CUDA13 package/channel/compiler choices formerly in120 belong to .31;
110's policy is selected but live reset execution remains HELD .30. No automatic
CUDA13.2 upgrade or new support follows. Final affected130 qualification needs
actual independently selected final pins and artifacts for its own cells.
Pin movement invalidates only affected receipts after explicit
comparison; unchanged evidence is reusable with recorded blast radius.

## Execution after readiness

1. Advance current D/E source/fixture/protocol preparation in parallel
   with110/140. Keep the accepted design and review coherent material deltas.
   Non-Tegra ARM+GPU B/C and cloud-session procurement do not gate .29. Final
   affected artifact/pin identity is hard for the campaign claim, not all tooling
   preparation. D/E retain their own permitted Orin reservation.
2. Same owner implements only approved seams with observed assertion-failing RED
   commits before separate GREEN commits. Existing passing coverage is preserved,
   never presented as new RED; no Slice20 exception applies here.
3. Commit reusable measurement tools/README and measurement-correctness controls;
   freeze protocol before results. Serialize shared native/manifests/interfaces
   with110/120/140; do not edit a running runner.
4. Run approved cells, stop on genuine failures, retain raw outcomes, exact
   commands/identities/exits and unit verdicts. No selective success retries.
5. Scoped lint→typecheck→tests during work; one full stable source-phase verifier,
   required affected feature/native/hardware controls, then fresh independent
   Astra-medium implementation/results review. Parent integrates only passing
   scope and updates JSON-owned state/views.160 receives results, not publication
   authority.

Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
to real lifecycle post-state, resource controls and bounded anomaly/measurement
tests. Defer exhaustive hardware, whole-workspace fuzz/MC-DC and new performance
sweeps. No tamper/digest-hardening test framework is added. Missing mandatory
acceptance requires explicit owner disposition; UNMEASURED is never PASS.
