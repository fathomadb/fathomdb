---
title: 0.8.25 Slice 71 — status
status: IN_PROGRESS
slice: 71
updated: 2026-09-08
---

# Slice 71 status

The owner approved [71B — General write regression](write-regression-subplan.md)
on 2026-09-08 as immediate work within Slice 71. The prior pause is superseded
for write attribution, correction, and focused verification under that plan.
71B is not another release-ladder slice. Planning integration and the new
measurement protocol are complete. The [planning integration review](71b-planning-review.md)
passed, and independent protocol/code review passed the immutable execution
contract at `76e42b90`.

Completed and retained work includes the reviewed plan/design, exact-runner
and redundant search-barrier TDD correction through `5546585d`, strict
evidence validation, and the admissible AC-013 campaign through `2863f7f8`.
Independent code review passed that exact commit; the separate verification
pass was interrupted. These are not review or completion claims for 71B.
The admissible result is
`environment_invalid`: all baseline and candidate p50 repetitions miss AC-072,
and candidate p99 within-arm spread is 40%. The registered stop condition
blocked the original bulk-ingest rerun. This read-latency disposition remains
unresolved but no longer blocks the authorized write work.

The sole sealed 71B campaign is retained with `spread_invalid` after 16 of 18
cells. All completed environments were valid, but AC-013 `generation_only`
acknowledgement spread was 118.430% while total spread was 5.776%. Two
independent read-only evidence audits passed the retained result and confirmed
that no causal attribution or product correction is admissible. No retry,
AC-072 run, or broad verification occurred. See the
[retained result](71b-attribution-result.md).

Immediate next action: obtain owner disposition of the
[prospective protocol-amendment draft](71b-prospective-amendment-draft.md).
No replacement measurement is authorized before approval and independent
prospective review. The [remaining-work outline](remaining-work-outline.md)
indexes both parent obligations. Slice 71B and Slice 71 remain incomplete;
Slice 72 stays dependency-blocked until the complete parent slice receives its
required disposition.
