# Slice 130 requirements

[Active owner dispositions](plan.md#phase-a-scope): seq-381 moves Tegra CUDA13
unit A to .31; D/E and existing CUDA12.6 obligations remain .29. Owner seq-387
excludes unavailable non-Tegra ARM+GPU B/C from known-working environments and
every .29 release gate; they remain unqualified with no future release assigned;
new x86 CUDA13 delivery/compiler work is .31 and real110 resets are HELD .30.
References to120 artifacts below mean applicable independently selected artifacts,
not a mandatory new x86 package or blanket120 dependency.

Status: Phase A review candidate; no canonical AC amendment or new support claim.
Authority: release N29-G/K/D, R29-REL-G, R29-3P-19/O3-19 and the accepted
[pool](../../../../adr/ADR-0.8.28-tegra-private-cuda-pool.md),
[allocation witness](../../../../adr/ADR-0.8.28-attributable-cuda-allocation-witness.md),
[CUDA policy](../../../../adr/ADR-0.8.28-cuda-version-support-policy.md),
[driverless linkage](../../../../adr/ADR-0.8.25-driverless-cuda-runtime-linkage.md)
and [Tegra distribution](../../../../adr/ADR-0.8.27-jetson-tegra-node-addon-distribution.md)
contracts. Canonical REQ-020a/b/021 and REQ-068, AC-082a–d apply within their
actual platform scope. Their x86 language is not silently widened to ARM.

| Need / local requirement | Human intent and required behavior | Exit |
| --- | --- | --- |
| N130-platform / S130-R1 | Users get claims only for actual source/artifact/ABI/architecture/toolkit/PTX/driver/library/model/device combinations that ran. A needs actual Tegra13 proof in .31 under seq-381, not as a .29 gate; B is unqualified and excluded from .29 by seq-387. Source-built proof and distributed product proof stay distinct. | S130-A4/A5/A6; A1 future .31, A2 excluded |
| N130-policy / S130-R2 | Forced CUDA performs CUDA or fails typed; CPU/driverless/auto controls obey each artifact's accepted contract. Wrong-major libraries never silently satisfy another artifact. No pool admission expansion, allocator switching, CPU fallback under forced CUDA, reset-survival claim or new precision is inferred. | S130-A4/A5; A1 future .31, A2/A3 excluded |
| N130-init / S130-R3 — outside .29 (seq-387) | Future unqualified C tests whether generic ARM needs load-time initialization and whether late init avoids the import-before-fork hazard. Preserve exact opt-out/all-CPU skip, loader-lock safety, qualified Orin heap behavior and fork-after-parent-CUDA restriction. Change requires evidence, accepted contract update and separate RED/GREEN. | S130-A3 |
| N130-release / S130-R4 | Every SDK must observe the actual decided private pool through real lifecycle paths. At max, RESERVED must be unchanged across first close with no intervening allocation/forward and another model live, before its healthy forward is tested; after the last non-singleton model is actually released, RESERVED reaches zero within a reviewed finite settling bound. Observation cannot trim, synchronize, initialize or create the success. | S130-A4 |
| N130-release / S130-R5 | Threshold0, first/last close, implicit drop and process-lifetime CLS/reranker exceptions agree with accepted Q29/Q34. Errors/no-private decision are named, not zero-filled. USED allocation proof, RESERVED release, RSS and shared UMA are distinct. | S130-A4 |
| N130-steady / S130-R6 | E removes deliberate Python sampler churn and measures a matched instrumented pool-on/off60-minute pair with frozen numeric terminal30-minute plateau/control rules, mandatory on-pool RESERVED series, no swap/errors/mismatches and explicit shipping-build equivalence. Exact shipping lifecycle/witness evidence is separately required; no shipping-binary numerical plateau is inferred. Historical Q39 and failed drift/v1 rows remain unchanged. | S130-A5 |
| N130-truth / S130-R7 | Each cell records final selected inputs, actual attempts/skips/failures/exits and exact reuse rationale. No spend/install/host modification proceeds from this plan alone; exact gates and source/review prerequisites hold. | S130-A6 |

See [acceptance](acceptance.md) for observable criteria and [test map](test-map.md)
for inspected existing coverage versus proposed gaps. S130 requirements preserve
store format37 and require no database migration or new public counter API.
