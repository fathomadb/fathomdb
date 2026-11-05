---
title: FathomDB 0.8.28 Slice 95 — planning status
status: PLANNED
---

# Slice 95 status

**PLANNED; execution NOT_STARTED.** Owner placed Slice 95 after completed
Slice 90 on 2026-11-04 (steward ledger seq-364). Owner placement of Slice 92
(seq-365) now makes its documentation convergence a prerequisite; S95-5 consumes
that completed baseline and checks later repair deltas rather than repeating the audit.
Slice92 completed at `7b24e4c0913ec6585781b1148b81da0853f9dd67`; its
[status](../slice-92/status.md) records assembled verification and candidate-consumer limits.

The [plan](plan.md) now reconciles the 0.8.26 Slice 50/65 and 0.8.27 Slice
150/publication-close lessons with current CI. S95-CI1–CI7 explicitly cover
cold pinned setup, parallel-runner disk/cache peaks, modular package exports,
ANSI Cargo output, host-dependent ShellCheck fixtures, preflight diagnostics
and manual-candidate empty-diff routing.
S95-1–11 also preserve executable routing/tier/run-hygiene controls, five-target
native receipts plus separate Windows WAL proof, shipped/private-hook separation,
permanent live feature coverage, external Rust consumers, separate generated-
evidence scanning, current-owner authority and distinct release rehearsal.

A read-only gh audit on 2026-11-04 confirmed the cited older CI run's fast tier
at 195/198, four failed non-Windows native smokes, queued Windows native smoke,
and separate heavy preflight / race-wrapper / cache post-step outcomes. These
are failure baselines, not qualification of the current candidate. No repair or
new workflow dispatch was performed. A separate read-only evaluation of actual
job predicates exposed the manual empty-diff routing counterexample; that is
static audit evidence, not an observed hosted run.

Physical erasure remains complete except for concrete defects. CI is
informational; no mandatory-check/merge-queue/soak policy or publication authority
is added. The existing 0.8.29 hand-over rule and owner deferrals remain in force.

Independent Astra comparative planning review APPROVE, including the added
manual-dispatch case. Planning checks PASS: `scripts/agent-lint-md.sh`
(structure, links, plan/design/traceability and authority validators), direct
repository-rule lint of all four changed Markdown files,
`scripts/check-release-state-views.sh` (10 blocks across eight state files)
and `git diff --check`. These checks do not establish execution or release
readiness.

Next: consume completed Slice92 documentation, refresh current code/CI/dependencies,
audit the job/evidence matrix and
review the concrete repair design before RED/GREEN implementation. No new CI run,
repair, qualification result or ready-to-publish verdict is claimed by this plan.
