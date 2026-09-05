---
title: 0.8.25 Slice 40 independent design review — cycle 2
status: CHANGES_REQUIRED
review_cycle: 2
reviewed_on: 2026-09-05
reviewed_commit: 2be5f809
---

# Slice 40 independent design review — cycle 2

The code-grounded review accepted the global in-place epoch boundary but found
that v4 still relied on a per-cursor assignment/work-mask model that could not
remain total across the existing scheduler, lifecycle, erasure, and repair
paths.

| ID | Severity | Finding | FIX-2 disposition |
|---|---|---|---|
| DR40-23 | P1 | `up_to_date` can mean no dense work or successful publication; it is not sufficient completion proof. | Require terminal plus sidecar plus vec0 for every dense-applicable live owner. |
| DR40-24 | P1 | Node, edge, no-runtime, late-enrolment, lifecycle, and unsupported-kind applicability differed from the proposed mask. | Define exact node/edge predicates and centralize applicability/completion across scheduler and status paths. |
| DR40-25 | P1 | Erasure and rebuild delete terminal/assignment rows, permanently stranding an integer-contiguous watermark. | Derive readiness over current applicable owners, not integer cursor continuity. |
| DR40-26 | P1 | The observed high-water omitted operational/reserved/closure cursor consumers. | Define it as the persisted `load_next_cursor` maximum and state that non-owner cursors are contextual, not work. |
| DR40-27 | P1 | A global transition left assignment ownership and O(N) rewrites ambiguous. | Remove assignment rows; do not rewrite receipts; correlate through indexed receipt identity. |
| DR40-28 | P1 | Receipt replay did not durably preserve generation correlation. | Add one nullable receipt column and require persist/hydrate/replay/redaction validation. |
| DR40-29 | P1 | Boot repair, late enrolment, lifecycle, supersession, and repair were unclassified. | Add an exhaustive path table and closed-source audit obligation. |
| DR40-30 | P2 | Slice-35 digest extension lacked canonical bytes and trigger names. | Define v2 domain/order/codec/goldens and exact `pg`/`pc` triggers. |
| DR40-31 | P2 | SDK placement, integer encoding, required ID, and error/state boundaries conflicted. | Place pure calls under `read`, require operation/cursor/generation, use canonical wire decimals, and make unavailable an error only. |
| DR40-32 | P2 | Embedder identity fields and exclusion boundary were not exact. | Hash persistent profile/name/revision/dimension and exclude runtime/device/mean/equivalence state. |
| DR40-33 | P2 | The 1,024-generation cap created permanent operator failure. | Remove the cap; retain content-free transition history without reuse. |
| DR40-34 | P2 | Performance could exclude newly introduced O(N) work. | Eliminate assignment rewrites and preregister metadata, write, storage, status, reopen, CPU/CUDA, and transition observations. |
| DR40-35 | P2 | Windows/package/CUDA routes were not executable. | Name mandatory target/package feature sets, artifact smokes, CUDA witness, and native Windows route. |

FIX-2 deliberately simplifies the model rather than adding a second scheduler:
generation history is per explicit transition, mutation correlation is a
receipt field, and current completeness is a pure snapshot query over live
owners and physical publication proof.
