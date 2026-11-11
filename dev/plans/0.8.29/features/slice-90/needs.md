---
title: "Slice 90: needs"
status: PROPOSED
target_release: 0.8.29
slice: 90
---

# Slice 90 needs

This is a bounded early proposal against clean source
`2f6913c1d59ac8ff3a30de835b57290a8635998c`, not approval to build.
The [release needs](../../../plan-0.8.29.md#draft-release-needs-requirements-and-acceptance-allocation)
and [prework allocation](../../prework/slice-3/needs.md) retain canonical IDs.
Local S90 IDs create no new canonical acceptance gate.

| Need | Observable user outcome | Trace |
| --- | --- | --- |
| S90-N1 | An agent distinguishes omitted unsupported freshness/confidence features from real readiness or execution failures. | N29-A; NEED-010/020/023; B2-1; R1/A1 |
| S90-N2 | An agent can correct an unknown top-level argument or foreign feedback reference without guessing accepted keys or reading record bodies. | N29-A; NEED-010/023; B2-4/5; R2/R3/A2/A3 |
| S90-N3 | An agent knows the session default even with hints disabled and can omit store intentionally. | N29-A; B2-6; R4/A4 |
| S90-N4 | A caller can inspect known stored head representation while retaining unknowns, exclusions and explicit stale/cold availability. | N29-A/K; R29-3P-15; B2-8; R5–R8/A5–A8 |
| S90-N5 | A caller can interpret the represented bytes of the exact returned vector participant in every supported language and output format. | N29-K; NEED-010/016/023; REQ-053/056/064; R9/R10/A9/A10 |
| S90-N6 | Existing hints, bounded output, ranking, queues, storage and frozen semantics keep working when residual fields arrive. | N29-A/S/K/D; R29-3P-14/15; R11/R12/A11/A12 |

[Requirements](requirements.md), [acceptance](acceptance.md), [design](design.md),
[source census](census.md) and [test mapping](test-map.md) form the review packet.
B2-2/3 hints, partial B2-6 default signals, B2-7 admission and B2-8 span storage
are built controls; no duplicate implementation is allocated. Existing calibrated
verdict/weak-result restrictions, query clone-equality scaffold, schema37/no
migration and no backcompat shims remain. SDK hints placement/default belongs
to conditional150. Actual approved80 and the separate Block1 follow-up exit
are both required before90 source;160 follows90.
