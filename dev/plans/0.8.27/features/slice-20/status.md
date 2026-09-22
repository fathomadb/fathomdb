---
title: FathomDB 0.8.27 Slice 20 - implementation status
status: COMPLETE
completed_on: 2026-09-21
implementation_candidate: 3943cb64dc2d1b99ef9fc4ec2131dca59a71b337
closeout_commit: 5fab7da58cd70335a9efd49d540f88e275bf574a
---

# Slice 20 implementation status

## Completed scope

- Reconciled the draft, current release branch, assigned erasure functions,
  Memex characterization, prework allocations, and Slice 10 changes before
  approving the bounded plan.
- Confirmed the shipped one-source invariant and rejected cross-bucket cursor
  expansion, recursion, new verbs, report fields, schema work, and early
  structural moves.
- Added release-local R27-01 through R27-03 and AC27-01 through AC27-06 with an
  independently reviewed design.
- Committed genuine RED at `3c6fee92`: the new correction matrix produced the
  diagnosed `Storage` failure while its independent rollback control passed.
- Corrected `excise_source_inner` so actuation receipts are validated and
  redacted before their completed correction closures are removed in the same
  immediate transaction.
- Preserved requested-bucket report counts, exact rollback, durable
  post-commit retry, audit idempotence for scrub retry, and binding error
  parity.
- Added exact Rust physical/at-rest evidence, Python and TypeScript smokes, and
  a durable unchanged-fixture Memex consumer oracle.
- Clarified the maintained Rust, Python, TypeScript, and operations contracts
  without changing their public shapes.

## Review, verification, and cleanup

Independent design review passed. Initial code review rejected two evidence
gaps; commit `3943cb64` closed both, and final independent code review passed
with no findings.

Independent verification passed 52 focused Rust tests, 8 Python tests, 8
TypeScript tests, the frozen Memex oracle, all 119 registered repository
suites with no skips or exclusions, security at 0/0/0, full-workspace Clippy
with warnings denied, and full-workspace Cargo check.

No branch or worktree was created for this slice. The user-provided durable
`release/0.8.27` worktree is retained for the release. Temporary Python
environments and generated test artifacts were removed; final candidate status
was clean.

Slice 20 is complete on `release/0.8.27`. Slice 30 and all later slices, tags,
registries, main integration, and publication remain separately gated.
