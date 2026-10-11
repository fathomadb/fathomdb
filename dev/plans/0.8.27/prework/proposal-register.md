---
title: FathomDB 0.8.27 prework proposal register
status: COMPLETE
target_release: 0.8.27
---

# FathomDB 0.8.27 prework proposal register

Scores use understanding `high/medium/low`, stability risk
`critical/high/medium/low`, and effort `XS/S/M/L/XL`. `Include` means placement
in the executable 0.8.27 plan, not authorization to tag or publish.

| ID | Proposal | Understanding | Risk | Effort | Disposition and placement |
| --- | --- | --- | --- | --- | --- |
| P27-01 | Reproduce and correct F27-01 through the existing public erasure path. | high | critical | M | **Include, Slice 20.** Implementation-neutral until native RED localizes the cause. |
| P27-02 | Full snapshot leases (former D27-01). | medium | high | L | **Postpone to 0.8.28 D28-05.** No proof that compact frozen reads are insufficient. |
| P27-03 | Fully bound cursors (former D27-02). | medium | high | L | **Postpone to 0.8.28 D28-06.** No demonstrated duplicate/omission/authorization failure. |
| P27-04 | General graph/state continuation (former D27-03). | medium | high | L | **Postpone; merge provenance into 0.8.28 D28-03.** No bounded consumer workload requiring it. |
| P27-05 | Persisted evidence replay (former D27-04). | medium | high | L | **Postpone to 0.8.28 D28-07.** No demonstrated need beyond compact reference lifetime. |
| P27-06 | Move the reviewed readability work from unscheduled 0.9.0 into the quiet 0.8.27 line after the correction. | high | high | XL | **Include, Slices 30-150.** Five targets only; preserve topology/surfaces; reuse shipped modules; stop at qualified candidate. |
| P27-07 | Re-run/merge the large-file experiment branches. | high | high | L | **Reject.** Carry conclusions, never merge the experimental histories wholesale. |
| P27-08 | Real-surface comparator and exact feature/move manifests before structural moves. | high | high | M | **Include, Slice 30.** Extend existing parity/release introspection rather than copied lists. |
| P27-09 | `smol-toml` advisory correction. | high | medium | S | **Include, Slice 10** if >=1.7.1 resolves cleanly; markdown neutrality and tooling tests required. |
| P27-10 | Broad dependency/toolchain refresh. | high | high | L | **Reject for 0.8.27.** Retain protected native stacks and postpone unrelated patch/major updates. |
| P27-11 | Correct two upload-artifact version comments without changing SHAs. | high | low | XS | **Include, Slice 10** as mechanical truth. |
| P27-12 | Reconcile public 0.8.26 truth and strengthen the focused truth checker. | high | medium | S | **Include, Slice 10.** Current check is already RED on README. |
| P27-13 | Consolidate recurring benign performance-digest secret-scan authority. | medium | high | S | **Include, Slice 10** with security review, exact paths/shapes, positive fixtures, and credential-shaped negatives. |
| P27-14 | Delete stale generated refactor maps and machine outputs. | medium | low | S | **Postpone.** First prove inbound readers and unique evidence; no cleanup needed for feature correctness. |
| P27-15 | Treat line count as a blocking gate or split the 59 settled LEAVEs. | high | high | XL | **Reject.** Size remains advisory; existing decisions stand. |
| P27-16 | Reuse current design-owner/lifecycle authority and converge citations by symbol/ADR/test or commit-bound location. | high | medium | M | **Include, Slice 140.** No parallel ownership system. |
| P27-17 | Route-specific disk/tool readiness and owned scratch/build roots. | high | high | S | **Include as execution conditions in Slice 30 and each heavy slice.** Do not globally serialize tests. |
| P27-18 | Pre-tag exact-environment-permission readiness check. | high | high | S | **Include, Slice 150/closeout.** Read-only check; no automatic policy widening. |
| P27-19 | Required-check aggregation, merge queue, nightly/soak expansion, or broad CI redesign. | high | medium | L | **Reject.** Preserve lightweight, diff-scoped CI. |
| P27-20 | Publish 0.8.27. | high | critical | M | **Not authorized.** Separate explicit user decision after qualification. |

## Release decisions

The reviewed recommendation is:

1. Include the F27-01 correction first.
2. Move all unproven D27 continuity candidates to the 0.8.28 draft.
3. Use the resulting quiet release line for the bounded five-file semantic
   refactor, with a comparator/manifest gate before any move.
4. Admit only the focused Slice 10 preparation bundle above.
5. Preserve all accepted topology, public surface, schema 34, and publication
   boundaries.

The direct 2026-09-21 user commission covers prework Slices 0-9 only. It does
not by itself authorize Slice 10+, pushes, tags, registries, or publication.
