---
title: Slice 140 — required fork repairs only
status: PROPOSED
target_release: 0.8.29
slice: 140
---

# Slice 140 — required fork repairs only

## Owner scope and boundary

Owner ruling, steward `seq-367` (November 10): **focus on A in 0.8.29; move
B/C work to 0.8.30**. Release allocation is decided; implementation design,
qualification and publication are separate gates. The
[decision log](../../prework/slice-9/decision-log.md) cites that ruling and the
[0.8.30 receiving plan](../../../0.8.30/fork-maintenance-plan.md) owns deferred
maintenance. It is not a prerequisite for closing .29.

This slice covers only the smallest fork repairs demonstrably required by the
selected reset-safety work in110 or artifact work in120, plus version-policy
decisions needed to release those repairs. It does not authorize a general
Candle rebase, unused-marker cleanup or multiarchitecture kernel expansion.
If a required repair cannot remain inside this boundary, stop for an explicit
owner scope decision. Do not silently pull .30 work back into .29.

## Units and release disposition

| Unit / stable identity | 0.8.29 scope | Receiving exit or destination |
| --- | --- | --- |
| U140-R required repair | Conditional on concrete approved110/120 design evidence; minimal cudarc/Candle lifetime, correctness or delivery repair | Fork-local requirement/regression, design, review and qualification; product proof stays in110/120/130. |
| U140-V / FT-1 / TC-daa1985b | Only if an included repair needs a new cudarc code release | Choose a distinct publishable version with exact upstream-base provenance; qualify before pin adoption. No independent maintenance expansion. |
| U140-C / TC-509dc8a6 | Excluded from .29 | Candle upstream rebase moves to .30. |
| U140-M / FT-2 / TC-93d11583 | Excluded from .29; retain current marker | Marker/removal-path cleanup moves to .30. |
| U140-K / TC-323f15b0 | Excluded from .29 | Multiarchitecture SASS/PTX, selection, dtype/kernel availability and MoE coverage move to .30. |

Retain qualified pins unless a selected required repair justifies a change:
cudarc-fathomdb0.19.10 (`b777f8d9`), Candle-fathomdb0.11.0/kernels source
`182ceab0`. The fork's0.11.0 is based on upstream0.10.2, not upstream0.11.x.
Current x86 kernels target7.5 and product embed/rerank useF32. Neither new GPU
targets nor BF16/F8 product modes are implied by required repair scope.

## Local requirements and acceptance

- R140-1/A140-1: every proposed diff identifies the specific110/120 obligation
  requiring it and why a smaller repair is insufficient. Deferred .30 work is
  explicitly excluded from the diff and dependency graph.
- R140-2/A140-2: preserve current capabilities: private-pool ownership, driverless
  CPU loading, AArch64 CPU safety, typed CUDA refusal and supported artifact
  behavior. Public API changes update accepted interfaces/ADRs in the same PR;
  no backwards-compatibility shims are introduced.
- R140-3/A140-3: authenticated fork sources and exact package/feature closure
  match qualified repair commits. Root patches do not propagate to registry
  consumers; source and installed-consumer claims stay distinct.
- R140-4/A140-4: observed failing behavioral controls precede repair; healthy
  teardown/resource release and affected CPU/GPU paths remain correct. An
  expected-crash characterization is not proof of safe teardown.
- R140-5/A140-5: optional new release versioning is bounded to required repairs;
  no stale candidate lock, unqualified pin or deferred rebase enters indirectly.
- R140-6/A140-6: .29 closure needs only selected required-repair exits or explicit
  no-repair-needed disposition; it never waits for the .30 maintenance plan.

These are local criteria, not new canonical AC IDs or current execution claims.

## Design, ownership and qualification

Use [accepted pool/fork contracts](../../../../adr/ADR-0.8.28-tegra-private-cuda-pool.md),
[driverless linkage](../../../../adr/ADR-0.8.25-driverless-cuda-runtime-linkage.md),
the [pin registry](../../../../../scripts/pinned-override-rot.json) and exact
consumed source. Read requirements/design/test mappings before code. The entire
ownership graph and cleanup paths must support the approved reset fault model;
artifact compiler exposure/mitigation remains a120 design prerequisite.

Requirements, design, TDD, review and fork qualification live in each owning
fork's `fathomdb/` directory. FathomDB consumes qualified exact identities and
tests product integration; it does not rerun or duplicate the fork test suites.
Apply the [quality roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md)
with bounded lifecycle/resource controls, independent outputs and honest platform
limits. General GPU/dtype expansion and broad build tuning belong to .30.

1. Identify concrete110/120 repair requirements and draft fork-local design/test
   mapping; independently review before implementation.
2. Stage/commit an observed failing regression before minimal repair; preserve
   the oracle through GREEN/refactor. Qualify affected fork behavior and healthy
   controls with an isolated writer and owned tools.
3. Resolve FT-1 only if a new cudarc code release is needed; obtain required fork
   release authority, verify package/source identities and review pin adoption.
4. Adopt qualified cudarc → dependent Candle → FathomDB pins where required;
   update locks, pin registry and relied-on item records together. Run affected
   product/runtime/packaging checks and the final source-phase full gate once.
5. Record required-repair/no-repair-needed disposition and affected consumer
   evidence. Leave deferred .30 units open under their stable TC identities.

Hard inputs: approved110/120 design and required10 setup; strict preflight before
implementation worktrees. Included repair qualification precedes affected110/120
pin adoption and130 final qualification. No dependency on a .30 rebase or kernel
campaign exists. Serialize manifests/pins/shared source landings. Fault execution
still requires a reviewed model and explicitly permitted isolated venue.
R580/primary12.6 stay fixed on windchill3; never runK620 or infer unmeasured
Orin/CUDA13 support. Publication remains separately authorized.
