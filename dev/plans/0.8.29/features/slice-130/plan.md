---
title: "0.8.29 Slice 130 — CUDA platform, SDK release and soak qualification"
status: PROPOSED
target_release: 0.8.29
slice: 130
---

# Slice 130 — CUDA platform, SDK release and soak qualification

## Goal, evidence and independent units

Qualify selected platform and resource claims at the chosen .29 source/artifacts.
This draft executes no campaign and authorizes no provisioning, installation or
host reflash. Read the [brief](../../planning-brief.md), [reconciliation](../../reconciliation-plan.md),
[evidence register](../../planning-evidence.md), [.28 qualification](../../../0.8.28/features/slice-31/qualification.md),
[pool contract](../../../../adr/ADR-0.8.28-tegra-private-cuda-pool.md),
[allocation-witness successor](../../../../adr/ADR-0.8.28-attributable-cuda-allocation-witness.md),
[close design](../../../0.8.28/features/slice-31/design.md#141-n1-the-close-release-q29-contract-ruled-by-q34)
and [historical soak](../../../0.8.28/features/slice-31/qualification/2026-10-15-ORIN-G8-SOAK-32295692f56b.md).

| Unit / carryover | Established evidence | Proposed qualification delta |
| --- | --- | --- |
| A / `TC-b267d940` | Q5 Tegra CUDA13 remains UNMEASURED | Approved actual venue, then exact CUDA13 Tegra artifact/SDK qualification |
| B / `TC-ff476f59` | x86 GCP CUDA cells do not qualify aarch64 | Approved non-Tegra AWS g5g host and actual generic ARM CUDA consumers |
| C / `TC-4755a9d9` | aarch64 Python/Node currently initialize CUDA at load | Separate non-Tegra early-init hypothesis, design and fork/V8 hazard evidence |
| D / `TC-8278f17d` | Embedder close release measured; SDK reserved bytes unmeasured | Rust/Python/Node test-only observations through real SDK lifecycle paths |
| E / `TC-84f7f62d` | Q39 accepted old soak as measured, including failed drift rows | Preregistered plateau/control criterion and isolated sampler churn |

Fork memories inspected: `cudarc-fork-qualification-and-tegra-pool-facts.md`,
`forks-coreyt-cudarc-and-candle-fathomdb.md` and
`windchill3-never-change-driver-or-primary-cuda.md` in the verified external store.
Their measured class is AGX Orin64/JetPack6/CUDA12.6; no new model/class support follows.

## Preparation: local needs, requirements, acceptance and design

Deliver `requirements.md`, `design.md`, `test-map.md` and `campaign-protocol.md`
beside this plan before code/runs. These are draft release-local IDs, not new
canonical acceptance IDs. Each selected unit needs a separately reviewable exit.

- N130-1: users need platform and toolkit claims backed by their actual artifacts.
- N130-2: SDK users need truthful last-model memory release and bounded steady use.
- R130-1 / A130-1: A/B receipts bind host architecture/Tegra identity, UUID,
  driver/toolkit/libraries, model, source/native artifact hashes and actual forward;
  missing hosts/artifacts produce UNMEASURED cells, never a pass or inferred support.
- R130-2 / A130-2: C changes initialization only after reviewed non-Tegra evidence;
  import/registration, CPU-only/driverless/opt-out, fork and Node heap controls
  meet the approved design while proven Orin behavior remains qualified.
- R130-3 / A130-3: D observes the actual decided private pool: under `max`, one
  remaining model retains reservation; after the last model drops, checked
  `reserved_current == 0`; threshold `0`, singleton and clean exit controls agree
  with the accepted contract. Define bounded scheduler-reference settling first.
- R130-4 / A130-4: E freezes numeric terminal-slope/plateau/control-difference
  bounds, warm-up, sample cadence, duration and rejection rules before results;
  the declared controls pass with zero errors/mismatches and no swap.
- R130-5 / A130-5: final claims identify final selected pins/artifacts and every
  failure/skip; unaffected historical evidence is reused only with a scoped rationale.
Read current [shared init helper](../../../../../src/rust/crates/fathomdb-embedder/src/cuda_driver_init.rs),
[Node hook](../../../../../src/rust/crates/fathomdb-napi/src/cuda_early_init.rs),
[Python hook](../../../../../src/rust/crates/fathomdb-py/src/lib.rs) and
[release tests](../../../../../src/rust/crates/fathomdb-embedder/src/cuda_pool_release_tests.rs).
Node registration runs outside the loader lock; do not replace it with a constructor.
Python CUDA features imply `tegra-pool`; source-built non-Tegra aarch64 can run
its hook too. Rust has no load hook and admits the successful probe initialization.
Research official pinned CUDA pool/init documentation, actual g5g/venue capabilities,
Python fork/spawn/forkserver and Node worker/V8 behavior only for unresolved design
questions. Record versions and observations; this draft performs no fresh web review.
Fit the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md): use
real SDK shutdown/resource controls and measured post-state; defer broad fuzzing and
exhaustive hardware matrices. The new USED witness improves allocation attribution,
but cannot prove reserved-memory release; keep both measurements separate.

## Dependencies, prerequisites and safe ordering

Hard: Slice9 selects units/claims and resolves venue/spend authority; approved10
provides owned tools/native environments, resource/GPU locks and UUID selector
preparation (`TC-ad1f7d4f`) before new CUDA campaigns. Missing setup fails preflight.
A requires an explicitly approved CUDA13 Tegra venue and its actual software stack;
CloudJetson is a candidate, not booked capacity or permission to reflash the Orin.
B requires available approved g5g access, driver/toolkit setup and ARM artifacts;
C needs B's host evidence plus its own accepted hazard/design change before code.
D/E can proceed on the qualified CUDA12.6 Orin independently of A/B availability.
Final affected campaigns require selected140 fork unit qualification → chosen pins
→ affected110 integration and120 artifact qualification →130 final reruns.
140's initial fork qualification and120's initial artifact qualification do NOT
require130 completion; their bounded unit tests precede these final composed runs.
Design/harness preparation can start earlier; pin movement invalidates only affected
receipts after explicit blast-radius review. Keep reset fault claims in110.
New prerequisites: exact available venues, artifact selection per cell, approved
settling bound, private counter test seam, C's hazard hypothesis and E's numeric rule.
windchill3 stays R580 with primary CUDA12.6; guard before/after runs, select approved
3090s by UUID, never K620. A host absence blocks its claim, not unrelated .29 work.

## Tasks, implementation discipline and deliverables

1. Freeze the selected-unit matrix, contract/test traceability and provenance fields;
   independently review requirements/design/protocol before implementation or spend.
2. A/B: establish approved host identity and installed consumer contracts, then run
   forced CUDA forwards, library/allocator reports, CPU/auto/driverless controls and
   orderly close/exit for relevant Rust/Python/Node consumers. Retain named refusals.
3. C: test the hypothesis that non-Tegra load-time initialization is unnecessary;
   compare enabled/late-init paths, import order, Node heap/worker loads and Python
   fork-before-CUDA, spawn/forkserver controls. Parent CUDA use still carries fork
   hazards. Preserve Orin heap-reservation proof, exact opt-out and all-CPU skip;
   no automatic removal or widened pool admission from a passing g5g forward alone.
4. D: add narrowly gated test-only counters bound to the real process/context/pool;
   no public SDK counter API. Separate subprocesses exercise each SDK, two engines,
   first/last close, drop, threshold0/max and process-lifetime model exceptions.
   Observe after actual ownership release without caller trim/synchronize that
   manufactures success. USED allocation-witness controls remain independent;
   RSS or global UMA free memory cannot substitute for `reserved_current`.
5. E: remove Python sampler's deliberate 200000-dict churn from the primary soak
   or isolate it as a separately identified arm. Run matched pool-on/off 60-minute
   controls and judge the preregistered terminal30-minute plateau, RSS/pool series,
   latency and output checks. No post-result threshold edits or selective retries.
6. Commit reproducible harnesses and raw/summary receipts with exact commands,
   durations, versions, identities, guards and failure dispositions. Preserve Q39,
   original failed drift rows and failed v1 witness records without relabeling them.

## Verification, review and completion decisions

For each behavior change, stage/commit its genuine assertion-failing RED before
GREEN; existing passing characterization is retained without inventing a RED.
Use focused lint→typecheck→tests during work, real native binding/SDK checks and
required full agent verification once on the final code phase; contract/docs edits
use their scoped validators. Obtain independent code/protocol/results review,
audit RED evidence and integrate from current .29 in isolated implementer worktrees.
Serialize native/interface/harness landings with110/120/140; never edit a running
runner. Stop the affected unit on genuine failure, identity ambiguity, unapproved
venue/change, missing artifact or unresolved hazard; resolve or record explicit
owner deferral and narrow the claim. Deliver unit verdicts to160, not publication
permission. No host availability, pass, review approval or new support is claimed here.

## Fork scope boundary (owner seq-367)

Only required repairs from .29 Slice140 may supply affected pins here. The
[broader .30 maintenance](../../../0.8.30/fork-maintenance-plan.md) is not a
prerequisite. A need for that broader work returns to the owner for explicit
scope review; it is not pulled into .29 automatically.
