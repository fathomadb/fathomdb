---
title: Slice 100 — bounded interrupted-erasure reconciliation
status: PROPOSED
target_release: 0.8.29
slice: 100
---

# Slice 100 — bounded interrupted-erasure reconciliation

This is a documentation and read-only source/evidence proposal at qualified
parent `1099978cd7d7aef6e745cfc850ddb8f9b57050d3`. The same dedicated Sol-high
owner follows the [direct mandate](../../direct-execution.md); parent commissions
fresh independent Astra-medium design review and a different fresh final output
review. No source/test/executable change, new fault or closure is selected.

## Authority and original obligation

[D28-09](../../../0.8.28-draft-scope.md) says “inside a transaction or WAL
checkpoint”. Its original minimum does not require both. It also requires
controlled named interruption, independent reopened-state and physical-erasure
oracles, truthful completion and repeatable recovery; repair is conditional on
an observed violation. Existing transaction-interior process exits address the
fault-position minimum, with the exact proof limits in [the census](coverage-matrix.md).
This proposal does not declare the entire original obligation complete.

Completed [3](../../prework/slice-3/requirements.md),
[4](../../prework/slice-4/alignment-matrix.md),
[5](../../prework/slice-5/traceability.md) and
[9](../../prework/slice-9/decision-log.md) retain bounded100 inclusion and
R29-3P-16/O3-16: compare existing coverage, then fill proven interruption/reopen
post-state gaps under accepted maintenance semantics. They name survivor/history,
projection/span/fence/ticket/telemetry/WAL and eventual completion, but do not
expressly require every dimension in one fixture or every hook, or an active
native-checkpoint interruption in addition to transaction proof. The proposed
interpretation and unresolved residual are explicit in [design](design.md).
A review may challenge this interpretation; it cannot supply an owner waiver.

Verified external memory `physical-erasure-done-defects-only.md` closes further
feature, optimization and research effort except concrete defects. The accepted
[maintenance contract](../../../../design/explicit-maintenance-erasure/contract.md),
[explicit ADR](../../../../adr/ADR-0.8.28-explicit-erasure-maintenance.md),
[autonomous ADR](../../../../adr/ADR-0.8.28-resumable-physical-cleanup.md) and
[format37 ADR](../../../../adr/ADR-0.8.28-transactional-visibility-coalescing.md)
remain in force. This reconciliation changes no API, format, threshold or guarantee.

## Proposed outcome and limits

Retain eight real explicit-policy crash controls, transaction-interior rollback,
positive sanitation continuation, autonomous reopen, fences, sink replay and
independent success/survivor/history/span witnesses. No demonstrated product
defect currently justifies source repair. A before-commit fault after writes
inside IMMEDIATE is transaction-interior interruption, not missing coverage.

G100-1 remains optional unproven active-native-checkpoint reachability. G100-2
contains three retained evidence gaps, not demonstrated bugs. An exhaustive
combined matrix is not an additional gate for the original disjunctive minimum.
The retained post-state gaps are concrete: unchanged survivor historical versions
after interruption (G100-2H), direct post-crash projection/span state (G100-2P),
and independent main/live-WAL absence after post-crash completion before close
(G100-2W). These cannot disappear by labeling a larger conjunction optional.
Whole100/D28-09 closure remains unclaimed. The test plan proposes two existing
transaction faults plus a healthy control for exactly these three predicates.

The prior bounded actual20 evidence/design Astra PASS remains valid for that
reviewed proposal. It accepted eight-control execution inference with quiet-log
limits; it did not waive either proof limit. Current source differs from that
baseline. [Status](status.md) and [design](design.md) record actual integrated
10/20/60/70/30-P and selective unchanged inputs, rather than entire-source equality.
Fresh v1 design review passed this current-source reconciliation with no P1/P2
findings and identified the three concrete retained predicates above. Exact v2
semantic design PASS covers the coverage-only proposal, with documentary fixes
addressed in exact v3 DESIGN PASS. Parent scoped checks and a different fresh
final DOC/OUTPUT PASS cover the imported stable11-file diffc0e80622 against1099978,
as bound in [review log](review-log.md). No verdict upgrades historical runtime
proof or grants a policy exception.

## Ordered work

The review, documentary correction, scoped-check and final-output steps below
completed for the exact v3/imported parent subjects recorded above. The conditional
repair boundary is unchanged; this record update adds no implementation step.

1. Review this seven-file reconciliation against original and later authority,
   complete named test bodies, source mechanisms and retained evidence.
2. Resolve concrete documentary findings through this same owner. Parent runs
   the applicable scoped Markdown/state checks on the exact reviewed patch;
   no borrowed tools or product execution are needed for this candidate.
3. Obtain a different fresh final output review. A reviewed no-code documentary
   result is valid. Preserve unproven cells and the separate closure decision.
4. If a concrete accepted-invariant defect is later demonstrated, scope its
   repair separately: observed per-finding RED commit/body evidence followed by
   separate minimal GREEN with unchanged tests and independent review. No new
   test, debugger alternative or passing qualification harness is selected now.

Strict TDD is unchanged. Implementing the proposed likely-passing coverage cells
would need an explicit narrow coverage-only exception; it is neither granted nor
requested from the user here. No Slice20 exception is inferred. Any later fault/source execution requires explicit
scope, actual-parent reread, own closed10 preflight/tools/resources and review.
Slice160 owns affected final combined qualification, without an automatic repeat
of all historical tests. No dependency on30 runtime,80/90 or CUDA is introduced;
30 G/D is isolated, unqualified and outside this parent's source/evidence.

## Quality-roadmap fit

The [roadmap](../../../../notes/fathomdb-quality-testing-roadmap.md) applies
bounded anomaly/reopen testing, positive witnesses and independent survivor-only
comparison already present. New evidence is substantial existing real-crash
coverage and unchanged erasure mechanisms despite later query/MCP work. Defer
new VFS/OOM/power-loss/fuzz/MC/DC or optimization campaigns under the owner closure;
roadmap recommendations create no automatic gates. Local architectural attribution
is sufficient: no contradiction requiring a global architecture edit was found.
