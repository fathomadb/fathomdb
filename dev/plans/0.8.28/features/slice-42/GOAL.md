---
title: FathomDB 0.8.28 Slice 42 — goal
status: PLANNED (docs written 2026-10-16; plan review R1 APPROVE-WITH-FIXES applied; Q-A1 pending)
target_release: 0.8.28
observed_on: 2026-10-16
---

# Slice 42 goal

Two units, placed by owner ruling on 2026-10-16:

- **Unit A (todos ledger `dev/todos-and-considerations-ledger.jsonl`
  seq 312, `TC-f9fa3b87-6f23-427b-b027-6889e52b3bef`).** Make the
  descriptor check in
  `slice135_persistent_sqlite_full_preserves_state_until_cap_lifts`
  independent of the other tests in its binary. Today it counts every
  open descriptor in the process, so it fails in 9 of 10 parallel runs.
- **Unit B (todos ledger seq 313,
  `TC-4786ddae-2a6a-4da9-b618-3f659cfacb6e`).** Investigate how published Rust artifacts
  can get one SQLite page cache per connection, and recommend a route to
  the owner. Investigation only: nothing is implemented in this slice
  unless the owner rules for it later.

The slice is complete when every AC42 in [requirements.md](requirements.md)
passes, the investigation is delivered and reviewed, and the codex review
approves the Unit A change. Steps are in [plan.md](plan.md); state is in
[status.md](status.md).

Paste the block below to continue the slice in a fresh session.

```text
Continue FathomDB 0.8.28 Slice 42 (test hygiene and the published-Rust page
cache) in worktree /home/coreyt/projects/fathomdb-worktrees/slice-0.8.28-42
on branch slice/0.8.28-42-test-hygiene-and-published-cache. Read AGENTS.md,
the memory index, and
dev/plans/0.8.28/features/slice-42/{requirements,design,test-plan,plan,status}.md.
Resume from status.md. Edit only the test lines the owner ruled on (Q-A1).
Unit B changes no tracked source, Cargo file or workflow.
```
