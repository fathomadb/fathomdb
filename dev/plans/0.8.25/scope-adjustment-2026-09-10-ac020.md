---
title: 0.8.25 AC-020 recovery ladder adjustment
status: APPROVED
date: 2026-09-10
authority: repository owner
---

# AC-020 recovery ladder adjustment

## Decision and qualification

The owner requested closure of Slice 75 as-is, two sequential experiment
slices, an evidence-driven implementation slice, and a final verification,
CI, and packaging slice without publication. This allocation implements that
request. Detailed experiment protocols remain subject to independent review
before READY; this document does not approve an unselected product design.

Slice 75 closes as an **evidence checkpoint with obligations transferred**.
It does not close the original release-acceptance requirement, waive AC-020,
or certify unverified receipts. Its implementation and approved oracle
corrections are retained; the rejected MEMSTATUS change stays reverted.
The safe source checkpoint is `5056db9e6de314a70c67ee5195b8dd1f7023e80f`.
The release remains blocked on AC-020 and final verification.

## Allocation

| Slice | Assignment | Dependency | Exit |
| ---: | --- | ---: | --- |
| 75 | Closed checkpoint; carry unfinished acceptance and artifact obligations forward | 73 | Honest inventory, not a green release |
| 76 | Current attribution and statement-reuse experiment | 75 | Mechanism evidence and explicit Slice 77 selection inputs |
| 77 | Adaptive follow-on experiments and candidate selection | 76 | Decision dossier, including negative or inconclusive outcome |
| 80 | Consult, design, then implement the selected correction | 77 | Reviewed product candidate and focused recovery evidence |
| 85 | Final verification, CI and non-publishing package rehearsal | 80 | Reviewed release-readiness evidence, not publication |

Slices 78–79 are reserved for additional experiments; 81–84 for additional
design/implementation work. Reservation is not execution authority. If needed,
propose a bounded slice with its question, cost and exit before occupying a
number; update dependencies explicitly. Do not force a losing candidate into
Slice 80 or expand the experiments indefinitely.

## Fixed boundaries

- AC-020 remains unchanged: concurrent <= sequential * 1.5 / 8.
- Shared-runtime memory-stat disabling and other unapproved host-global
  configuration changes are rejected. No shutdown/reinitialize workaround.
- Statistics-enabled optimizations come first. Isolation, private statistics
  disabling, source forks, API changes and packaging redesign require an
  explicit owner decision after the experiment dossier.
- Direct agents only; no Steward/Orchestrator role or handshake.
- No product implementation is authorized by this planning edit. Experiment
  slices may create isolated, non-shipping prototypes after protocol review.
- Slices 76/77/80 use focused verification only. Slice 85 owns one deduplicated
  final broad round; an exceptional second broad round requires owner
  authorization explaining invalidated coverage. Existing Slice 75 work is
  reused by applicability, never falsely represented as final-code evidence.
- No version cut, publishing, registry mutation, tags, release creation,
  push, or merge to main is authorized here. Package rehearsal is local/staged.
- Maintain Slice 71 AC-072 and both 71B 10k recoveries. Do not rerun historical
  write baselines.

## Research input and corrections

The retained research is
[SQLite contention research](../runs/0.8.25-ac020-research/report.md).
This is a byte-identical evidence snapshot of the supplied main-checkout
report under dev/research/, which is locally Git-excluded; its original stays
untouched. The snapshot is not an endorsement of every research claim.
Its original grounding is main `8b4bc1c6`, not the release checkpoint.
The exact imported report SHA-256 is
`604e1152d2178ae826370853fd20eb66e04376e266e8e7b4c7e30506ba7bb83f`.
Refresh code references before experiments; preserve historical measurements
as evidence, not current baselines or performance predictions.

The [experiment protocol](ac020-experiment-protocol.md) supersedes the
report's experimental recommendations where they conflict: measure actual
allocation churn, queue waiting without requiring a full queue, reserve a
post-candidate profile, preserve the registered oracle, and do not interpret
WAL/VFS contention alone as evidence that private MEMSTATUS disabling helps.

## Current and historical records

[Slice 75 status](features/slice-75/status.md) records closure and carry-forward.
[Slice 76](features/slice-76/plan.md) and
[Slice 77](features/slice-77/plan.md) contain executable experimental designs.
[Slice 80](features/slice-80/plan.md) is a planning contract pending results
and further owner consultation, not an approved implementation design.
[Slice 85](features/slice-85/plan.md) owns final evidence reconciliation.
Earlier Slice 71–75 allocations remain historical; this adjustment supersedes
their references to Slice 75 as the last verification owner.
