---
title: Slice 140 — required fork repairs only
status: WAITING_FOR_REQUIRED_REPAIR_SELECTION
target_release: 0.8.29
slice: 140
---

# Slice 140 — required fork repairs only

Active .29 repair selection follows owner seq-376/377: necessary110 source/seam
repairs and independently required existing .29 artifact repairs only. New x86
CUDA13 delivery/compiler work formerly in120 is [.31](../../../0.8.31/cuda-delivery-plan.md)
under seq-378 and cannot create a mandatory .29 repair. Real Orin/3090 reset
children are HELD .30; no .29 live-fault gate, survival or original-defect closure
is inferred. Fork B/C maintenance remains .30. No repair is selected by this note.

## Scope and present maturity

Owner ruling seq-367 selects **A only** for .29: concrete required110 reset-
lifetime or independently required existing .29 artifact fork repairs and versioning necessary to
release those repairs. [B/C maintenance](../../../0.8.30/fork-maintenance-plan.md)
belongs to .30 and never gates .29.130 qualification A–E is a different label
set. No exact repair, new version, pin or artifact choice is selected here.
Status is WAITING_FOR_REQUIRED_REPAIR_SELECTION, not PLAN_READY.

This read-only Phase A reconciles actual source at
`02e30d861beddc8061af0dc22f12106d5e09dd32`. Actual10 implementation
`104dcbcbe92a1eca730d146b99ab17d444df6217` is integrated/closed via
`d7d928f8e`; its assembled/resumed qualification preserves failures/skips.
0–9 are proposal/decision outputs, not runtime qualification.20's active source
checkout is not predecessor evidence. See [source census](source-census.md)
and [status](status.md).

## Review package and selection contract

- [Requirements](requirements.md) and [acceptance](acceptance.md) retain the
  original R140/A140-1–6 local identities and canonical joins.
- [Design](design.md) distinguishes exact candidate repairs, evidence that
  selects them, fork ownership and version/pin provenance.
- [Test map](test-map.md) separates fork human-intent unit/negative/device
  qualification from110/existing .29 artifact integration and final130 qualification.

110's quiescent completed-construction/frozen-live-set policy is selected by
seq-376, with opaque host bytes unbounded and restart required; seq-377 holds
real reset execution to .30. Required source/seam changes still need exact
fork-local design/TDD/review/qualification. New120 x86 delivery/compiler choices
are proposed .31 work and select no .29 repair. Independently required current
artifact failures retain their own concrete selectors.

Every proposed U140-R repair must name an accepted110 or independently required existing .29 artifact obligation, exact
source witness, observed failing behavior, smaller alternative comparison,
affected fork files/APIs, finite cleanup/qualification contract and exact
required pin/version change. Without that packet, independently useful designs
remain unselected. A no-repair-needed exit requires explicit evidence against
the selected110/existing .29 artifact obligations; lack of selection is not that exit.

## Dependency and execution sequence

| Edge | Kind and meaning |
| --- | --- |
| Actual10 closure and current accepted source → source entry | Hard; reconcile later predecessor changes, parent-created isolated worktree and strict preflight; owned tools/artifacts.10 is now closed, but its environment is not borrowed. |
| Accepted110/existing .29 artifact requirement/design → a particular140 repair | Conditional prerequisite. Neither completed110 nor completed120 is required to identify a repair. |
| Fresh independent Astra design PASS → implementation | Hard for the selected fork-local design/test packet. The parent dispatches reviewers and creates fork worktrees at explicit reviewed bases. |
| Initial fork qualification → chosen exact pins → affected110/existing .29 artifact integration/artifacts → final130 | Conditional hard chain for an included repair. Final130 never gates initial140 or initial120 proof. |
| Cudarc before dependent Candle; cheap checks before live cells | Preferred order where the selected repair actually affects both; no gratuitous Candle release or GPU campaign. |
| .30 rebase/marker/multiarch/dtype/MoE maintenance → .29 | No edge. Preserve stable deferred TC identities in the receiving plan. |

After selection/review, each behavior gets an observed failing RED commit
before separate GREEN, with the oracle preserved. No missing-helper compile
failure, intentional break or passing-on-arrival coverage substitutes. Fork
requirements/design/tests/reviews/receipts live in each fork's `fathomdb/`;
FathomDB records qualified consumption and product proof, not duplicate fork
suite execution. Source phases use scoped checks while working and one final
clean full applicable gate; fresh independent implementation/results review
precedes parent integration. Serialize shared manifests, pins and interfaces.

## Fixed boundaries and exit

Preserve format37 **NO DATABASE MIGRATION**, query scaffold, existing spans,
hints and queue, private-pool/allocator behavior, CPU/ARM safety, driverless
linkage, strict forced CUDA and current typed errors. No backwards-compatibility
shim, CPU substitution, allocator switching or same-process reconstruction is
implied. Public changes need the affected accepted ADR/interface updates.

Keep windchill3 R580 and primary12.6; never K620, host-wide reset, driver/toolkit
symlink changes or Orin reflash. Exact venue/session/cap and publication remain
specific gates. Preparing a distinct release version does not authorize registry
or tag publication. No fork PR/upstream write/force push is permitted here.

Exit requires A140-1–6 for every selected repair or an explicit evidenced
no-repair-needed disposition. U140-V/FT-1/TC-daa1985b applies only when a required
cudarc code release needs a distinct version. TC-509dc8a6, TC-93d11583 and
TC-323f15b0 stay .30. This proposal makes no repair/qualification/closure claim.
