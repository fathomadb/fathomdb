---
title: 0.8.26 Slice 2 — repository cruft review
status: COMPLETE
target_release: 0.8.26
depends_on: 1
---

# Slice 2 plan

## Slice-complete workflow

This plan adopts the [lean slice execution contract](../../slice-execution-contract.md).
It is evidence-only: reconcile draft deltas, complete and independently review
the disposition design, verify the inventory, write status, and mark behavioral
TDD/code review not applicable unless scope changes.

## Outcome

Produce a repo-wide, evidence-backed cruft proposal without moving, renaming,
archiving, deprecating, or deleting anything.

## Review domains

- program/release plans, boards, ledgers, handoffs, and experiments;
- `dev/` needs, requirements, acceptance, architecture, interfaces, design,
  test plans, traceability, and historical `dev/progress`;
- source, tests, fixtures, scripts, migrations, generated files, feature flags,
  and compatibility paths;
- developer notes and intermediate findings; and
- public docs, examples, release notes, API references, and package metadata.

For every finding record an exact path or reproducible tracked-file pattern,
inbound-reference class, authority, evidence, risk, and one action: keep,
deprecate-in-place, archive-in-place, or candidate-delete. Per-file reachability
proof for a large candidate group is deferred to the implementation slice and
must pass before any deletion.
Historical scientific/release evidence is presumed valuable until proven
otherwise.

## Acceptance

- Every repository domain has a reviewed inventory and explicit no-finding row
  where appropriate.
- Candidate deletion names the proof still required; actual deletion later
  proves no authority, consumer, evidence, or inbound link.
- Active-versus-historical navigation is distinguished from deletion.
- No repository content under review is changed.

The delta review rejected per-file proof for hundreds of logs during an
evidence-only proposal slice as overbuilt and unsafe before Slices 6–7 were
complete. Grouped reproducible patterns are the review unit; deletion remains
individually fail-closed.
