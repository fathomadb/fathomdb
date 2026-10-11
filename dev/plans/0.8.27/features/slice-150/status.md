---
title: FathomDB 0.8.27 Slice 150 — qualification status
status: QUALIFICATION_IN_PROGRESS
target_release: 0.8.27
planning_baseline: 3a69eef82e0381641e5bfbf7dcd8ed25141d97ff
---

# Slice 150 qualification status

Planning began on 2026-10-10 in an isolated worktree from local
`release/0.8.27` at `3a69eef82`. The initial fetch showed the remote branch
behind the local release branch. At that planning baseline, release state
recorded Slice 140 as `PLANNED`. The owner required explicit confirmation of
its completion before execution; that confirmation arrived later the same day.

The [plan](plan.md) reconciles work since the original 2026-09-21 draft. The
[design](design.md) specifies exact-candidate receipts, gates, artifact
provenance, security and retention. The first `gpt-6.1-sol` high design review
returned **APPROVE WITH FIXES**; the assigned Rust SDK hidden probe and exact
retirement documentation targets were added. The `gpt-6-sol` high follow-up
accepted those corrections and requested exact final security sequencing. The
plan now requires a second complete grant/check/revert cycle and input-tree
identity proof after retirement. The subsequent `gpt-6-sol` high confirmation
returned **APPROVE** with no remaining design finding. Slice 132 has
already delivered the dedicated Rust SDK peer of Python and TypeScript;
Slice 150 verifies its parity and packages rather than implementing it again.

## Execution checkpoint — 2026-10-10

The owner confirmed Slice 140 closed. Fetched `release/0.8.27` at
`1f20b1d1badca9d9e142901ab8cb1e4ea117bd91`; release state records Slice
140 complete at `3a69eef82e0381641e5bfbf7dcd8ed25141d97ff` and advances
to Slice 150. The isolated branch merged that closeout and passed
`scripts/preflight.sh --expect-closed 140` with a clean source checkout.

Slice 132's assigned SDK probe has genuine RED `d80f43bcc` and GREEN
`c3239879e`; all fast hidden-surface self-tests pass, Rust SDK parity is
44/44, and its 28 checker tests pass. The independent `gpt-6-sol` high
[code review](code-review.md) approves that bounded change. The exact-source
[public and hidden captures](surface-comparison.md) have been independently
compared; only the intended SDK hidden row and its equivalent test-row identity
change are new. The [retention survey](retention-survey.md) verifies the
relocated 81,701,383-byte Slice 90 checkpoint set and records zero objects
above 100 decimal GB in each required scope. The completed Slice 140
checkout's ignored Cargo target cache was cleared to meet the public
comparator's 100 GB disk preflight.

A subsequent Slice 140 board-only handoff commit advanced `release/0.8.27`.
It was merged into this branch at `04a024ed1`; source, scripts, tools,
workflows and package inputs are byte-identical to the capture commit. The
release preflight and its 11-test regression pass. A full verifier run at
`04a024ed1` passed lint, typecheck, live security and Rust workspace tests,
then encountered the Python test harness's clean-worktree requirement while
this status and evidence were uncommitted. That interrupted run is not a full
gate pass. The records are being committed before an unaffected full rerun.

The unconfined strict security route produced both live AC-037 pass lines and
`0 violation(s), 0 blocker(s), 0 downgrade(s)`. The runbook's procedural
grant/revert witness remains open: on this host `unshare -rUn true` already
succeeds before any grant, so a pre-grant and post-revert denial cannot be
truthfully recorded. The owner has been asked for a restricted shell or a
specific ruling; no alternate witness is being inferred from sandbox denial.

No release PASS, publication, or Slice 150 closeout is claimed here.
