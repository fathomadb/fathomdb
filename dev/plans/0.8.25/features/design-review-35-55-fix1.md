---
title: 0.8.25 Slices 35–55 design review FIX-1 resolution
status: COMPLETE
review_cycle: FIX-1
reviewed_on: 2026-09-01
design_status: DRAFT_REVIEW
---

# Slices 35–55 FIX-1 resolution

FIX-1 changes only the five successor designs. Historical designs remain
unchanged, and READY remains blocked on Slice 7 plus independent re-review.

| Finding | Changed design section | Resolution | Status |
| --- | --- | --- | --- |
| DR35-01 | Slice 35 Public/wire; Slice 50 Public/wire | V1 now requires exact canonical originating `ReadContext`; narrower/subset semantics are explicitly unsupported and tested. | RESOLVED |
| DR35-02 | Slice 35 Lifetime/persistence | UTC seconds, default 900s, allowed 60–3600s, no renewal, expiry precedence, bounded pruning, and pre/post-prune outcomes are fixed; later leases cannot outlive it. | RESOLVED |
| DR35-03 | Slice 35 Projection binding/flow; Slice 40 Migration | `ProjectionBindingV1` uses declaration/boundary/cursor/terminal digests; Slice 40 maps only with unique equivalence proof and never rebinds. | RESOLVED |
| DR40-01 | Slice 40 Applicability/advancement; Migration/activation | Readiness and serving role are separate; activation requires all applicable work ready through fixed boundary; ambiguous legacy serving is degraded and cannot satisfy readiness. | RESOLVED |
| DR40-02 | Slice 40 Applicability/advancement | Immutable applicability manifest plus explicit `skipped` rows defines every mutation/work kind and exact `ready_through` advancement. | RESOLVED |
| DR40-03 | Slice 40 Lifecycle/erasure | Erasure now scrubs serving, legacy-serving, building, retired, and failed generation rows/work and invalidates leases; raw-state canaries required. | RESOLVED |
| DR45-01 | Slice 45 Public/wire; Cursor/authority flow | Every continuation requires exact current context; current fences are rechecked; newly hidden state aborts whole page with one non-disclosing outcome. | RESOLVED |
| DR45-02 | Slice 45 Public/wire | Graph paging is scoped to one-hop direct adjacency with seed/direction, deduped neighbor item, and stable ID order; Slice 60 exclusively owns combined/path expansion. | RESOLVED |
| DR45-03 | Slices 40/45/55 Public/wire | Every public object has `schema_version: 1`; all inherit Slice 15 unknown/u64/error/SDK/Windows/registry rules and canonical fixtures. | RESOLVED |
| DR50-01 | Slice 50 Public/wire | Defined separate `EvidenceSearchResultV1` sidecar with strict equal-length index+identity association and whole-request failure; default result/hit unchanged. | RESOLVED |
| DR50-02 | Slice 50 Receipt/reference persistence | Added atomic Engine-owned receipt/hit schema, caps, privacy fields, lease-equal retention, restart behavior, and erasure cleanup before handles escape. | RESOLVED |
| DR50-03 | Slice 50 Resolution precedence | Authentication/context/current authority collapses to identical `EvidenceUnavailable`; state-specific outcomes occur only after authorization, with bounded fixed metadata probes. | RESOLVED |
| DR50-04 | Slice 50 Resolution precedence | Dependencies/contributions are complete under explicit 256/64 caps or resolution refuses; success carries explicit complete flags and no partial continuation. | RESOLVED |
| DR55-01 | Slice 55 Trace/continuation | Added persisted frontier/visited trace lease, total node/edge/work caps, exact-context cursor binding, crash semantics, and explicit complete/truncated states. | RESOLVED |
| DR55-02 | Slice 55 Integrity/repair | Added bounded durable integrity jobs/pages/status and a closed three-action repair enum limited to Engine-owned state; large rebuilds enqueue governed work. | RESOLVED |

FIX-1 resolves all nine P1 and six P2 cycle-0 findings. This is an author
resolution record, not an independent PASS verdict.
