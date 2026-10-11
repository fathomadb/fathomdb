---
title: FathomDB 0.8.27 Slice 115 design reviews
status: APPROVED
---

# Slice 115 design reviews

## First review — `gpt-6.1-sol` high

Verdict: adjust before execution. The reviewer found three material gaps:

1. AC27-115C required attribution and two deeper profiles, but the initial
   design could have closed with total wall time or a blocked `perf` attempt.
   The revision requires a nontrivial stage for every path, two usable deeper
   profiles, and names unconfined GDB sampling as the fallback. Missing
   mandatory profiles leaves the slice open.
2. The draft simultaneously required and conditionally scheduled a current
   D27 campaign. The revision treats Slice 90's reviewed v2 D27 receipt as
   historical qualification and runs a current campaign only for a concrete
   same-protocol drift question. Its report-only swap rule stays unchanged.
3. A large CPU real-model run would conflict with the standing GPU eval
   policy. The revision bounds the real CPU cell to a small compatibility and
   repeatability probe; substantial model work uses a 3090 and is labeled
   separately.

The reviewer confirmed the vector-stage seam and nonempty graph-evidence
fixture are feasible and the unused `_material` construction exists in the
current engine. The first review made no file edits.

## Second review — `gpt-6-sol` high

Verdict: approve for execution after one correction. The reviewer confirmed
the first three findings closed, then found that repeated mutating samples
could change database state or measure no-op erasure. The design now requires
an equivalently seeded fresh database for each write, projection and erasure
sample; setup is outside the timed region, and pre-state digest/counts plus
post-state semantics are checked. The reviewer reread the revision and found
no remaining material design issue. Profiler availability remains a
fail-closed execution condition, not a presumed pass.
